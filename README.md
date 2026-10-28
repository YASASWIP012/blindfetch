# Blindfetch

Zcash light wallets read their memos by asking the server for each received
transaction by txid. That request tells the server which payments are yours.
Blindfetch answers the same request over private information retrieval (PIR):
the wallet gets its transaction, checks it against the txid, and the server
cannot tell which one it sent.

ZECATHON 2026, Core & Tooling.

| | |
|---|---|
| Live demo | https://blindfetch.vercel.app |
| Mainnet server | https://blindfetch.vercel.app/main/v1/info |
| Video | VIDEO_URL |
| Integration guide | [docs/INTEGRATION.md](docs/INTEGRATION.md) |
| Spec | [docs/SPEC.md](docs/SPEC.md) |
| Threat model | [docs/THREAT-MODEL.md](docs/THREAT-MODEL.md) |
| Measurements | [docs/MEASUREMENTS.md](docs/MEASUREMENTS.md) |
| What is real | [docs/WHAT-IS-REAL.md](docs/WHAT-IS-REAL.md) |

## The leak

A light wallet finds its payments by trial-decrypting compact blocks
([ZIP 307](https://zips.z.cash/zip-0307)). Compact blocks carry only the first
52 bytes of each note ciphertext: enough to find a note and its value, not
enough for the memo. So the wallet asks lightwalletd for the whole
transaction, by txid. In librustzcash this is
`TransactionDataRequest::Enhancement`, and zcash-devtool's own `enhance`
command carries a TODO about it ("Create an isolated `lightwalletd` connection
for each transaction... with timing noise").

Every one of those requests names a transaction you received. One session, one
IP, a list of your payments.

## What Blindfetch does

1. **Index.** `bf-indexer` walks the chain from Ironwood activation and keeps
   every transaction with a shielded output. It zeroes the proofs and
   signatures, which sit outside the txid (ZIP 244, ZIP 229), and collapses
   the zeros. On mainnet blocks 3,529,900 to 3,530,800 that cuts 120 MB of
   transactions to 23 MB, and every one still hashes to its txid.
2. **Serve.** `bf-server` packs them into 32 KiB rows and answers YPIR queries
   over the rows. YPIR comes from `valar-ypir`, the Zellic-audited crate ZODL
   uses for private nullifier lookups in shielded voting.
3. **Fetch.** A wallet already knows where its transactions were mined. It
   downloads the whole index (a few bytes per transaction), finds its rows,
   and asks for them in rounds of exactly four queries, with queries for
   random rows shuffled in. On a fixed schedule (`--every`) it sends the same
   number of rounds every tick, whether or not anything arrived.
4. **Verify.** The wallet checks every row, dummies too, against a hash in the
   index and recomputes each transaction's txid. A server that sends anything
   wrong is locked out, and a scheduled wallet keeps sending it dummy ticks so
   the server cannot tell which wallet caught it. `--compare-with` checks the
   index against a second server before any query.

## Try it

```bash
cargo build --release -p bf-indexer -p bf-server -p zcash-devtool

# Index a recent slice of mainnet and serve it
./target/release/bf-indexer --network main --data data --from 3532500 sync
./target/release/bf-indexer --network main --data data --from 3532500 build
./target/release/bf-server --data data/main &

# Fetch random indexed transactions privately and check them against their txids
./target/release/bf-indexer --network main --data data --from 3532500 bench

# A wallet that reads its memos through Blindfetch, on a fixed schedule
./target/release/zcash-devtool wallet -w <dir> enhance --blindfetch http://127.0.0.1:8484 --every 300

# The mainnet wallet demo: stock path against Blindfetch, same memos
./demo/wallet-demo.sh
```

## Layout

| Crate | What it does |
|---|---|
| `bf-core` | Index format, row math, fixed-size query rounds, zero-run codec. No Zcash or network code. |
| `bf-tx` | Zeroes proofs; checks stripped bytes against a txid. |
| `bf-pir` | Thin wrapper over `valar-ypir`. |
| `bf-indexer` | Builds the database from lightwalletd; `bench` measures fetches. |
| `bf-server` | `/v1/info`, `/v1/index`, `/v1/query`, and `/v1/seen`, its own request log. |
| `bf-client` | What a wallet links. |
| `bf-wasm` + `web/` | The browser demo. |
| `vendor/zcash-devtool` | Upstream zcash-devtool with `enhance --blindfetch`. The change alone is in `patches/`. |

## Measured

On an Apple M1 laptop: a query against a 1 GiB database takes 1.15 s on one
core, with 784 KiB up and 120 KiB down. A round of four queries through the
public URL takes 6 to 9 seconds, most of it the home connection the server
sits on. Details in [docs/MEASUREMENTS.md](docs/MEASUREMENTS.md).

## Limits

Read [docs/THREAT-MODEL.md](docs/THREAT-MODEL.md) and
[docs/WHAT-IS-REAL.md](docs/WHAT-IS-REAL.md). In short: transactions mined
before Ironwood activation are not covered, the newest blocks are covered once
the server catches up, PIR does nothing for your IP address, and the public
server runs on a laptop because the project had no budget for hosting.

## License

MIT or Apache-2.0, at your option.
