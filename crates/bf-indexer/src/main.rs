//! `bf-indexer sync` walks the chain from Ironwood activation, downloads every
//! transaction with a shielded output, strips its proofs and appends it to a
//! log. `bf-indexer build` packs the log into the index and PIR rows the
//! server loads.

mod lwd;
mod store;

use std::{path::PathBuf, time::Instant};

use anyhow::Context;
use bf_core::{Network, Packer, ROW_BYTES, TxKey};
use clap::{Parser, Subcommand, ValueEnum};
use futures_util::{StreamExt, stream};
use serde::Serialize;
use tracing::info;
use zcash_protocol::consensus::{self, BlockHeight, BranchId, NetworkUpgrade, Parameters};

use store::{Record, Store};

#[derive(Parser)]
#[command(about = "Builds the Blindfetch database from a lightwalletd server")]
struct Cli {
    #[arg(long, value_enum, default_value = "main")]
    network: Net,
    /// Directory holding one subdirectory per network.
    #[arg(long, default_value = "data")]
    data: PathBuf,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Clone, Copy, ValueEnum)]
enum Net {
    Main,
    Test,
}

#[derive(Subcommand)]
enum Cmd {
    /// Download and strip transactions up to the chain tip.
    Sync {
        /// lightwalletd server. Defaults to zec.rocks for the network.
        #[arg(long)]
        server: Option<String>,
        /// Stay this many blocks behind the tip so reorgs never reach the log.
        #[arg(long, default_value_t = 10)]
        confirmations: u32,
        /// Stop at this height instead of the tip.
        #[arg(long)]
        to: Option<u32>,
        #[arg(long, default_value_t = 500)]
        chunk: u32,
        #[arg(long, default_value_t = 24)]
        concurrency: usize,
        /// Keep following the tip after catching up.
        #[arg(long)]
        follow: bool,
    },
    /// Pack the log into index.bin and rows.bin.
    Build,
}

impl Net {
    fn core(self) -> Network {
        match self {
            Net::Main => Network::Main,
            Net::Test => Network::Test,
        }
    }

    fn params(self) -> consensus::Network {
        match self {
            Net::Main => consensus::Network::MainNetwork,
            Net::Test => consensus::Network::TestNetwork,
        }
    }

    fn default_server(self) -> &'static str {
        match self {
            Net::Main => "https://zec.rocks:443",
            Net::Test => "https://testnet.zec.rocks:443",
        }
    }

    fn dir_name(self) -> &'static str {
        match self {
            Net::Main => "main",
            Net::Test => "test",
        }
    }

    /// The database starts where the Ironwood pool does.
    fn first_height(self) -> u32 {
        self.params()
            .activation_height(NetworkUpgrade::Nu6_3)
            .map(u32::from)
            .expect("Ironwood is active on both networks")
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let cli = Cli::parse();
    let dir = cli.data.join(cli.network.dir_name());
    match cli.cmd {
        Cmd::Sync { server, confirmations, to, chunk, concurrency, follow } => {
            let server = server.unwrap_or_else(|| cli.network.default_server().to_string());
            sync(cli.network, &dir, &server, confirmations, to, chunk, concurrency, follow).await
        }
        Cmd::Build => build(cli.network, &dir),
    }
}

