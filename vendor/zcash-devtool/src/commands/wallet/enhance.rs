use std::collections::BTreeSet;

use anyhow::anyhow;
use rusqlite::OptionalExtension;
use clap::Args;
use futures_util::StreamExt;
use rand::{rand_core::UnwrapErr, rngs::SysRng};
use tonic::{Code, transport::Channel};
use tracing::info;
use zcash_client_backend::{
    data_api::{
        TransactionDataRequest, TransactionStatus, WalletRead, WalletWrite,
        wallet::decrypt_and_store_transaction,
    },
    proto::service::{
        self, BlockRange, RawTransaction, compact_tx_streamer_client::CompactTxStreamerClient,
    },
};
use zcash_client_sqlite::{WalletDb, util::SystemClock};
use zcash_keys::encoding::AddressCodec;
use zcash_primitives::transaction::{Transaction, TxId};
use zcash_protocol::consensus::{BlockHeight, BranchId, Parameters};

use crate::{config::get_wallet_network, data::get_db_paths, remote::ConnectionArgs};

// Options accepted for the `enhance` command
#[derive(Debug, Args)]
pub(crate) struct Command {
    #[command(flatten)]
    connection: ConnectionArgs,

    /// Fetch received transactions from this Blindfetch server over PIR
    /// instead of asking lightwalletd for each txid.
    #[arg(long)]
    blindfetch: Option<String>,

    /// With --blindfetch, fetch transactions the Blindfetch database does not
    /// cover from lightwalletd by txid. This reveals those txids.
    #[arg(long, requires = "blindfetch")]
    allow_fallback: bool,
}

/// Where the wallet saw a transaction mined, which is all a Blindfetch server
/// needs to be asked for it.
fn mined_position(
    db_path: &std::path::Path,
    txid: &TxId,
) -> Result<Option<bf_core::TxKey>, anyhow::Error> {
    let conn = rusqlite::Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let row = conn
        .query_row(
            "SELECT mined_height, tx_index FROM transactions WHERE txid = ?1",
            [txid.as_ref().as_slice()],
            |r| Ok((r.get::<_, Option<u32>>(0)?, r.get::<_, Option<u32>>(1)?)),
        )
        .optional()?;
    Ok(match row {
        Some((Some(height), Some(tx_index))) => Some(bf_core::TxKey { height, tx_index }),
        _ => None,
    })
}

fn parse_raw_transaction<P: Parameters>(
    params: &P,
    chain_tip: BlockHeight,
    tx: RawTransaction,
) -> Result<(Transaction, Option<BlockHeight>), anyhow::Error> {
    let mined_height = (tx.height > 0 && tx.height <= u64::from(u32::MAX))
        .then(|| BlockHeight::from_u32(u32::try_from(tx.height).unwrap()));

    let tx = Transaction::read(
        &tx.data[..],
        // We assume unmined transactions are created with the current consensus branch ID.
        BranchId::for_height(params, mined_height.unwrap_or(chain_tip)),
    )?;

    Ok((tx, mined_height))
}

async fn fetch_transaction<P: Parameters>(
    client: &mut CompactTxStreamerClient<Channel>,
    params: &P,
    chain_tip: BlockHeight,
    txid: TxId,
) -> Result<Option<(Transaction, Option<BlockHeight>)>, anyhow::Error> {
    let request = service::TxFilter {
        hash: txid.as_ref().to_vec(),
        ..Default::default()
    };
    // This is the request Blindfetch replaces: it names the transaction.
    println!("lightwalletd GetTransaction {txid}");

    let raw_tx = match client.get_transaction(request).await {
        Ok(response) => Ok(Some(response.into_inner())),
        Err(status) => {
            if status.code() == Code::NotFound {
                Ok(None)
            } else {
                Err(status)
            }
        }
    }?;

    raw_tx
        .map(|raw_tx| parse_raw_transaction(params, chain_tip, raw_tx))
        .transpose()
}

