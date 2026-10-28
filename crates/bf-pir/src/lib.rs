//! YPIR in SimplePIR mode, as ZODL deploys it for voting nullifiers.
//!
//! A database is `num_items` rows of `item_size_bits` bits. A query asks for
//! one row and the server cannot tell which. All cryptography lives in the
//! `valar-ypir` crate, which Zellic audited; this crate only moves bytes.
//!
//! Query wire format, matching ZODL's `vote-nullifier-pir`:
//! `u64 LE len(q) | q as u64 LE words | public params as u64 LE words`.

use anyhow::ensure;
use ypir::{
    client::YPIRClient,
    params::{YPIRSPConfig, params_for_scenario_simplepir_with_config},
};
#[cfg(feature = "server")]
use ypir::params::{DbRowsCols, PtModulusBits};

/// Shape of a PIR database.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scenario {
    pub num_items: usize,
    pub item_size_bits: usize,
    pub poly_len: usize,
}

impl Scenario {
    pub fn new(num_rows: usize, row_bytes: usize) -> Self {
        Scenario { num_items: num_rows, item_size_bits: row_bytes * 8, poly_len: 2048 }
    }

    pub fn row_bytes(&self) -> usize {
        self.item_size_bits / 8
    }

    #[cfg(feature = "server")]
    fn params(&self) -> spiral_rs::params::Params {
        params_for_scenario_simplepir_with_config(
            self.num_items as u64,
            self.item_size_bits as u64,
            YPIRSPConfig::for_poly_len(self.poly_len),
        )
    }
}

fn words_to_bytes(out: &mut Vec<u8>, words: &[u64]) {
    for w in words {
        out.extend_from_slice(&w.to_le_bytes());
    }
}

/// Client side: builds queries and opens responses. Holds no secrets between
/// queries; each query carries its own seed.
pub struct Client {
    scenario: Scenario,
    inner: YPIRClient,
}

/// A query on its way out, and what is needed to read the answer.
pub struct Pending {
    pub payload: Vec<u8>,
    seed: ypir::seed::Seed,
}

impl Client {
    pub fn new(scenario: Scenario) -> Self {
        let inner = YPIRClient::from_db_sz_simplepir_with_config(
            scenario.num_items as u64,
            scenario.item_size_bits as u64,
            YPIRSPConfig::for_poly_len(scenario.poly_len),
        );
        Client { scenario, inner }
    }

    pub fn scenario(&self) -> Scenario {
        self.scenario
    }

    pub fn query(&self, row: usize) -> anyhow::Result<Pending> {
        ensure!(row < self.scenario.num_items, "row {row} out of range");
        let (query, seed) = self.inner.generate_query_simplepir(row);
        let (q, pp) = (query.0.as_slice(), query.1.as_slice());
        let mut payload = Vec::with_capacity(8 + 8 * (q.len() + pp.len()));
        payload.extend_from_slice(&((q.len() * 8) as u64).to_le_bytes());
        words_to_bytes(&mut payload, q);
        words_to_bytes(&mut payload, pp);
        Ok(Pending { payload, seed })
    }

    /// Opens a response. A malformed response is an error, never a panic.
    pub fn decode(&self, pending: Pending, response: &[u8]) -> anyhow::Result<Vec<u8>> {
        let seed = pending.seed;
        let row = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.inner.decode_response_simplepir(seed, response)
        }))
        .map_err(|_| anyhow::anyhow!("could not decode PIR response"))?;
        ensure!(row.len() >= self.scenario.row_bytes(), "PIR response too short");
        Ok(row[..self.scenario.row_bytes()].to_vec())
    }
}

#[cfg(feature = "server")]
pub use server::Server;

#[cfg(feature = "server")]
mod server {
    use super::*;
    use std::io::Cursor;
    use ypir::{
        serialize::{FilePtIter, OfflinePrecomputedValues},
        server::YServer,
    };

