# Threat model

## Adversary

The operator of the server a wallet reads from, or anyone who can read that
server's logs or traffic: a hosting company, a subpoena, an attacker who got
in. Today that is the lightwalletd operator. With Blindfetch it is also the
Blindfetch operator, who may be malicious.

## What a stock light wallet tells lightwalletd

| Request | Reveals |
|---|---|
| `GetBlockRange` (compact blocks) | The wallet's birthday height and when it syncs. Every wallet downloads the same blocks, so nothing about which notes are its own. |
| `GetTransaction(txid)` for enhancement | **Which transactions are the wallet's**, one request per received transaction, in one session from one address. |
| `GetTransaction(txid)` for status | Transactions the wallet sent. The server saw them at broadcast. |
| `SendTransaction` | Transactions the wallet sent. |
| `GetTaddressTxids` | The wallet's transparent addresses. |

Blindfetch replaces the second row. zcash-devtool in Blindfetch mode never
sends it, even for transactions the database does not cover yet; those wait.

## What a Blindfetch server learns

- That an address downloaded the index and sent requests. Every request
  against one database version has the same size in each direction and carries
  exactly `batch` queries. Responses are never compressed.
- The time of each request. A wallet on a fixed schedule makes this the same
  whether or not it received anything.

## What it does not learn

- Which rows were asked for. YPIR queries are LWE ciphertexts. Telling a query
  for row 5 from a query for row 5,000 means breaking the lattice assumption.
- How many queries in a round were real. Dummies are queries for random rows,
  built the same way, in random slots.
- Which transactions. Rows hold packed transactions in chain order, and the
  index is public and the same for everyone.

## Attacks

**Wrong or altered transaction.** The wallet recomputes the txid of what it
received and compares it with the txid it found while scanning. Memos, notes
and values are all under the txid.

**Selective failure.** A server corrupts one answer, or answers from a
database with some rows changed, and watches whether the client reacts. Every
answer, dummies included, is checked against the row hashes in the index, so a
corrupted dummy fails the same way as a corrupted real row. Any failure ends the
session with the same error, and zcash-devtool locks the server out. The server
gets at most one failed session per wallet, and a failure that does not depend
on which rows were real.

**A lying index.** A server could publish an index whose hashes match its own
altered rows. Real rows would then fail the txid check, which ends the session
the same way. A wallet can also compare the index digest with a second server,
since the index is a deterministic function of the chain.

**Per-client versions.** A server could hand each client its own database
version and recognise it later by the version header. Clients that compare
`/v1/info` with a second server, or with the expected tip, see the
mismatch. This is the weakest point in the current design: versions are not
yet signed or cross-checked automatically.

**Counting rounds.** Without a fixed schedule, a wallet that needs more rows
sends more rounds, so the server learns an upper bound on how many rows it
fetched. `fetch_tick` and `enhance --every` remove this.

**Timing.** A wallet that fetches right after a payment arrives reveals that
something arrived. The fixed schedule removes this too.

**Network address.** PIR does nothing for the IP address. Run the client over
Tor or a relay if that matters. The server does not log addresses. The public
demo is proxied through Vercel and Cloudflare, which see addresses as any proxy
does.

## Out of scope

- Transactions mined before Ironwood activation.
- Network observers who watch both the client and other parties.
- Denial of service.
- Bugs in `valar-ypir`. We use its client and server code unchanged.