impl Command {
    pub(crate) async fn run(self, wallet_dir: Option<String>) -> Result<(), anyhow::Error> {
        let params = get_wallet_network(wallet_dir.as_ref())?;
        let (_, db_path) = get_db_paths(wallet_dir.as_ref());

        let mut db_data = WalletDb::for_path(&db_path, params, SystemClock, UnwrapErr(SysRng))?;
        let chain_tip = db_data.chain_height()?.ok_or_else(|| {
            anyhow!("Chain height must be available to perform transaction enhancement.")
        })?;

        // TODO:
        // - Create a shared Tor client.
        // - Create an isolated `lightwalletd` connection for each transaction.
        // - Spread transactions across all available servers.
        // - Fetch transactions in parallel, with timing noise.
        let mut client = self.connection.connect(params, wallet_dir.as_ref()).await?;

        let mut satisfied_requests = BTreeSet::new();

        let strict = self.blindfetch.is_some() && !self.allow_fallback;
        if let Some(url) = &self.blindfetch {
            let mut bf = bf_client::Blindfetch::connect(url).await?;
            let info = bf.info();
            println!(
                "Blindfetch: {} transactions, heights {}..={}, {} rows, {} queries per round",
                info.transactions, info.first_height, info.last_height, info.num_rows, info.batch
            );

            let mut wants = Vec::new();
            let mut unlocated = Vec::new();
            for request in db_data.transaction_data_requests()? {
                if let TransactionDataRequest::Enhancement(txid) = &request {
                    match mined_position(&db_path, txid)? {
                        Some(key) => wants.push(bf_client::Want { key, txid: *txid }),
                        None => unlocated.push(*txid),
                    }
                }
            }

            // A rebuilt database is the one failure worth retrying, once. Any
            // other failure means the server misbehaved: stop, do not probe.
            let (results, rounds) = match bf.fetch(&params, &wants).await {
                Err(e) if e.to_string().contains("rebuilt its database") => {
                    bf = bf_client::Blindfetch::connect(url).await?;
                    bf.fetch(&params, &wants).await?
                }
                other => other?,
            };
            let mut not_covered = unlocated;
            for (want, fetched) in results {
                match fetched {
                    bf_client::Fetched::Found(tx) => {
                        info!("Enhancing tx {:?} from Blindfetch", want.txid);
                        decrypt_and_store_transaction(
                            &params,
                            &mut db_data,
                            &tx,
                            Some(BlockHeight::from_u32(want.key.height)),
                        )?;
                        satisfied_requests.insert(TransactionDataRequest::Enhancement(want.txid));
                    }
                    bf_client::Fetched::NotCovered => not_covered.push(want.txid),
                }
            }

            let (up, down, ms): (usize, usize, u128) = rounds
                .iter()
                .fold((0, 0, 0), |a, r| (a.0 + r.bytes_up, a.1 + r.bytes_down, a.2 + r.millis));
            println!(
                "Fetched {} of {} transactions in {} round(s) of {} queries: {:.1} MiB up, {:.1} MiB down, {} ms",
                wants.len() - not_covered.len().min(wants.len()),
                wants.len(),
                rounds.len(),
                info.batch,
                up as f64 / 1048576.0,
                down as f64 / 1048576.0,
                ms
            );
            println!("lightwalletd was not asked for any of them.");

            if !self.allow_fallback {
                for txid in &not_covered {
                    println!("Not in the Blindfetch database, left for later: {txid}");
                    satisfied_requests.insert(TransactionDataRequest::Enhancement(*txid));
                }
            }
        }

        loop {
            let mut new_request_encountered = false;
            for data_request in db_data.transaction_data_requests()? {
                if satisfied_requests.contains(&data_request) {
                    continue;
                } else {
                    new_request_encountered = true;
                }

                info!("Fetching data for request {:?}", data_request);
                match &data_request {
                    TransactionDataRequest::GetStatus(txid) => {
                        let status = fetch_transaction(&mut client, &params, chain_tip, *txid)
                            .await?
                            .map_or(TransactionStatus::TxidNotRecognized, |(_, mined_height)| {
                                mined_height.map_or(
                                    TransactionStatus::NotInMainChain,
                                    TransactionStatus::Mined,
                                )
                            });
                        info!("Got status {:?}", status);
                        db_data.set_transaction_status(*txid, status)?;
                    }
                    TransactionDataRequest::Enhancement(txid) if strict => {
                        // Asking lightwalletd would name the transaction.
                        println!("Deferred, not asking lightwalletd: {txid}");
                    }
                    TransactionDataRequest::Enhancement(txid) => {
                        match fetch_transaction(&mut client, &params, chain_tip, *txid).await? {
                            None => {
                                info!("Txid not recognized {:?}", txid);
                                db_data.set_transaction_status(
                                    *txid,
                                    TransactionStatus::TxidNotRecognized,
                                )?;
                            }
                            Some((tx, mined_height)) => {
                                info!(
                                    "Enhancing tx {:?} with mined height {:?}",
                                    txid, mined_height
                                );
                                decrypt_and_store_transaction(
                                    &params,
                                    &mut db_data,
                                    &tx,
                                    mined_height,
                                )?;
                            }
                        }
                    }
                    TransactionDataRequest::TransactionsInvolvingAddress(tia) => {
                        let address = tia.address().encode(&params);
                        let request = service::TransparentAddressBlockFilter {
                            address: address.clone(),
                            range: Some(BlockRange {
                                start: Some(service::BlockId {
                                    height: u64::from(tia.block_range_start()),
                                    ..Default::default()
                                }),
                                end: tia.block_range_end().map(|h| service::BlockId {
                                    height: u64::from(h - 1), // `BlockRange` end is inclusive.
                                    ..Default::default()
                                }),
                                pool_types: Default::default(),
                            }),
                        };

                        let mut stream = client.get_taddress_txids(request).await?.into_inner();
                        while let Some(raw_tx) = stream.next().await {
                            let (tx, mined_height) =
                                parse_raw_transaction(&params, chain_tip, raw_tx?)?;
                            info!(
                                "Found tx {:?} for address {} with mined height {:?}",
                                tx.txid(),
                                address,
                                mined_height
                            );
                            decrypt_and_store_transaction(&params, &mut db_data, &tx, mined_height)?
                        }
                    }
                }

                satisfied_requests.insert(data_request);
            }

            if !new_request_encountered {
                break;
            }
        }

        Ok(())
    }
}
