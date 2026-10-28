//! Fetch your own transactions without telling the server which ones.
//!
//! A wallet that has scanned compact blocks knows where its transactions are
//! (height and position in the block) but not their full contents, which it
//! needs for memos. Instead of asking lightwalletd for each txid, it asks a
//! Blindfetch server for the rows that hold them, over PIR, in rounds of a
//! fixed size. Then it checks each result against the txid it expected.
//!
//! What the server sees: one download of the whole index, then requests of
//! exactly `batch` encrypted queries each. Not which rows, not how many of
//! them were real, not which transactions.

use std::time::Instant;

use anyhow::{Context, anyhow, ensure};
use bf_core::{Entry, Index, QueryPlan, TxKey, wire};
use bf_pir::{Client as Pir, Scenario};
use tracing::debug;
use zcash_primitives::transaction::{Transaction, TxId};
use zcash_protocol::consensus::{BlockHeight, BranchId, Parameters};

pub use bf_core::wire::Info;

/// The only error a misbehaving server can cause. Wallets should stop using
/// that server and not retry automatically: retrying would let it probe.
pub const UNTRUSTED: &str =
    "the Blindfetch server returned data that does not match its index or the chain; stop using it";

/// What one round trip cost, for the measurements the demo reports.
#[derive(Clone, Debug, Default)]
pub struct RoundStats {
    pub queries: usize,
    pub real: usize,
    pub bytes_up: usize,
    pub bytes_down: usize,
    pub millis: u128,
}

pub struct Blindfetch {
    base: String,
    http: reqwest::Client,
    info: Info,
    index: Index,
    pir: Pir,
}

/// A transaction the wallet wants, located by where it was mined.
#[derive(Clone, Copy, Debug)]
pub struct Want {
    pub key: TxKey,
    pub txid: TxId,
}

pub enum Fetched {
    /// Checked against its txid.
    Found(Box<Transaction>),
    /// Not in the database: mined before Ironwood activation, after the
    /// database's last height, or carrying no shielded output.
    NotCovered,
    /// Left for the next tick of a fixed schedule.
    Deferred,
}

fn random_u64() -> u64 {
    let mut b = [0u8; 8];
    getrandom::fill(&mut b).expect("system randomness");
    u64::from_le_bytes(b)
}

impl Blindfetch {
    /// Downloads the server's info and its whole index.
    pub async fn connect(base: &str) -> anyhow::Result<Self> {
        // Both rustls backends end up in a wallet's dependency graph; pick one
        // if the host has not.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let base = base.trim_end_matches('/').to_string();
        let http = reqwest::Client::builder().build()?;
        let info: Info = http
            .get(format!("{base}/v1/info"))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .context("read /v1/info")?;
        let index_bytes = http.get(format!("{base}/v1/index")).send().await?.error_for_status()?.bytes().await?;
        let index = Index::decode(&index_bytes)?;
        ensure!(
            index.last_height == info.last_height && index.num_rows() == info.num_rows,
            "server's index does not match its info; try again"
        );
        ensure!(info.row_bytes == bf_core::ROW_BYTES, "unsupported row size {}", info.row_bytes);
        ensure!(
            info.batch >= bf_core::MIN_BATCH,
            "server asks for rounds of {} queries; refusing anything under {}",
            info.batch,
            bf_core::MIN_BATCH
        );
        ensure!(matches!(info.poly_len, 2048 | 4096), "unsupported YPIR ring dimension {}", info.poly_len);
        ensure!(info.num_rows <= bf_core::MAX_ROWS, "database too large: {} rows", info.num_rows);
        let mut scenario = Scenario::new(info.num_rows, info.row_bytes);
        scenario.poly_len = info.poly_len;
        Ok(Blindfetch { base, http, info, index, pir: Pir::new(scenario) })
    }

