//! The browser side of Blindfetch: plan a round, build its PIR queries, open
//! the answers and check the transaction against its txid. The page does the
//! HTTP; nothing here touches the network.

use bf_core::{Entry, Index, QueryPlan, TxKey, wire};
use bf_pir::{Client as Pir, Pending, Scenario};
use serde::Serialize;
use wasm_bindgen::prelude::*;
use zcash_primitives::transaction::{Transaction, TxId};
use zcash_protocol::consensus::{self, BlockHeight, BranchId};

fn err(e: impl std::fmt::Display) -> JsError {
    JsError::new(&e.to_string())
}

fn random_u64() -> u64 {
    let mut b = [0u8; 8];
    getrandom03::fill(&mut b).expect("browser randomness");
    u64::from_le_bytes(b)
}

#[wasm_bindgen]
pub struct Session {
    info: wire::Info,
    index: Index,
    pir: Pir,
    pending: Vec<Pending>,
    /// The entry being fetched, and the round's row numbers and real slots.
    target: Option<(Entry, Vec<usize>, Vec<bool>)>,
}

#[derive(Serialize)]
struct EntryJs {
    height: u32,
    tx_index: u32,
    offset: u64,
    len: u32,
    rows: Vec<usize>,
}

impl From<Entry> for EntryJs {
    fn from(e: Entry) -> Self {
        EntryJs {
            height: e.key.height,
            tx_index: e.key.tx_index,
            offset: e.offset,
            len: e.len,
            rows: e.rows().collect(),
        }
    }
}

#[derive(Serialize)]
struct TxSummary {
    txid: String,
    version: String,
    stripped_bytes: usize,
    transparent_inputs: usize,
    transparent_outputs: usize,
    sapling_outputs: usize,
    orchard_actions: usize,
    ironwood_actions: usize,
    /// First bytes of each Ironwood action's encrypted note, so the page can
    /// show the ciphertext a wallet would decrypt.
    ironwood_ciphertexts: Vec<String>,
    matches_expected: Option<bool>,
}

fn params(info: &wire::Info) -> consensus::Network {
    match info.network {
        bf_core::Network::Main => consensus::Network::MainNetwork,
        bf_core::Network::Test => consensus::Network::TestNetwork,
    }
}

#[wasm_bindgen]
impl Session {
    #[wasm_bindgen(constructor)]
    pub fn new(info_json: &str, index_bytes: &[u8]) -> Result<Session, JsError> {
        let info: wire::Info = serde_json::from_str(info_json).map_err(err)?;
        let index = Index::decode(index_bytes).map_err(err)?;
        if index.last_height != info.last_height || index.num_rows() != info.num_rows {
            return Err(err("index does not match server info"));
        }
        if info.batch < bf_core::MIN_BATCH || info.row_bytes != bf_core::ROW_BYTES {
            return Err(err("server parameters would weaken privacy; refusing"));
        }
        let mut scenario = Scenario::new(info.num_rows, info.row_bytes);
        scenario.poly_len = info.poly_len;
        Ok(Session { info, index, pir: Pir::new(scenario), pending: Vec::new(), target: None })
    }

    #[wasm_bindgen(js_name = dbVersion)]
    pub fn db_version(&self) -> u32 {
        self.info.last_height
    }

    #[wasm_bindgen(js_name = txCount)]
    pub fn tx_count(&self) -> usize {
        self.index.entries().len()
    }

    /// The last `n` indexed transactions, newest first, for the page to offer.
    pub fn recent(&self, n: usize) -> Result<JsValue, JsError> {
        let list: Vec<EntryJs> = self.index.entries().iter().rev().take(n).map(|e| (*e).into()).collect();
        Ok(serde_wasm(&list)?)
    }

    pub fn locate(&self, height: u32, tx_index: u32) -> Result<JsValue, JsError> {
        match self.index.get(TxKey { height, tx_index }) {
            Some(e) => serde_wasm(&EntryJs::from(e)),
            None => Ok(JsValue::NULL),
        }
    }

