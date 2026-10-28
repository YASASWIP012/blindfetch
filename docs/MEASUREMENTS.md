# Measurements

All numbers were measured on 28 October 2026 on an Apple M1 MacBook Air with
8 GB of memory, the same machine that serves the public demo. Mainnet data
came from zec.rocks.

## Storage

Zeroing what the txid does not cover (proofs, signatures, transparent script
signatures) and collapsing the zeros:

| Mainnet blocks | Transactions | Mined size | Stored size | Ratio |
|---|---|---|---|---|
| 3,529,900 to 3,530,800 | 12,969 | 120.1 MB | 22.8 MB | 5.3× |
| 3,532,500 to 3,533,662 | 4,442 | 42.2 MB | 9.6 MB (proofs only) | 4.4× |

Every stored transaction was checked against its txid when it was stored, and
every transaction fetched by a client is checked again.

The index costs about 3.5 bytes per transaction plus 16 bytes per row of
hashes. For the full Ironwood history (roughly 400,000 transactions in about
1 GiB of rows) that is about 2 MB, downloaded once per database version.

## Server cost per query

From `cargo run --release --features server --example memprobe -- <rows> 32768`
in `crates/bf-pir`, one query answered on one core:

| Database | Rows | Start-up (precompute) | One query | Query up | Answer down |
|---|---|---|---|---|---|
| 64 MiB | 2,048 | 9.8 s | 0.63 s | 544 KiB | 120 KiB |
| 128 MiB | 4,096 | 10.9 s | 0.61 s | 560 KiB | 120 KiB |
| 1 GiB | 32,768 | 30.2 s | 1.15 s | 784 KiB | 120 KiB |

Peak memory stayed near 1.2 GB plus the database. The server answers the four
queries of a round on four cores at once.

Row width matters because YPIR's query shrinks as rows widen while its answer
grows. At a 1 GiB database, client-only query sizes:

| Row size | Query size |
|---|---|
| 4 KiB | 2,576 KiB |
| 16 KiB | 1,040 KiB |
| 32 KiB | 784 KiB |
| 64 KiB | 656 KiB |

32 KiB rows give the smallest round trip once the answers are counted.

## Round trips

A round is four queries. Measured end to end against the 64 MiB sample
database:

| Path | Up | Down | Time |
|---|---|---|---|
| Client and server on the same machine | 2.13 MiB | 0.47 MiB | 0.6 s |
| Through the public URL (Vercel, Cloudflare tunnel, home connection) | 2.13 MiB | 0.47 MiB | 6 to 9 s |
| Browser demo through the public URL | 2.1 MiB | 480 KiB | 6.2 s round trip, 184 ms to build the queries, 55 ms to open the answers |

Public-URL times are dominated by the home connection the server sits on.

## A real wallet

`demo/wallet-demo.sh`, a watch-only mainnet wallet with three transactions to
read, against a server covering blocks 3,529,900 to 3,530,800:

| | Requests naming the wallet's transactions | Rounds | Up | Down | Time |
|---|---|---|---|---|---|
| Stock zcash-devtool | 3 `GetTransaction` to lightwalletd | | | | |
| `enhance --blindfetch` | 0 | 2 of 4 queries | 4.3 MiB | 0.9 MiB | 1.5 s |

Both wallets ended with identical transaction and memo listings.

On a fixed schedule (`--every 15 --rounds 2`), three ticks, the first of which
fetched the wallet's three transactions and the other two nothing, each left
the same record in the server's log: two requests of 2,228,276 bytes in and
491,540 bytes out.

## What this costs a wallet

At the full database size, a tick of one round (the default) is about
3.1 MiB up and 0.5 MiB down. Downloading the index is about 2 MB once per version. A desktop
wallet can tick every few minutes. A mobile wallet should tick when the app
opens and then rarely.

For comparison, a wallet that wanted perfect privacy without PIR would
download every stored transaction in the range it scanned: about 8 MB per day
of chain at current volume (about 7 KB per block), so about 55 MB for a wallet
that was offline for a week, against one or two ticks for Blindfetch.
