//! `bf-indexer sync` walks the chain from Ironwood activation, downloads every
//! transaction with a shielded output, strips its proofs and appends it to a
//! log. `bf-indexer build` packs the log into the index and PIR rows the
//! server loads.

mod lwd;
mod store;

use std::{path::PathBuf, time::Instant};

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
    /// Start here instead of at Ironwood activation. For sampling only: a
    /// database that starts late does not cover older transactions.
    #[arg(long)]
    from: Option<u32>,
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
        /// While following, rebuild index.bin and rows.bin once this many new
        /// blocks have been indexed. 0 disables rebuilding.
        #[arg(long, default_value_t = 20)]
        rebuild_every: u32,
    },
    /// Pack the log into index.bin and rows.bin.
    Build,
    /// Fetch random logged transactions from a Blindfetch server, check each
    /// against its txid, and report what it cost.
    Bench {
        #[arg(long, default_value = "http://127.0.0.1:8484")]
        url: String,
        /// How many transactions to fetch, in one go.
        #[arg(long, default_value_t = 8)]
        count: usize,
        /// Repeat the fetch this many times.
        #[arg(long, default_value_t = 3)]
        runs: usize,
    },
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
    // Both rustls backends end up in the dependency graph; pick one.
    let _ = rustls::crypto::ring::default_provider().install_default();
    let cli = Cli::parse();
    let dir = cli.data.join(cli.network.dir_name());
    let first = cli.from.unwrap_or_else(|| cli.network.first_height());
    match cli.cmd {
        Cmd::Sync { server, confirmations, to, chunk, concurrency, follow, rebuild_every } => {
            let server = server.unwrap_or_else(|| cli.network.default_server().to_string());
            let opts = SyncOpts { confirmations, to, chunk, concurrency, follow, rebuild_every };
            sync(cli.network, first, &dir, &server, opts).await
        }
        Cmd::Build => build(cli.network, first, &dir),
        Cmd::Bench { url, count, runs } => bench(cli.network, first, &dir, &url, count, runs).await,
    }
}

struct SyncOpts {
    confirmations: u32,
    to: Option<u32>,
    chunk: u32,
    concurrency: usize,
    follow: bool,
    rebuild_every: u32,
}

