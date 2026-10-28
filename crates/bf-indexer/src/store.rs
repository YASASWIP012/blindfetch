//! Append-only log of stripped transactions, so a sync can stop and resume.
//!
//! Each record is:
//!
//! ```text
//! u32 LE height | u32 LE tx index | [32] txid | u32 LE mined size | u32 LE len | bytes
//! ```
//!
//! `progress` holds the next height to scan. Records at or above it belong to
//! a chunk that was cut short and are dropped on open.

use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use bf_core::TxKey;

pub struct Record {
    pub key: TxKey,
    pub txid: [u8; 32],
    pub original_len: u32,
    pub bytes: Vec<u8>,
}

pub struct Store {
    dir: PathBuf,
    next_height: u32,
}

impl Store {
    pub fn open(dir: &Path, start_height: u32) -> anyhow::Result<Self> {
        fs::create_dir_all(dir)?;
        let next_height = fs::read_to_string(dir.join("progress"))
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(start_height);
        let store = Store { dir: dir.to_path_buf(), next_height };
        store.truncate_unfinished()?;
        Ok(store)
    }

    pub fn next_height(&self) -> u32 {
        self.next_height
    }

    /// Hash of the last block committed, if known, for checking that the next
    /// chunk extends the same chain.
    pub fn last_hash(&self) -> Option<Vec<u8>> {
        std::fs::read(self.dir.join("last_hash")).ok().filter(|h| h.len() == 32)
    }

    pub fn set_last_hash(&self, hash: &[u8]) -> anyhow::Result<()> {
        fs::write(self.dir.join("last_hash"), hash)?;
        Ok(())
    }

    /// Forgets everything from `height` on, after a reorg.
    pub fn rewind(&mut self, height: u32) -> anyhow::Result<()> {
        let height = height.min(self.next_height);
        fs::write(self.dir.join("progress"), height.to_string())?;
        let _ = fs::remove_file(self.dir.join("last_hash"));
        self.next_height = height;
        self.truncate_unfinished()
    }

    fn log_path(&self) -> PathBuf {
        self.dir.join("txs.bin")
    }

    /// Drops records from a chunk whose progress was never written.
    fn truncate_unfinished(&self) -> anyhow::Result<()> {
        let path = self.log_path();
        if !path.exists() {
            return Ok(());
        }
        let mut keep = 0u64;
        for item in RecordIter::open(&path)? {
            let (record, end) = match item {
                Ok(v) => v,
                Err(_) => break,
            };
            if record.key.height >= self.next_height {
                break;
            }
            keep = end;
        }
        OpenOptions::new().write(true).open(&path)?.set_len(keep)?;
        Ok(())
    }

    /// Appends one finished chunk and moves progress past it.
    pub fn commit(&mut self, records: &[Record], next_height: u32) -> anyhow::Result<()> {
        let file = OpenOptions::new().create(true).append(true).open(self.log_path())?;
        let mut w = BufWriter::new(file);
        for r in records {
            w.write_all(&r.key.height.to_le_bytes())?;
            w.write_all(&r.key.tx_index.to_le_bytes())?;
            w.write_all(&r.txid)?;
            w.write_all(&r.original_len.to_le_bytes())?;
            w.write_all(&(r.bytes.len() as u32).to_le_bytes())?;
            w.write_all(&r.bytes)?;
        }
        w.into_inner()?.sync_all()?;
        let tmp = self.dir.join("progress.tmp");
        fs::write(&tmp, next_height.to_string())?;
        fs::rename(tmp, self.dir.join("progress"))?;
        self.next_height = next_height;
        Ok(())
    }

    pub fn records(&self) -> anyhow::Result<impl Iterator<Item = anyhow::Result<Record>>> {
        let path = self.log_path();
        if !path.exists() {
            File::create(&path)?;
        }
        Ok(RecordIter::open(&path)?.map(|r| r.map(|(rec, _)| rec)))
    }
}

struct RecordIter {
    r: BufReader<File>,
    pos: u64,
}

impl RecordIter {
    fn open(path: &Path) -> anyhow::Result<Self> {
        let f = File::open(path).with_context(|| format!("open {}", path.display()))?;
        Ok(RecordIter { r: BufReader::with_capacity(1 << 20, f), pos: 0 })
    }
}

impl Iterator for RecordIter {
    type Item = anyhow::Result<(Record, u64)>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut head = [0u8; 4 + 4 + 32 + 4 + 4];
        match self.r.read_exact(&mut head) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return None,
            Err(e) => return Some(Err(e.into())),
        }
        let u32_at = |i: usize| u32::from_le_bytes(head[i..i + 4].try_into().unwrap());
        let len = u32_at(44) as usize;
        let mut bytes = vec![0u8; len];
        if let Err(e) = self.r.read_exact(&mut bytes) {
            return Some(Err(e.into()));
        }
        self.pos += (head.len() + len) as u64;
        Some(Ok((
            Record {
                key: TxKey { height: u32_at(0), tx_index: u32_at(4) },
                txid: head[8..40].try_into().unwrap(),
                original_len: u32_at(40),
                bytes,
            },
            self.pos,
        )))
    }
}