    /// Checks this server's index against another operator's. The index is a
    /// function of the chain, so two honest servers at the same height publish
    /// the same bytes. A server that rewrote its index to target one
    /// transaction shows up here before any query is sent.
    pub async fn compare_with(&self, other: &str) -> anyhow::Result<()> {
        let other = other.trim_end_matches('/');
        let info: Info = self.http.get(format!("{other}/v1/info")).send().await?.error_for_status()?.json().await?;
        ensure!(
            info.first_height == self.info.first_height && info.last_height == self.info.last_height,
            "the second server is at heights {}..={}, this one at {}..={}; compare when both are at the same height",
            info.first_height,
            info.last_height,
            self.info.first_height,
            self.info.last_height
        );
        let bytes = self.http.get(format!("{other}/v1/index")).send().await?.error_for_status()?.bytes().await?;
        let theirs = Index::decode(&bytes)?;
        ensure!(theirs.digest() == self.index.digest(), "{}", UNTRUSTED);
        Ok(())
    }

    pub fn info(&self) -> &Info {
        &self.info
    }

    pub fn index(&self) -> &Index {
        &self.index
    }

    pub fn locate(&self, key: TxKey) -> Option<Entry> {
        self.index.get(key)
    }

    /// Sends one round of exactly `batch` queries and returns every row, real
    /// or dummy, after checking each against the index.
    ///
    /// Any mismatch is the same error whatever the slot, and the caller must
    /// stop using the server. A server that corrupts one answer to see whether
    /// the client notices learns nothing, because dummies are checked too.
    async fn round(&self, rows: &[usize], real: usize) -> anyhow::Result<(Vec<Vec<u8>>, RoundStats)> {
        let started = Instant::now();
        let pending: Vec<_> = rows.iter().map(|&r| self.pir.query(r)).collect::<anyhow::Result<_>>()?;
        let body = wire::encode_parts(&pending.iter().map(|p| p.payload.as_slice()).collect::<Vec<_>>());
        let bytes_up = body.len();
        let resp = self
            .http
            .post(format!("{}/v1/query", self.base))
            .header("x-db-version", self.info.last_height.to_string())
            .body(body)
            .send()
            .await?;
        ensure!(resp.status() != reqwest::StatusCode::CONFLICT, "the server rebuilt its database; reconnect");
        let resp = resp.error_for_status()?.bytes().await?;
        let parts = wire::decode_parts(&resp).ok_or_else(|| anyhow!(UNTRUSTED))?;
        ensure!(parts.len() == pending.len(), UNTRUSTED);
        let mut out = Vec::with_capacity(rows.len());
        let mut ok = true;
        for ((p, answer), &r) in pending.into_iter().zip(parts).zip(rows) {
            match self.pir.decode(p, answer) {
                Ok(row) if self.index.check_row(r, &row) => out.push(row),
                _ => {
                    ok = false;
                    out.push(Vec::new());
                }
            }
        }
        ensure!(ok, UNTRUSTED);
        let stats = RoundStats {
            queries: rows.len(),
            real,
            bytes_up,
            bytes_down: resp.len(),
            millis: started.elapsed().as_millis(),
        };
        debug!(?stats, "round");
        Ok((out, stats))
    }

    /// One round of dummy queries. A wallet can send these on a timer so that
    /// rounds with real queries do not stand out.
    pub async fn cover_round(&self) -> anyhow::Result<RoundStats> {
        let plan = QueryPlan::new([], self.info.num_rows, self.info.batch, 1, random_u64);
        let (_, stats) = self.round(&plan.batches[0].rows, 0).await?;
        Ok(stats)
    }

    /// Fetches and verifies the wanted transactions, in as many rounds as
    /// they need (at least `MIN_ROUNDS`).
    pub async fn fetch<P: Parameters>(
        &self,
        params: &P,
        wants: &[Want],
    ) -> anyhow::Result<(Vec<(Want, Fetched)>, Vec<RoundStats>)> {
        self.fetch_inner(params, wants, None).await
    }