    /// Server side: one database, precomputed once, answering any number of
    /// queries.
    ///
    /// YPIR's server borrows its parameters and data for its whole life, so
    /// both are leaked into `'static` here. A server process holds one or two
    /// databases at a time and swaps them every few minutes at most; the
    /// leak is reclaimed by replacing the process, which the deployment does
    /// on each nightly full rebuild.
    pub struct Server {
        scenario: Scenario,
        server: YServer<'static, u16>,
        offline: OfflinePrecomputedValues<'static>,
    }

    // YServer holds raw pointers into leaked, never-mutated memory.
    unsafe impl Send for Server {}
    unsafe impl Sync for Server {}

    impl Server {
        pub fn new(rows: Vec<u8>, scenario: Scenario) -> anyhow::Result<Self> {
            ensure!(
                rows.len() == scenario.num_items * scenario.row_bytes(),
                "database is {} bytes, expected {}",
                rows.len(),
                scenario.num_items * scenario.row_bytes()
            );
            let params: &'static spiral_rs::params::Params = Box::leak(Box::new(scenario.params()));
            let data: &'static [u8] = Box::leak(rows.into_boxed_slice());
            let iter = FilePtIter::new(
                Cursor::new(data),
                scenario.row_bytes(),
                params.db_cols_simplepir(),
                params.pt_modulus_bits(),
            );
            let server = YServer::<u16>::new(params, iter, true, false, true);
            let offline = server.perform_offline_precomputation_simplepir(None, None, None);
            Ok(Server { scenario, server, offline })
        }

        pub fn scenario(&self) -> Scenario {
            self.scenario
        }

        pub fn answer(&self, query: &[u8]) -> anyhow::Result<Vec<u8>> {
            ensure!(query.len() >= 8, "query too short");
            let q_len = u64::from_le_bytes(query[..8].try_into().unwrap()) as usize;
            let rest = &query[8..];
            ensure!(q_len % 8 == 0 && q_len > 0 && q_len < rest.len(), "bad query layout");
            ensure!((rest.len() - q_len) % 8 == 0, "bad public parameter length");
            let words = |b: &[u8]| -> Vec<u64> {
                b.chunks_exact(8).map(|c| u64::from_le_bytes(c.try_into().unwrap())).collect()
            };
            let q = words(&rest[..q_len]);
            let pp = words(&rest[q_len..]);
            let resp = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.server.perform_online_computation_simplepir(&q, &self.offline, &[&pp], None)
            }))
            .map_err(|_| anyhow::anyhow!("query rejected by YPIR"))?;
            Ok(resp)
        }
    }
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn fetches_the_right_row() {
        let scenario = Scenario::new(2048, 4096);
        let mut rows = vec![0u8; scenario.num_items * scenario.row_bytes()];
        for (i, row) in rows.chunks_mut(scenario.row_bytes()).enumerate() {
            row[..8].copy_from_slice(&(i as u64 * 7919).to_le_bytes());
            row[4095] = (i % 251) as u8;
        }
        let expected: Vec<Vec<u8>> = [0usize, 1, 1337, 2047]
            .iter()
            .map(|&i| rows[i * 4096..(i + 1) * 4096].to_vec())
            .collect();
        let server = Server::new(rows, scenario).unwrap();
        let client = Client::new(scenario);
        for (&i, want) in [0usize, 1, 1337, 2047].iter().zip(expected) {
            let p = client.query(i).unwrap();
            let resp = server.answer(&p.payload).unwrap();
            assert_eq!(client.decode(p, &resp).unwrap(), want, "row {i}");
        }
    }
}

#[cfg(test)]
mod sizes {
    use super::*;

    /// `cargo test -p bf-pir --release -- --ignored --nocapture sizes`
    #[test]
    #[ignore]
    fn query_sizes() {
        for (rows, row_bytes) in [(4096usize, 4096usize), (262_144, 4096), (65_536, 16_384), (32_768, 32_768), (16_384, 65_536), (8_192, 131_072)] {
            let c = Client::new(Scenario::new(rows, row_bytes));
            let p = c.query(rows - 1).unwrap();
            println!("{rows} rows x {} KiB ({} MiB db): query {} KiB", row_bytes >> 10, (rows * row_bytes) >> 20, p.payload.len() >> 10);
        }
    }
}
