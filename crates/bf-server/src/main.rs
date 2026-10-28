//! Serves the index and answers PIR queries over it.
//!
//! Routes:
//! - `GET  /v1/info`   database shape and version
//! - `GET  /v1/index`  the full index; every client downloads all of it
//! - `POST /v1/query`  exactly `batch` PIR queries, answered in order
//! - `GET  /v1/seen`   everything this server has learned about recent
//!   requests: their time, size and duration. This is the log a curious
//!   operator would read, published so anyone can see what it contains.
//!
//! The server never logs or stores client addresses. A reverse proxy in front
//! of it may; see docs/THREAT-MODEL.md.

use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant, SystemTime},
};

use anyhow::Context;
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use bf_core::{Index, ROW_BYTES, wire};
use bf_pir::{Scenario, Server};
use clap::Parser;
use serde::Serialize;
use tower_http::{compression::CompressionLayer, cors::CorsLayer};
use tracing::{info, warn};

#[derive(Parser)]
struct Cli {
    /// Directory with index.bin, rows.bin and stats.json from bf-indexer.
    #[arg(long)]
    data: PathBuf,
    #[arg(long, default_value = "127.0.0.1:8484")]
    listen: String,
    #[arg(long, default_value_t = bf_core::DEFAULT_BATCH)]
    batch: usize,
    /// How often to look for a rebuilt database.
    #[arg(long, default_value_t = 120)]
    reload_secs: u64,
    /// On a rebuilt database, exit instead of loading a second copy beside the
    /// first. Run under a service manager that restarts it; the server is
    /// down for the minute or two a load takes.
    #[arg(long)]
    low_memory: bool,
}

struct Db {
    info: wire::Info,
    index: Bytes,
    pir: Server,
}

#[derive(Clone, Serialize)]
struct Seen {
    /// Unix time rounded down to the minute. Finer timing would help anyone
    /// reading this log match requests to wallets.
    at: u64,
    kind: &'static str,
    bytes_in: usize,
    bytes_out: usize,
    queries: usize,
    db_version: u32,
}

struct AppState {
    db: RwLock<Arc<Db>>,
    seen: Mutex<VecDeque<Seen>>,
    batch: usize,
    /// Each query scans the whole database; two rounds at a time keep a small
    /// machine responsive.
    busy: tokio::sync::Semaphore,
}

impl AppState {
    fn current(&self) -> Arc<Db> {
        self.db.read().unwrap().clone()
    }

    fn record(&self, s: Seen) {
        let mut seen = self.seen.lock().unwrap();
        if seen.len() == 500 {
            seen.pop_front();
        }
        seen.push_back(s);
    }
}

fn load(dir: &Path, batch: usize) -> anyhow::Result<Db> {
    let started = Instant::now();
    let index_bytes = std::fs::read(dir.join("index.bin")).context("read index.bin")?;
    let index = Index::decode(&index_bytes)?;
    let rows = std::fs::read(dir.join("rows.bin")).context("read rows.bin")?;
    // A rows.bin that does not match its index (a half-written file, a bad
    // disk) would make every honest wallet lock this server out. Check every
    // row before serving any.
    anyhow::ensure!(rows.len() == index.num_rows() * ROW_BYTES, "rows.bin has the wrong size");
    for (i, row) in rows.chunks_exact(ROW_BYTES).enumerate() {
        anyhow::ensure!(index.check_row(i, row), "rows.bin row {i} does not match index.bin");
    }
    let scenario = Scenario::new(index.num_rows(), ROW_BYTES);
    let info = wire::Info {
        network: index.network,
        first_height: index.first_height,
        last_height: index.last_height,
        num_rows: scenario.num_items,
        row_bytes: ROW_BYTES,
        batch,
        poly_len: scenario.poly_len,
        transactions: index.entries().len(),
    };
    let pir = Server::new(rows, scenario)?;
    info!(
        last_height = info.last_height,
        rows = info.num_rows,
        txs = info.transactions,
        secs = started.elapsed().as_secs(),
        "database ready"
    );
    Ok(Db { info, index: Bytes::from(index_bytes), pir })
}

fn now_minute() -> u64 {
    let secs = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    secs - secs % 60
}

async fn info(State(app): State<Arc<AppState>>) -> Json<wire::Info> {
    Json(app.current().info.clone())
}

async fn index(State(app): State<Arc<AppState>>) -> Response {
    let db = app.current();
    app.record(Seen {
        at: now_minute(),
        kind: "index",
        bytes_in: 0,
        bytes_out: db.index.len(),
        queries: 0,
        db_version: db.info.last_height,
    });
    (
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::HeaderName::from_static("x-db-version"), db.info.last_height.to_string()),
        ],
        db.index.clone(),
    )
        .into_response()
}

