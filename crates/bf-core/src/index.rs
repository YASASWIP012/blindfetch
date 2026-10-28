//! The public index: where each transaction sits in the packed stream.
//!
//! Every client downloads the whole index, so fetching it reveals nothing.
//! Entries are ordered by (height, position in block) and packed back to
//! back, so only lengths are stored; offsets are running sums.
//!
//! Wire format, all integers LEB128 unless noted:
//!
//! ```text
//! "BFIX"  magic
//! u8      format version (1)
//! u8      network tag
//! u32 LE  first height covered
//! u32 LE  last height covered (inclusive); also the database version
//! n       number of blocks that have entries
//! per block: height delta from the previous listed block (or from the
//!            first height), entry count, then per entry: tx index, length
//! n       number of rows
//! [16]*n  BLAKE2b-128 of each row, personalised "Blindfetch_row_1"
//! ```
//!
//! The row hashes let a client check every row it decodes, including the
//! dummies it never needed. Without them a server could corrupt one answer
//! and watch whether the client noticed, which would show which queries were
//! real.

use std::ops::Range;

use crate::{Network, ROW_BYTES, row_count, varint};

const MAGIC: &[u8; 4] = b"BFIX";
const VERSION: u8 = 2;
const ROW_PERSONAL: &[u8; 16] = b"Blindfetch_row_1";
const INDEX_PERSONAL: &[u8; 16] = b"Blindfetch_idx_1";

pub type RowHash = [u8; 16];

/// The commitment to one row that the index carries.
pub fn row_hash(row: &[u8]) -> RowHash {
    let h = blake2b_simd::Params::new().hash_length(16).personal(ROW_PERSONAL).hash(row);
    h.as_bytes().try_into().unwrap()
}

/// Identifies a mined transaction by where it is, which is what a wallet that
/// scanned compact blocks already knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TxKey {
    pub height: u32,
    pub tx_index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub key: TxKey,
    /// Byte offset of the transaction in the packed stream.
    pub offset: u64,
    pub len: u32,
}

impl Entry {
    /// Rows this transaction touches.
    pub fn rows(&self) -> Range<usize> {
        let start = (self.offset / ROW_BYTES as u64) as usize;
        let end = (self.offset + u64::from(self.len)).div_ceil(ROW_BYTES as u64) as usize;
        start..end
    }

