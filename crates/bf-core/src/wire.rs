//! What travels between client and server.

use serde::{Deserialize, Serialize};

use crate::Network;

/// `GET /v1/info`. Everything a client needs to build queries, and nothing
/// about any client.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Info {
    pub network: Network,
    pub first_height: u32,
    /// Last height in the database. Also its version: a query built for one
    /// version is refused by another.
    pub last_height: u32,
    pub num_rows: usize,
    pub row_bytes: usize,
    /// Queries per request. The server refuses any other count.
    pub batch: usize,
    /// YPIR ring dimension.
    pub poly_len: usize,
    pub transactions: usize,
}

/// Packs several byte strings as `u32 count` then `u32 len | bytes` each.
pub fn encode_parts<B: AsRef<[u8]>>(parts: &[B]) -> Vec<u8> {
    let total: usize = parts.iter().map(|p| 4 + p.as_ref().len()).sum();
    let mut out = Vec::with_capacity(4 + total);
    out.extend_from_slice(&(parts.len() as u32).to_le_bytes());
    for p in parts {
        out.extend_from_slice(&(p.as_ref().len() as u32).to_le_bytes());
        out.extend_from_slice(p.as_ref());
    }
    out
}

/// Inverse of [`encode_parts`]. Fails on trailing or missing bytes.
pub fn decode_parts(buf: &[u8]) -> Option<Vec<&[u8]>> {
    let read_u32 = |pos: usize| -> Option<usize> {
        Some(u32::from_le_bytes(buf.get(pos..pos + 4)?.try_into().ok()?) as usize)
    };
    let count = read_u32(0)?;
    let mut pos = 4;
    let mut out = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let len = read_u32(pos)?;
        pos += 4;
        out.push(buf.get(pos..pos.checked_add(len)?)?);
        pos += len;
    }
    (pos == buf.len()).then_some(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn parts_round_trip() {
        let parts = vec![vec![1u8, 2, 3], vec![], vec![9; 1000]];
        let enc = super::encode_parts(&parts);
        let dec = super::decode_parts(&enc).unwrap();
        assert_eq!(dec, parts.iter().map(|p| &p[..]).collect::<Vec<_>>());
        assert!(super::decode_parts(&enc[..enc.len() - 1]).is_none());
        let mut extra = enc.clone();
        extra.push(0);
        assert!(super::decode_parts(&extra).is_none());
    }
}
