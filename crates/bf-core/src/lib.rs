//! Shared layout for Blindfetch.
//!
//! The server keeps every shielded transaction since Ironwood activation in one
//! packed byte stream, with proofs removed (they are authorizing data, so the
//! txid does not cover them). The stream is cut into fixed-size rows, and the
//! rows are what the PIR database serves. A small index, which every client
//! downloads in full, says where each transaction starts and how long it is.
//!
//! Nothing in this crate talks to the network or knows about Zcash types; it
//! only does the arithmetic both sides must agree on.

mod index;
mod plan;
mod varint;
pub mod wire;

pub use index::{Entry, Index, IndexError, Packer, TxKey};
pub use plan::{Batch, QueryPlan};

/// Bytes per PIR row. YPIR's SimplePIR mode needs at least 28,672 bits
/// (3,584 bytes) per item; 4 KiB holds a stripped two-action Ironwood
/// transaction with room to spare.
pub const ROW_BYTES: usize = 4096;

/// YPIR's SimplePIR mode needs at least this many rows. Smaller databases are
/// padded with zero rows.
pub const MIN_ROWS: usize = 2048;

/// Every round a client sends exactly this many row queries, real or not, so
/// the server cannot count how many transactions a wallet is fetching.
pub const DEFAULT_BATCH: usize = 8;

/// Which chain an index or database describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Main,
    Test,
}

impl Network {
    pub fn tag(self) -> u8 {
        match self {
            Network::Main => 0,
            Network::Test => 1,
        }
    }

    pub fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            0 => Some(Network::Main),
            1 => Some(Network::Test),
            _ => None,
        }
    }
}

/// Number of rows a packed stream of `stream_len` bytes occupies, after
/// padding to YPIR's minimum and up to a power of two, which is the shape
/// YPIR's SimplePIR parameters are derived for.
pub fn row_count(stream_len: u64) -> usize {
    let rows = stream_len.div_ceil(ROW_BYTES as u64) as usize;
    rows.max(MIN_ROWS).next_power_of_two()
}