#[allow(clippy::too_many_arguments)]
async fn sync(
    net: Net,
    dir: &std::path::Path,
    server: &str,
    confirmations: u32,
    to: Option<u32>,
    chunk: u32,
    concurrency: usize,
    follow: bool,
) -> anyhow::Result<()> {
    let params = net.params();
    let mut store = Store::open(dir, net.first_height())?;
    let mut client = lwd::connect(server).await?;
    info!(server, from = store.next_height(), "syncing");

    loop {
        let tip = lwd::tip(&mut client).await?;
        let target = to.unwrap_or(u32::MAX).min(tip.saturating_sub(confirmations));
        let started = Instant::now();
        let first = store.next_height();
        let (mut txs, mut raw_bytes, mut kept_bytes) = (0usize, 0usize, 0usize);

        while store.next_height() <= target {
            let start = store.next_height();
            let end = (start + chunk - 1).min(target);
            let blocks = lwd::blocks(&mut client, start, end).await?;

            let wanted: Vec<(TxKey, Vec<u8>)> = blocks
                .iter()
                .flat_map(|b| {
                    b.vtx.iter().filter_map(move |tx| {
                        let shielded_out =
                            !tx.outputs.is_empty() || !tx.actions.is_empty() || !tx.ironwood_actions.is_empty();
                        shielded_out.then(|| {
                            (TxKey { height: b.height as u32, tx_index: tx.index as u32 }, tx.txid.clone())
                        })
                    })
                })
                .collect();

            let fetched: Vec<_> = stream::iter(wanted)
                .map(|(key, txid)| {
                    let c = client.clone();
                    async move { (key, txid.clone(), lwd::raw_tx(c, txid).await) }
                })
                .buffered(concurrency)
                .collect()
                .await;

            let mut records = Vec::with_capacity(fetched.len());
            for (key, txid, raw) in fetched {
                let raw = raw?;
                let branch = BranchId::for_height(&params, BlockHeight::from_u32(key.height));
                let s = bf_tx::strip(&raw, branch)
                    .with_context(|| format!("strip {} at {}", lwd::hex_rev(&txid), key.height))?;
                anyhow::ensure!(s.txid.as_ref()[..] == txid[..], "server sent the wrong transaction");
                raw_bytes += s.original_len;
                kept_bytes += s.bytes.len();
                records.push(Record {
                    key,
                    txid: txid.try_into().expect("32-byte txid"),
                    original_len: s.original_len as u32,
                    bytes: s.bytes,
                });
            }
            txs += records.len();
            store.commit(&records, end + 1)?;

            let done = end + 1 - first;
            let rate = f64::from(done) / started.elapsed().as_secs_f64().max(0.001);
            info!(
                height = end,
                target,
                txs,
                mib_mined = raw_bytes >> 20,
                mib_kept = kept_bytes >> 20,
                blocks_per_s = format!("{rate:.0}"),
                "chunk done"
            );
        }

        if !follow {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
    }
}

#[derive(Serialize)]
struct Stats {
    network: Network,
    first_height: u32,
    last_height: u32,
    transactions: usize,
    mined_bytes: u64,
    stripped_bytes: u64,
    rows: usize,
    row_bytes: usize,
    database_bytes: u64,
    index_bytes: usize,
    largest_tx_bytes: u32,
}

fn build(net: Net, dir: &std::path::Path) -> anyhow::Result<()> {
    let store = Store::open(dir, net.first_height())?;
    let mut packer = Packer::new(net.core(), net.first_height());
    let (mut n, mut mined, mut largest) = (0usize, 0u64, 0u32);
    for rec in store.records()? {
        let rec = rec?;
        packer.push(rec.key, &rec.bytes);
        n += 1;
        mined += u64::from(rec.original_len);
        largest = largest.max(rec.bytes.len() as u32);
    }
    packer.covered_through(store.next_height().saturating_sub(1));
    let (index, rows) = packer.finish();
    let index_bytes = index.encode();

    let stats = Stats {
        network: net.core(),
        first_height: index.first_height,
        last_height: index.last_height,
        transactions: n,
        mined_bytes: mined,
        stripped_bytes: index.stream_len(),
        rows: index.num_rows(),
        row_bytes: ROW_BYTES,
        database_bytes: rows.len() as u64,
        index_bytes: index_bytes.len(),
        largest_tx_bytes: largest,
    };

    write_atomic(&dir.join("rows.bin"), &rows)?;
    write_atomic(&dir.join("index.bin"), &index_bytes)?;
    write_atomic(&dir.join("stats.json"), serde_json::to_string_pretty(&stats)?.as_bytes())?;
    println!("{}", serde_json::to_string_pretty(&stats)?);
    Ok(())
}

fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> anyhow::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
