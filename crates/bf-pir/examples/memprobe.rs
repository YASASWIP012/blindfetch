//! Builds a server over a zero database of the given shape so peak memory can
//! be measured: `/usr/bin/time -l cargo run --release --features server
//! --example memprobe -- <rows> <row_bytes>`.
fn main() {
    let args: Vec<usize> = std::env::args().skip(1).map(|a| a.parse().unwrap()).collect();
    let (rows, row_bytes) = (args[0], args[1]);
    let scenario = bf_pir::Scenario::new(rows, row_bytes);
    let started = std::time::Instant::now();
    let server = bf_pir::Server::new(vec![0u8; rows * row_bytes], scenario).unwrap();
    println!("{rows} x {row_bytes} B ({} MiB) ready in {:.1}s", rows * row_bytes >> 20, started.elapsed().as_secs_f64());
    let client = bf_pir::Client::new(scenario);
    let q = client.query(rows / 2).unwrap();
    let t = std::time::Instant::now();
    let resp = server.answer(&q.payload).unwrap();
    println!("one query: {:.2}s, {} KiB up, {} KiB down", t.elapsed().as_secs_f64(), q.payload.len() >> 10, resp.len() >> 10);
}