    /// Builds one full round: the target's rows plus random dummies, exactly
    /// `batch` queries. Returns the request body for `POST /v1/query`.
    #[wasm_bindgen(js_name = buildRound)]
    pub fn build_round(&mut self, height: u32, tx_index: u32) -> Result<Vec<u8>, JsError> {
        let entry = self
            .index
            .get(TxKey { height, tx_index })
            .ok_or_else(|| err("that transaction is not in the database"))?;
        let rows: Vec<usize> = entry.rows().collect();
        if rows.len() > self.info.batch {
            return Err(err("transaction spans more rows than one round holds"));
        }
        let plan = QueryPlan::new(rows.iter().copied(), self.info.num_rows, self.info.batch, 1, random_u64);
        let batch = &plan.batches[0];
        self.pending = batch.rows.iter().map(|&r| self.pir.query(r)).collect::<Result<_, _>>().map_err(err)?;
        self.target = Some((entry, batch.rows.clone(), batch.real.clone()));
        Ok(wire::encode_parts(&self.pending.iter().map(|p| p.payload.as_slice()).collect::<Vec<_>>()))
    }

    /// Opens the server's answer to the last round, cuts the transaction out
    /// of its rows and parses it. If `expected_txid` is given (display order
    /// hex), also says whether it matches.
    #[wasm_bindgen(js_name = openRound)]
    pub fn open_round(&mut self, response: &[u8], expected_txid: Option<String>) -> Result<JsValue, JsError> {
        const UNTRUSTED: &str = "the server returned data that does not match its index; stop using it";
        let (entry, row_ids, real) = self.target.take().ok_or_else(|| err("no round in flight"))?;
        let parts = wire::decode_parts(response).ok_or_else(|| err(UNTRUSTED))?;
        let pending = std::mem::take(&mut self.pending);
        if parts.len() != pending.len() {
            return Err(err(UNTRUSTED));
        }
        // Decode and check every slot, dummies too, so a corrupted dummy fails
        // exactly like a corrupted real row.
        let mut rows: Vec<(usize, Vec<u8>)> = Vec::new();
        let mut ok = true;
        for (((p, a), &r), &is_real) in pending.into_iter().zip(parts).zip(&row_ids).zip(&real) {
            match self.pir.decode(p, a) {
                Ok(row) if self.index.check_row(r, &row) => {
                    if is_real {
                        rows.push((r, row));
                    }
                }
                _ => ok = false,
            }
        }
        if !ok {
            return Err(err(UNTRUSTED));
        }
        rows.sort_by_key(|(r, _)| *r);
        let rows: Vec<Vec<u8>> = rows.into_iter().map(|(_, row)| row).collect();
        let bytes = entry.extract(&rows).ok_or_else(|| err(UNTRUSTED))?;
        let branch = BranchId::for_height(&params(&self.info), BlockHeight::from_u32(entry.key.height));
        let tx = Transaction::read(&bf_tx::expand(&bytes).map_err(err)?[..], branch).map_err(err)?;
        let txid = tx.txid();

        let matches_expected = match expected_txid.filter(|s| !s.trim().is_empty()) {
            Some(hex_txid) => {
                let mut b = hex::decode(hex_txid.trim()).map_err(err)?;
                if b.len() != 32 {
                    return Err(err("a txid is 32 bytes"));
                }
                b.reverse();
                let want = TxId::from_bytes(b.try_into().unwrap());
                Some(bf_tx::verify(&bytes, want, branch).is_ok())
            }
            None => None,
        };

        let iw = tx.ironwood_bundle();
        let summary = TxSummary {
            txid: txid.to_string(),
            version: format!("{:?}", tx.version()),
            stripped_bytes: bytes.len(),
            transparent_inputs: tx.transparent_bundle().map_or(0, |b| b.vin.len()),
            transparent_outputs: tx.transparent_bundle().map_or(0, |b| b.vout.len()),
            sapling_outputs: tx.sapling_bundle().map_or(0, |b| b.shielded_outputs().len()),
            orchard_actions: tx.orchard_bundle().map_or(0, |b| b.actions().len()),
            ironwood_actions: iw.map_or(0, |b| b.actions().len()),
            ironwood_ciphertexts: iw
                .map(|b| {
                    b.actions()
                        .iter()
                        .map(|a| hex::encode(&a.encrypted_note().enc_ciphertext.as_ref()[..24]))
                        .collect()
                })
                .unwrap_or_default(),
            matches_expected,
        };
        serde_wasm(&summary)
    }
}

fn serde_wasm<T: Serialize>(v: &T) -> Result<JsValue, JsError> {
    let s = serde_json::to_string(v).map_err(err)?;
    js_sys_parse(&s)
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = JSON, js_name = parse, catch)]
    fn json_parse(s: &str) -> Result<JsValue, JsValue>;
}

fn js_sys_parse(s: &str) -> Result<JsValue, JsError> {
    json_parse(s).map_err(|_| err("JSON.parse failed"))
}
