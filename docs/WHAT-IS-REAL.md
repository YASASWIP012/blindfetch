# What is real and what is not

Everything below ran on Zcash mainnet unless it says otherwise. Nothing in the
demo is simulated or replayed.

## Real

| Claim | How to check it |
|---|---|
| The server indexes every transaction with a shielded output since Ironwood activation (block 3,428,143) from a public lightwalletd (zec.rocks). | `GET /v1/info` on the live server gives the height range and transaction count. `bf-indexer sync` rebuilds the same database from scratch. |
| Zeroing proofs and signatures leaves every txid unchanged. | `bf-tx::strip` recomputes the txid of every transaction it stores and skips any that changes (none have). `stats.json` counts records that could not be re-stripped. The client recomputes every txid on receipt. |
| Stored transactions are about 5 times smaller than mined ones. | `stats.json` from `bf-indexer build`: all of Ironwood (410,977 transactions) is 5.16 GB mined and 1.09 GB stored; blocks 3,529,900 to 3,530,800 alone are 120.1 MB and 22.8 MB. |
| PIR uses `valar-ypir` 0.2.1 from crates.io, unchanged. | `crates/bf-pir/Cargo.toml`. Its README links the Zellic audit report. |
| A real mainnet wallet reads its memos through Blindfetch with no txid sent to lightwalletd, and ends with the same memos as the stock path. | `demo/wallet-demo.sh`. The wallet is watch-only, restored from a viewing key its owners published for audit (the Tacit project's mainnet lot 2 vault). |
| The browser demo fetches any indexed mainnet transaction over PIR. | blindfetch.vercel.app. Paste the transaction's txid from any explorer into the "expected txid" field to have the page check it. |
| A tick on the fixed schedule produces the same server-side traffic whether or not the wallet had anything to fetch. | `enhance --blindfetch URL --every 15 --rounds 2`, then read `/v1/seen`. |
| A server that sends a wrong row, real or dummy, is caught and locked out. | `bf_client::UNTRUSTED`, row hashes in `crates/bf-core/src/index.rs`, lockout in the devtool `enhance` command. |
| A server whose index was altered to target one transaction is refused before any query when compared with a second server. | `enhance --blindfetch A --compare-with B`. Tested against a copy with one transaction changed. |

## Not real, or not yet

- **Hosting.** The public server is one Azure VM (2 vCPU, 8 GB, Korea
  Central) paid from a student credit, with Caddy in front and access logs
  turned off. Vercel proxies a stable address to it. There is one operator:
  the author. If the server is ever unreachable, the page says so and shows a
  recorded run, labelled as a recording. The steps to run your own are in
  `deploy/`.
- **Wallets.** The only wallet integration is zcash-devtool, opened upstream
  as [zcash/zcash-devtool#270](https://github.com/zcash/zcash-devtool/pull/270)
  on 28 October and not merged. Zodl, Zingo and other wallets would need the
  change described in `docs/INTEGRATION.md`.
- **No faucet coins.** The testnet faucet was not paying out on 28 October, so
  the wallet demo uses a published mainnet viewing key instead of a wallet of
  our own.
- **History before Ironwood.** Not indexed. Wallets with older payments
  still fetch those the old way, or over Tor.
- **Malicious-server PIR.** YPIR keeps the row secret from an honest-but-curious
  server. The row hashes and the lockout stop a malicious server from learning
  anything from failures. A server can still refuse service.
