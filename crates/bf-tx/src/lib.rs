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
//! The stripped bytes are an ordinary transaction encoding with zero-length
//! proofs, so any code that parses transactions can read them. They must
//! never be broadcast: without proofs they are not valid on chain.

use orchard::{bundle::Authorized as BundleAuthorized, circuit::Proof};
use zcash_primitives::transaction::{Authorized, Transaction, TxId, TxVersion};
use zcash_protocol::consensus::BranchId;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not parse transaction: {0}")]
    Parse(#[from] std::io::Error),
    #[error("txid mismatch: expected {expected}, got {actual}")]
    TxidMismatch { expected: TxId, actual: TxId },
}

/// A transaction with its Orchard and Ironwood proofs removed.
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
pub fn strip(raw: &[u8], branch: BranchId) -> Result<Stripped, Error> {
    let tx = Transaction::read(raw, branch)?;
    let txid = tx.txid();
    if matches!(tx.version(), TxVersion::V4) {
        return Ok(Stripped { txid, bytes: raw.to_vec(), original_len: raw.len() });
    }

    let without_proofs = |bundle: Option<orchard::Bundle<BundleAuthorized, _>>| {
        bundle.map(|b| {
            b.map_authorization(
                &mut (),
                |_, _, sig| sig,
                |_, auth: BundleAuthorized| {
                    BundleAuthorized::from_parts(Proof::new(vec![]), *auth.binding_signature())
                },
            )
        })
    };
    let data = tx
        .into_data()
        .map_bundles::<Authorized>(|t| t, |s| s, without_proofs);
    let stripped = data.freeze()?;

    // Dropping authorizing data must not change the txid. If it ever does,
    // the indexer would publish rows no wallet could verify.
    let actual = stripped.txid();
    if actual != txid {
        return Err(Error::TxidMismatch { expected: txid, actual });
    }
    let mut bytes = Vec::with_capacity(raw.len() / 2);
    stripped.write(&mut bytes)?;
    Ok(Stripped { txid, bytes, original_len: raw.len() })
}

/// Parses stripped bytes and checks them against the txid the wallet asked
/// for. This is the only check a wallet needs: the txid commits to every
/// note, memo and value in the transaction.
pub fn verify(bytes: &[u8], expected: TxId, branch: BranchId) -> Result<Transaction, Error> {
    let tx = Transaction::read(bytes, branch)?;
    let actual = tx.txid();
    if actual != expected {
        return Err(Error::TxidMismatch { expected, actual });
    }
    Ok(tx)
}