    /// One tick of a fixed schedule: exactly `rounds` rounds, whatever is
    /// wanted. Transactions that do not fit wait for the next tick and come
    /// back as `Deferred`. A wallet that calls this on a timer sends the same
    /// traffic whether it received nothing or many payments, so even the fact
    /// that it fetched says nothing.
    pub async fn fetch_tick<P: Parameters>(
        &self,
        params: &P,
        wants: &[Want],
        rounds: usize,
    ) -> anyhow::Result<(Vec<(Want, Fetched)>, Vec<RoundStats>)> {
        self.fetch_inner(params, wants, Some(rounds)).await
    }

    async fn fetch_inner<P: Parameters>(
        &self,
        params: &P,
        wants: &[Want],
        fixed_rounds: Option<usize>,
    ) -> anyhow::Result<(Vec<(Want, Fetched)>, Vec<RoundStats>)> {
        let mut located: Vec<(Want, Option<Entry>)> = wants.iter().map(|w| (*w, self.locate(w.key))).collect();
        // On a fixed schedule, take whole transactions in chain order until
        // the tick's capacity is used; the rest wait.
        let mut deferred = Vec::new();
        if let Some(rounds) = fixed_rounds {
            let capacity = rounds * self.info.batch;
            let mut taken = std::collections::BTreeSet::new();
            located.sort_by_key(|(w, _)| w.key);
            let mut keep = Vec::new();
            for (w, e) in located {
                match e {
                    Some(entry) => {
                        let mut with: std::collections::BTreeSet<usize> = taken.clone();
                        with.extend(entry.rows());
                        if with.len() <= capacity {
                            taken = with;
                            keep.push((w, Some(entry)));
                        } else {
                            deferred.push(w);
                        }
                    }
                    None => keep.push((w, None)),
                }
            }
            located = keep;
        }
        let needed: Vec<usize> = located.iter().filter_map(|(_, e)| *e).flat_map(|e| e.rows()).collect();
        let min_rounds = fixed_rounds.unwrap_or(bf_core::MIN_ROUNDS);
        let plan = QueryPlan::new(needed, self.info.num_rows, self.info.batch, min_rounds, random_u64);
        if let Some(rounds) = fixed_rounds {
            ensure!(plan.batches.len() == rounds, "planned {} rounds for a {rounds}-round tick", plan.batches.len());
        }

        let mut rows = std::collections::HashMap::new();
        let mut stats = Vec::new();
        // Send every planned round even after one fails, so the server sees
        // the same traffic either way, and only then report the failure.
        let mut failed: Option<anyhow::Error> = None;
        for batch in &plan.batches {
            match self.round(&batch.rows, batch.real_count()).await {
                Ok((got, s)) => {
                    for ((r, bytes), real) in batch.rows.iter().zip(got).zip(&batch.real) {
                        if *real {
                            rows.insert(*r, bytes);
                        }
                    }
                    stats.push(s);
                }
                // A rebuilt database is not the server misbehaving; stop and
                // let the caller reconnect.
                Err(e) if e.to_string().contains("rebuilt its database") => return Err(e),
                Err(e) => {
                    failed.get_or_insert(e);
                }
            }
        }
        if let Some(e) = failed {
            return Err(e);
        }

        let mut out = Vec::with_capacity(located.len());
        for (want, entry) in located {
            let Some(entry) = entry else {
                out.push((want, Fetched::NotCovered));
                continue;
            };
            let parts: Vec<Vec<u8>> = entry.rows().map(|r| rows[&r].clone()).collect();
            // Rows matched the index, so a bad transaction here means the
            // index itself lies. Same error, same response: stop.
            let bytes = entry.extract(&parts).ok_or_else(|| anyhow!(UNTRUSTED))?;
            let branch = BranchId::for_height(params, BlockHeight::from_u32(want.key.height));
            let tx = bf_tx::verify(&bytes, want.txid, branch).map_err(|_| anyhow!(UNTRUSTED))?;
            out.push((want, Fetched::Found(Box::new(tx))));
        }
        out.extend(deferred.into_iter().map(|w| (w, Fetched::Deferred)));
        Ok((out, stats))
    }
}
