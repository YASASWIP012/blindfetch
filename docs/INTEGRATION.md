# Adding Blindfetch to a wallet

Any wallet built on `zcash_client_backend` already has the hook. After a scan,
the wallet asks its data store for `transaction_data_requests()` and answers
each `TransactionDataRequest::Enhancement(txid)` by downloading the full
transaction. That download is the leak. Blindfetch replaces it and nothing else.

The zcash-devtool change in `patches/zcash-devtool-blindfetch.patch` is the
complete reference. The core of it:

```rust
let bf = bf_client::Blindfetch::connect(url).await?;

// Where each wanted transaction was mined. The wallet already stores this
// (`transactions.mined_height` and `tx_index` in zcash_client_sqlite).
let wants: Vec<bf_client::Want> = db
    .transaction_data_requests()?
    .into_iter()
    .filter_map(|r| match r {
        TransactionDataRequest::Enhancement(txid) => {
            mined_position(&txid).map(|key| bf_client::Want { key, txid })
        }
        _ => None,
    })
    .collect();

// Fixed schedule: call this on a timer with the same `rounds` every time.
let (results, _stats) = bf.fetch_tick(&params, &wants, 2).await?;

for (want, fetched) in results {
    if let bf_client::Fetched::Found(tx) = fetched {
        // Already checked against `want.txid` by the client.
        decrypt_and_store_transaction(&params, &mut db, &tx, Some(want.key.height.into()))?;
    }
}
```

## Rules a wallet must keep

1. **Never fall back to `GetTransaction` for an `Enhancement` request.** A
   transaction the database does not cover yet (the newest blocks) comes back
   as `NotCovered`; leave the request pending and try on the next tick.
2. **Stop on `bf_client::UNTRUSTED`.** It means the server sent data that does
   not match its own index or the chain. Do not retry against that server:
   retries would let it probe which queries were real. zcash-devtool records
   the server in `blindfetch-untrusted` next to the wallet database and refuses
   it from then on.
3. **Fetch on a timer, not on discovery.** If the wallet only fetches after it
   finds a note, the fetch itself says "something arrived". `fetch_tick` sends
   exactly `rounds` rounds every time and defers anything that does not fit.
4. **Do not relay stored transactions.** The bytes Blindfetch returns have
   zeroed proofs and signatures. They are correct for the wallet's records
   (same txid, same notes, same memos) and invalid on the network.

## Mobile

A tick costs about what the measurements in `docs/MEASUREMENTS.md` show for a
round, times `rounds`. A mobile wallet should tick when the app opens and then
on a slow timer (say every 30 minutes while open), and can use one round per
tick when few payments are expected.

## Running a server

`bf-indexer` and `bf-server` need nothing but a lightwalletd endpoint. The
database is a deterministic function of the chain: two operators who index
the same heights produce the same index, byte for byte, so wallets can compare
index digests across servers. A wallet company that already runs lightwalletd
can run Blindfetch beside it.