async fn sync(net: Net, first_height: u32, dir: &std::path::Path, server: &str, opts: SyncOpts) -> anyhow::Result<()> {
    let SyncOpts { confirmations, to, chunk, concurrency, follow, rebuild_every } = opts;
    let params = net.params();
    let mut built_at: Option<u32> = None;
    let mut store = Store::open(dir, first_height)?;
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
            let blocks = match lwd::blocks(&mut client, start, end).await {
                Ok(b) => b,
                Err(e) if follow => {
                    tracing::warn!("block range {start}..={end} failed, retrying: {e:#}");
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                    client = lwd::connect(server).await.unwrap_or(client);
                    continue;
                }
                Err(e) => return Err(e),
            };

            // Each block must extend the one before it, including the last
            // block of the previous chunk. If not, the server switched forks:
            // drop the last hundred blocks and index them again.
            let mut prev = store.last_hash();
            let mut forked = false;
            for b in &blocks {
                if prev.as_ref().is_some_and(|p| *p != b.prev_hash) {
                    forked = true;
                    break;
                }
                prev = Some(b.hash.clone());
            }
            if forked {
                let back = start.saturating_sub(100).max(first_height);
                tracing::warn!(at = start, rewind_to = back, "chain does not link up; rewinding");
                store.rewind(back)?;
                // Re-anchor the link check on the block just below the rewind.
                if back > first_height
                    && let Ok(b) = lwd::blocks(&mut client, back - 1, back - 1).await
                {
                    store.set_last_hash(&b[0].hash)?;
                }
                continue;
            }

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
                let s = match bf_tx::strip(&raw, branch) {
                    Ok(s) => s,
                    // A transaction this build cannot parse (say, a format
                    // from a later upgrade) is left out. Wallets see it as
                    // not covered; the rest of the chain is still indexed.
                    Err(e) => {
                        tracing::warn!(height = key.height, txid = lwd::hex_rev(&txid), "skipping: {e}");
                        continue;
                    }
                };
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
            if let Some(last) = blocks.last() {
                store.set_last_hash(&last.hash)?;
            }

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
        let indexed = store.next_height().saturating_sub(1);
        if rebuild_every > 0 && built_at.is_none_or(|b| indexed >= b + rebuild_every) {
            let dir = dir.to_path_buf();
            tokio::task::spawn_blocking(move || build(net, first_height, &dir)).await??;
            built_at = Some(indexed);
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
    /// Records an older build stored that could not be re-stripped. They are
    /// served as stored, still checked against their txids by wallets.
    unslimmed: usize,
}

fn build(net: Net, first_height: u32, dir: &std::path::Path) -> anyhow::Result<()> {
    let store = Store::open(dir, first_height)?;
    let mut packer = Packer::new(net.core(), first_height);
    let (mut n, mut mined, mut largest, mut unslimmed) = (0usize, 0u64, 0u32, 0usize);
    let params = net.params();
    for rec in store.records()? {
        let rec = rec?;
        // Records from older builds may still carry signatures; drop them
        // now, with the txid checked again. One that fails stays as stored.
        let branch = BranchId::for_height(&params, BlockHeight::from_u32(rec.key.height));
        let bytes = match bf_tx::slim(&rec.bytes, branch) {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(height = rec.key.height, index = rec.key.tx_index, "kept as stored: {e}");
                unslimmed += 1;
                rec.bytes
            }
        };
        largest = largest.max(bytes.len() as u32);
        packer.push(rec.key, &bytes);
        n += 1;
        mined += u64::from(rec.original_len);
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
        unslimmed,
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

async fn bench(net: Net, first_height: u32, dir: &std::path::Path, url: &str, count: usize, runs: usize) -> anyhow::Result<()> {
    let store = Store::open(dir, first_height)?;
    let bf = bf_client::Blindfetch::connect(url).await?;
    let last = bf.info().last_height;
    let all: Vec<(TxKey, [u8; 32])> = store
        .records()?
        .filter_map(|r| r.ok())
        .filter(|r| r.key.height <= last)
        .map(|r| (r.key, r.txid))
        .collect();
    anyhow::ensure!(!all.is_empty(), "no logged transactions inside the server's database");
    let params = net.params();
    let mut seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos() as u64 | 1;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for run in 0..runs {
        let wants: Vec<bf_client::Want> = (0..count)
            .map(|_| {
                let (key, txid) = all[(next() % all.len() as u64) as usize];
                bf_client::Want { key, txid: zcash_primitives::transaction::TxId::from_bytes(txid) }
            })
            .collect();
        let started = Instant::now();
        let (results, rounds) = bf.fetch(&params, &wants).await?;
        let found = results.iter().filter(|(_, f)| matches!(f, bf_client::Fetched::Found(_))).count();
        let up: usize = rounds.iter().map(|r| r.bytes_up).sum();
        let down: usize = rounds.iter().map(|r| r.bytes_down).sum();
        let queries: usize = rounds.iter().map(|r| r.queries).sum();
        let real: usize = rounds.iter().map(|r| r.real).sum();
        println!(
            "run {run}: {found}/{count} verified against their txids | {} round(s), {queries} queries ({real} real) | up {:.2} MiB, down {:.2} MiB | {} ms",
            rounds.len(),
            up as f64 / 1048576.0,
            down as f64 / 1048576.0,
            started.elapsed().as_millis()
        );
    }
    let cover = bf.cover_round().await?;
    println!(
        "cover round: {} queries, up {:.2} MiB, down {:.2} MiB, {} ms",
        cover.queries,
        cover.bytes_up as f64 / 1048576.0,
        cover.bytes_down as f64 / 1048576.0,
        cover.millis
    );
    Ok(())
}