async fn query(State(app): State<Arc<AppState>>, headers: HeaderMap, body: Bytes) -> Response {
    let db = app.current();
    let version = headers
        .get("x-db-version")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok());
    if version != Some(db.info.last_height) {
        return (StatusCode::CONFLICT, "database version changed; fetch /v1/info and /v1/index again")
            .into_response();
    }
    let Some(parts) = wire::decode_parts(&body) else {
        return (StatusCode::BAD_REQUEST, "malformed batch").into_response();
    };
    // Every request is the same size in queries, so its shape says nothing
    // about how many transactions the wallet wanted.
    if parts.len() != app.batch {
        return (StatusCode::BAD_REQUEST, format!("send exactly {} queries", app.batch)).into_response();
    }
    let Ok(_permit) = app.busy.acquire().await else {
        return (StatusCode::SERVICE_UNAVAILABLE, "shutting down").into_response();
    };
    let owned: Vec<Vec<u8>> = parts.iter().map(|p| p.to_vec()).collect();
    let db2 = db.clone();
    // Queries in a round are independent; answer them on separate cores.
    let answers = tokio::task::spawn_blocking(move || {
        std::thread::scope(|s| {
            let handles: Vec<_> = owned.iter().map(|q| s.spawn(|| db2.pir.answer(q))).collect();
            handles
                .into_iter()
                .map(|h| h.join().map_err(|_| anyhow::anyhow!("query thread panicked"))?)
                .collect::<anyhow::Result<Vec<_>>>()
        })
    })
    .await;
    let answers = match answers {
        Ok(Ok(a)) => a,
        Ok(Err(e)) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let out = wire::encode_parts(&answers);
    app.record(Seen {
        at: now_minute(),
        kind: "query",
        bytes_in: body.len(),
        bytes_out: out.len(),
        queries: app.batch,
        db_version: db.info.last_height,
    });
    ([(header::CONTENT_TYPE, "application/octet-stream")], out).into_response()
}

async fn seen(State(app): State<Arc<AppState>>) -> Json<Vec<Seen>> {
    Json(app.seen.lock().unwrap().iter().rev().cloned().collect())
}

/// Swaps in a rebuilt database when the indexer writes a new index.bin.
fn spawn_reloader(app: Arc<AppState>, dir: PathBuf, every: Duration, low_memory: bool) {
    std::thread::spawn(move || {
        let mtime = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
        let mut last = mtime(&dir.join("index.bin"));
        loop {
            std::thread::sleep(every);
            let now = mtime(&dir.join("index.bin"));
            if now == last {
                continue;
            }
            // Let the indexer finish writing rows.bin before reading.
            std::thread::sleep(Duration::from_secs(5));
            if low_memory {
                // Freed memory does not reliably go back to the system, so a
                // second copy in the same process ends in swap. Exit and let
                // the service manager start a fresh process on the new files.
                info!("rebuilt database found; exiting so a fresh process loads it");
                std::process::exit(0);
            }
            match load(&dir, app.batch) {
                Ok(db) => {
                    *app.db.write().unwrap() = Arc::new(db);
                    last = now;
                }
                Err(e) => warn!("reload failed: {e:#}"),
            }
        }
    });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let cli = Cli::parse();
    let db = load(&cli.data, cli.batch)?;
    let app = Arc::new(AppState {
        db: RwLock::new(Arc::new(db)),
        seen: Mutex::new(VecDeque::new()),
        batch: cli.batch,
        busy: tokio::sync::Semaphore::new(2),
    });
    spawn_reloader(app.clone(), cli.data.clone(), Duration::from_secs(cli.reload_secs), cli.low_memory);

    // Only the index is compressed. Query responses stay a fixed size, which
    // compression could break.
    let router = Router::new()
        .route("/v1/info", get(info))
        .route("/v1/index", get(index).layer(CompressionLayer::new()))
        .route("/v1/query", post(query))
        .route("/v1/seen", get(seen))
        .route("/healthz", get(|| async { "ok" }))
        // A round is about 3 MiB at the largest database size; allow headroom.
        .layer(DefaultBodyLimit::max(8 << 20))
        .layer(CorsLayer::permissive().expose_headers([header::HeaderName::from_static("x-db-version")]))
        .with_state(app);

    let listener = tokio::net::TcpListener::bind(&cli.listen).await?;
    info!(listen = cli.listen, "serving");
    axum::serve(listener, router).with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}
