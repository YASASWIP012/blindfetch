//! Smaller transactions that still prove who they are.
//!
//! Since ZIP 244 (and ZIP 229 for v6), a txid commits only to a transaction's
//! effecting data. Proofs, signatures and anchors are authorizing data and sit
//! outside it. The encrypted notes and memos are effecting data.
//!
//! So the server can drop the Orchard and Ironwood proofs, which are most of a
//! shielded transaction's size, and a wallet can still check what it received
//! by recomputing the txid. A server that alters a memo, swaps in another
//! transaction or serves stale data is caught, because the txid will not
//! match.
//!
//! The transaction parser insists that proofs have their canonical length,
//! so stripping replaces each proof with zeros of the same length and then
//! collapses long zero runs (`bf_core::zeros`). Decoding restores an
//! ordinary transaction encoding that any parser reads. It must never be
//! broadcast: with zeroed proofs it is not valid on chain.

#[cfg(feature = "strip")]
use orchard::{bundle::Authorized as BundleAuthorized, circuit::Proof};
#[cfg(feature = "strip")]
use zcash_primitives::transaction::{Authorized, TxVersion};
use zcash_primitives::transaction::{Transaction, TxId};
use zcash_protocol::consensus::BranchId;

/// Undoes the zero-run compression.
pub fn expand(bytes: &[u8]) -> Result<Vec<u8>, Error> {
    bf_core::zeros::decode(bytes, MAX_TX_BYTES).ok_or(Error::BadEncoding)
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("stripped transaction is not validly encoded")]
    BadEncoding,
    #[error("could not parse transaction: {0}")]
    Parse(#[from] std::io::Error),
    #[error("txid mismatch: expected {expected}, got {actual}")]
    TxidMismatch { expected: TxId, actual: TxId },
}

/// Largest transaction a client will expand a row into. Zcash blocks are at
/// most 2 MB.
pub const MAX_TX_BYTES: usize = 2_000_000;

/// A transaction with its Orchard and Ironwood proofs zeroed and compressed.
#[cfg(feature = "strip")]
#[derive(Clone, Debug)]
pub struct Stripped {
    pub txid: TxId,
    pub bytes: Vec<u8>,
    /// Size of the transaction as mined, for reporting savings.
    pub original_len: usize,
}

/// Removes the Orchard and Ironwood proofs from a raw transaction.
///
/// `branch` is only consulted for v4 transactions, whose encoding does not
/// carry a branch ID; v4 transactions have no Orchard or Ironwood bundle and
/// come back unchanged.
#[cfg(feature = "strip")]
pub fn strip(raw: &[u8], branch: BranchId) -> Result<Stripped, Error> {
    let tx = Transaction::read(raw, branch)?;
    Ok(Stripped { txid: tx.txid(), bytes: zero_auth(tx)?, original_len: raw.len() })
}

/// Re-strips bytes that an older indexer stored, zeroing anything it left in.
/// The txid check runs again, so a stored record that does not survive this
/// is reported, never silently changed.
#[cfg(feature = "strip")]
pub fn slim(stored: &[u8], branch: BranchId) -> Result<Vec<u8>, Error> {
    zero_auth(Transaction::read(&expand(stored)?[..], branch)?)
}

/// Transparent script signatures sit outside the v5/v6 txid too. Shielding
/// transactions often carry dozens of them.
#[cfg(feature = "strip")]
struct ZeroScriptSigs;

#[cfg(feature = "strip")]
impl transparent::bundle::MapAuth<transparent::bundle::Authorized, transparent::bundle::Authorized>
    for ZeroScriptSigs
{
    fn map_script_sig(&self, s: transparent::address::Script) -> transparent::address::Script {
        transparent::address::Script(zcash_script::script::Code(vec![0; s.0.0.len()]))
    }

    fn map_authorization(&self, a: transparent::bundle::Authorized) -> transparent::bundle::Authorized {
        a
    }
}

/// Zeroes every proof and signature the txid does not commit to, keeping
/// their lengths so the transaction still parses, then collapses the zeros.
#[cfg(feature = "strip")]
fn zero_auth(tx: Transaction) -> Result<Vec<u8>, Error> {
    use orchard::primitives::redpallas;

    let txid = tx.txid();
    if matches!(tx.version(), TxVersion::V4) {
        // v4 txids cover Sapling proofs, so nothing can be dropped. Check the
        // re-encoding anyway, like every other path.
        let mut raw = Vec::new();
        tx.write(&mut raw)?;
        let actual = Transaction::read(&raw[..], tx.consensus_branch_id())?.txid();
        if actual != txid {
            return Err(Error::TxidMismatch { expected: txid, actual });
        }
        return Ok(bf_core::zeros::encode(&raw));
    }

    let orchard_like = |bundle: Option<orchard::Bundle<BundleAuthorized, _>>| {
        bundle.map(|b| {
            b.map_authorization(
                &mut (),
                |_, _, _sig| redpallas::Signature::<redpallas::SpendAuth>::from([0u8; 64]),
                |_, auth: BundleAuthorized| {
                    let len = auth.proof().as_ref().len();
                    BundleAuthorized::from_parts(
                        Proof::new(vec![0; len]),
                        redpallas::Signature::<redpallas::Binding>::from([0u8; 64]),
                    )
                },
            )
        })
    };
    let sapling_like = |bundle: Option<sapling::Bundle<sapling::bundle::Authorized, _>>| {
        bundle.map(|b| {
            b.map_authorization(
                (),
                |_, _| [0u8; 192],
                |_, _| [0u8; 192],
                |_, _| redjubjub::Signature::<redjubjub::SpendAuth>::from([0u8; 64]),
                |_, _| sapling::bundle::Authorized { binding_sig: redjubjub::Signature::from([0u8; 64]) },
            )
        })
    };
    let transparent_like =
        |bundle: Option<transparent::bundle::Bundle<transparent::bundle::Authorized>>| {
            bundle.map(|b| b.map_authorization(ZeroScriptSigs))
        };
    let stripped = tx
        .into_data()
        .map_bundles::<Authorized>(transparent_like, sapling_like, orchard_like)
        .freeze()?;

    // Dropping authorizing data must not change the txid. If it ever does,
    // the indexer would publish rows no wallet could verify.
    let actual = stripped.txid();
    if actual != txid {
        return Err(Error::TxidMismatch { expected: txid, actual });
    }
    let mut zeroed = Vec::new();
    stripped.write(&mut zeroed)?;
    Ok(bf_core::zeros::encode(&zeroed))
}

/// Parses stripped bytes and checks them against the txid the wallet asked
/// for. This is the only check a wallet needs: the txid commits to every
/// note, memo and value in the transaction.
pub fn verify(bytes: &[u8], expected: TxId, branch: BranchId) -> Result<Transaction, Error> {
    let tx = Transaction::read(&expand(bytes)?[..], branch)?;
    let actual = tx.txid();
    if actual != expected {
        return Err(Error::TxidMismatch { expected, actual });
    }
    Ok(tx)
}