    /// Cuts this transaction out of the rows that hold it. `rows` must be the
    /// rows from [`Entry::rows`], in order.
    pub fn extract(&self, rows: &[Vec<u8>]) -> Option<Vec<u8>> {
        if rows.len() != self.rows().len() || rows.iter().any(|r| r.len() < ROW_BYTES) {
            return None;
        }
        let skip = (self.offset % ROW_BYTES as u64) as usize;
        let mut out: Vec<u8> = rows.iter().flat_map(|r| r[..ROW_BYTES].iter().copied()).collect();
        out.drain(..skip);
        out.truncate(self.len as usize);
        (out.len() == self.len as usize).then_some(out)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IndexError {
    #[error("not a Blindfetch index")]
    BadMagic,
    #[error("unsupported index version {0}")]
    BadVersion(u8),
    #[error("unknown network tag {0}")]
    BadNetwork(u8),
    #[error("index is truncated or malformed")]
    Malformed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Index {
    pub network: Network,
    pub first_height: u32,
    pub last_height: u32,
    entries: Vec<Entry>,
    row_hashes: Vec<RowHash>,
}

impl Index {
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn get(&self, key: TxKey) -> Option<Entry> {
        self.entries
            .binary_search_by_key(&key, |e| e.key)
            .ok()
            .map(|i| self.entries[i])
    }

    /// Bytes in the packed stream.
    pub fn stream_len(&self) -> u64 {
        self.entries.last().map_or(0, |e| e.offset + u64::from(e.len))
    }

    /// Rows in the PIR database built from this index.
    pub fn num_rows(&self) -> usize {
        row_count(self.stream_len())
    }

    /// Whether `row` is what the index committed to for row number `i`.
    pub fn check_row(&self, i: usize, row: &[u8]) -> bool {
        self.row_hashes.get(i).is_some_and(|h| *h == row_hash(row))
    }

    /// A short fingerprint of the whole index. Two servers built from the
    /// same chain produce the same bytes, so clients can compare fingerprints
    /// across operators.
    pub fn digest(&self) -> [u8; 32] {
        let h = blake2b_simd::Params::new().hash_length(32).personal(INDEX_PERSONAL).hash(&self.encode());
        h.as_bytes().try_into().unwrap()
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(16 + self.entries.len() * 4);
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.push(self.network.tag());
        out.extend_from_slice(&self.first_height.to_le_bytes());
        out.extend_from_slice(&self.last_height.to_le_bytes());

        let blocks: Vec<&[Entry]> = self.entries.chunk_by(|a, b| a.key.height == b.key.height).collect();
        varint::write(&mut out, blocks.len() as u64);
        let mut prev = self.first_height;
        for block in blocks {
            let height = block[0].key.height;
            varint::write(&mut out, u64::from(height - prev));
            prev = height;
            varint::write(&mut out, block.len() as u64);
            for e in block {
                varint::write(&mut out, u64::from(e.key.tx_index));
                varint::write(&mut out, u64::from(e.len));
            }
        }
        varint::write(&mut out, self.row_hashes.len() as u64);
        for h in &self.row_hashes {
            out.extend_from_slice(h);
        }
        out
    }

    pub fn decode(buf: &[u8]) -> Result<Self, IndexError> {
        if buf.len() < 14 || &buf[..4] != MAGIC {
            return Err(IndexError::BadMagic);
        }
        if buf[4] != VERSION {
            return Err(IndexError::BadVersion(buf[4]));
        }
        let network = Network::from_tag(buf[5]).ok_or(IndexError::BadNetwork(buf[5]))?;
        let first_height = u32::from_le_bytes(buf[6..10].try_into().unwrap());
        let last_height = u32::from_le_bytes(buf[10..14].try_into().unwrap());

        let mut pos = 14;
        let next = |pos: &mut usize| varint::read(buf, pos).ok_or(IndexError::Malformed);
        let num_blocks = next(&mut pos)?;
        let mut entries = Vec::new();
        let mut height = u64::from(first_height);
        let mut offset = 0u64;
        for _ in 0..num_blocks {
            height += next(&mut pos)?;
            let count = next(&mut pos)?;
            let mut last_index: Option<u64> = None;
            for _ in 0..count {
                let tx_index = next(&mut pos)?;
                let len = next(&mut pos)?;
                if last_index.is_some_and(|l| tx_index <= l) || len == 0 || len > u64::from(u32::MAX) {
                    return Err(IndexError::Malformed);
                }
                last_index = Some(tx_index);
                entries.push(Entry {
                    key: TxKey {
                        height: u32::try_from(height).map_err(|_| IndexError::Malformed)?,
                        tx_index: u32::try_from(tx_index).map_err(|_| IndexError::Malformed)?,
                    },
                    offset,
                    len: len as u32,
                });
                offset += len;
            }
        }
        if height > u64::from(last_height) {
            return Err(IndexError::Malformed);
        }
        let num_rows = next(&mut pos)? as usize;
        let hashes = buf.get(pos..).ok_or(IndexError::Malformed)?;
        if hashes.len() != num_rows.checked_mul(16).ok_or(IndexError::Malformed)? {
            return Err(IndexError::Malformed);
        }
        let row_hashes = hashes.chunks_exact(16).map(|c| c.try_into().unwrap()).collect();
        let index = Index { network, first_height, last_height, entries, row_hashes };
        if index.num_rows() != num_rows {
            return Err(IndexError::Malformed);
        }
        Ok(index)
    }
}

/// Builds the packed stream and its index together, in chain order.
pub struct Packer {
    network: Network,
    first_height: u32,
    last_height: u32,
    entries: Vec<Entry>,
    stream: Vec<u8>,
}

impl Packer {
    pub fn new(network: Network, first_height: u32) -> Self {
        Packer { network, first_height, last_height: first_height, entries: Vec::new(), stream: Vec::new() }
    }

    /// Appends one transaction. Calls must come in (height, tx index) order.
    pub fn push(&mut self, key: TxKey, tx: &[u8]) {
        assert!(!tx.is_empty(), "empty transaction");
        assert!(key.height >= self.first_height, "height before first height");
        if let Some(last) = self.entries.last() {
            assert!(key > last.key, "transactions out of order");
        }
        self.entries.push(Entry { key, offset: self.stream.len() as u64, len: tx.len() as u32 });
        self.stream.extend_from_slice(tx);
        self.last_height = self.last_height.max(key.height);
    }

    /// Records that every block up to `height` has been scanned, even if the
    /// last ones held nothing to index.
    pub fn covered_through(&mut self, height: u32) {
        self.last_height = self.last_height.max(height);
    }

    /// The index and the database rows, padded to YPIR's minimum row count.
    pub fn finish(self) -> (Index, Vec<u8>) {
        let mut index = Index {
            network: self.network,
            first_height: self.first_height,
            last_height: self.last_height,
            entries: self.entries,
            row_hashes: Vec::new(),
        };
        let mut rows = self.stream;
        rows.resize(index.num_rows() * ROW_BYTES, 0);
        index.row_hashes = rows.chunks_exact(ROW_BYTES).map(row_hash).collect();
        (index, rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MIN_ROWS;

    fn key(height: u32, tx_index: u32) -> TxKey {
        TxKey { height, tx_index }
    }

    fn sample() -> (Index, Vec<u8>, Vec<Vec<u8>>) {
        let txs: Vec<(TxKey, Vec<u8>)> = vec![
            (key(100, 1), vec![1; ROW_BYTES / 2 + 100]),
            (key(100, 4), vec![2; ROW_BYTES]),
            (key(103, 0), vec![3; 10]),
            (key(250, 2), vec![4; ROW_BYTES * 2]),
        ];
        let mut p = Packer::new(Network::Test, 100);
        for (k, tx) in &txs {
            p.push(*k, tx);
        }
        p.covered_through(300);
        let (index, db) = p.finish();
        (index, db, txs.into_iter().map(|(_, t)| t).collect())
    }

    #[test]
    fn encode_decode() {
        let (index, _, _) = sample();
        let bytes = index.encode();
        assert_eq!(Index::decode(&bytes).unwrap(), index);
        assert_eq!(index.last_height, 300);
        // A few bytes per transaction, not a txid's worth, plus 16 per row.
        assert!(bytes.len() < 14 + 4 * 6 + 4 + 3 + 16 * MIN_ROWS);
    }

    #[test]
    fn rows_and_extract() {
        let (index, db, txs) = sample();
        assert_eq!(db.len(), MIN_ROWS * ROW_BYTES);
        for (entry, tx) in index.entries().iter().zip(&txs) {
            let rows: Vec<Vec<u8>> = entry
                .rows()
                .map(|r| db[r * ROW_BYTES..(r + 1) * ROW_BYTES].to_vec())
                .collect();
            assert_eq!(&entry.extract(&rows).unwrap(), tx);
        }
        for (i, row) in db.chunks_exact(ROW_BYTES).enumerate() {
            assert!(index.check_row(i, row));
        }
        assert!(!index.check_row(0, &[0u8; ROW_BYTES]));
        // The row-sized transaction starts mid-row and spans two rows.
        assert_eq!(index.get(key(100, 4)).unwrap().rows(), 0..2);
        assert_eq!(index.get(key(101, 0)), None);
    }

    #[test]
    fn rejects_garbage() {
        let (index, _, _) = sample();
        let mut bytes = index.encode();
        assert_eq!(Index::decode(&bytes[..bytes.len() - 1]), Err(IndexError::Malformed));
        bytes[0] = b'X';
        assert_eq!(Index::decode(&bytes), Err(IndexError::BadMagic));
    }

    #[test]
    #[should_panic(expected = "out of order")]
    fn packer_needs_order() {
        let mut p = Packer::new(Network::Main, 1);
        p.push(key(5, 1), &[1]);
        p.push(key(5, 1), &[1]);
    }
}
