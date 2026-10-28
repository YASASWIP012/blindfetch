//! The few lightwalletd calls the indexer needs.
//!
//! The indexer downloads every shielded transaction, so asking for them by
//! txid reveals nothing: it is the same set of requests for everyone.

use anyhow::{Context, anyhow};
use futures_util::StreamExt;
use tonic::{
    Code,
    codec::CompressionEncoding,
    transport::{Channel, ClientTlsConfig},
};
use zcash_client_backend::proto::{
    compact_formats::CompactBlock,
    service::{
        BlockId, BlockRange, ChainSpec, TxFilter, compact_tx_streamer_client::CompactTxStreamerClient,
    },
};

pub type Client = CompactTxStreamerClient<Channel>;

pub async fn connect(url: &str) -> anyhow::Result<Client> {
    let uri: tonic::transport::Uri = url.parse().with_context(|| format!("bad server URL {url}"))?;
    let host = uri.host().ok_or_else(|| anyhow!("server URL has no host"))?.to_string();
    // A dropped connection must fail, not hang the sync forever.
    let channel = Channel::from_shared(url.to_string())?
        .connect_timeout(std::time::Duration::from_secs(20))
        .timeout(std::time::Duration::from_secs(120))
        .tcp_keepalive(Some(std::time::Duration::from_secs(30)));
    let channel = if uri.scheme_str() == Some("https") {
        channel.tls_config(
            ClientTlsConfig::new()
                .domain_name(host)
                .assume_http2(true)
                .with_webpki_roots(),
        )?
    } else {
        channel
    };
    Ok(CompactTxStreamerClient::new(channel.connect().await?)
        .accept_compressed(CompressionEncoding::Gzip)
        .max_decoding_message_size(64 << 20))
}

pub async fn tip(client: &mut Client) -> anyhow::Result<u32> {
    let block = client.get_latest_block(ChainSpec::default()).await?.into_inner();
    Ok(u32::try_from(block.height)?)
}

/// Compact blocks for `start..=end`, in order.
pub async fn blocks(client: &mut Client, start: u32, end: u32) -> anyhow::Result<Vec<CompactBlock>> {
    let range = BlockRange {
        start: Some(BlockId { height: start.into(), ..Default::default() }),
        end: Some(BlockId { height: end.into(), ..Default::default() }),
        // Empty means the legacy default: every shielded pool, Ironwood included.
        pool_types: vec![],
    };
    let mut stream = client.get_block_range(range).await?.into_inner();
    let mut out = Vec::with_capacity((end - start + 1) as usize);
    while let Some(block) = stream.next().await {
        out.push(block?);
    }
    anyhow::ensure!(
        out.len() == (end - start + 1) as usize,
        "server returned {} blocks for {start}..={end}",
        out.len()
    );
    Ok(out)
}

/// The raw transaction with this txid (internal byte order), retried a few
/// times on transient errors.
pub async fn raw_tx(mut client: Client, txid: Vec<u8>) -> anyhow::Result<Vec<u8>> {
    let mut wait = std::time::Duration::from_millis(250);
    for attempt in 0.. {
        let req = TxFilter { hash: txid.clone(), ..Default::default() };
        match client.get_transaction(req).await {
            Ok(resp) => return Ok(resp.into_inner().data),
            Err(s) if s.code() == Code::NotFound || attempt == 5 => {
                return Err(anyhow!("get_transaction {}: {s}", hex_rev(&txid)));
            }
            Err(_) => {
                tokio::time::sleep(wait).await;
                wait *= 2;
            }
        }
    }
    unreachable!()
}

/// Txids are displayed byte-reversed.
pub fn hex_rev(txid: &[u8]) -> String {
    let mut b = txid.to_vec();
    b.reverse();
    hex::encode(b)
}
