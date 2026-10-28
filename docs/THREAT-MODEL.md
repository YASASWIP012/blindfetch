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
sends it. A transaction the database does not cover yet stays pending, and on
a fixed schedule a later tick fetches it once the server has indexed it.

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

**Selective failure.** A server corrupts one answer and watches whether the
client reacts. Every answer, dummies included, is checked against the row
hashes in the index, so a corrupted dummy fails the same way as a corrupted
real row.

**A lying index.** The server writes the index, so it can change one
transaction T and publish a matching row hash. Dummies then pass and only a
wallet that wanted T fails its txid check. Two things stop the server from
learning who that was:

- The client sends every planned round even after a failure, and a wallet on a
  fixed schedule keeps sending dummy-only ticks to a server that failed a
  check. The server sees the same traffic either way. The wallet records the
  server as locked out and alerts its user locally.
- `--compare-with <url>` checks the index digest against a second operator
  before any query is sent. The index is a function of the chain and the start
  height, so honest servers at the same height publish the same bytes. We
  tested this against a copy with one altered transaction; the wallet refused
  it before querying.

What remains: a wallet run once by hand, with no second server to compare
against, that hits a lying index exits with an error after its planned rounds.
If its user then behaves differently where the server can see it (for example
by retrying at once), the server can learn something. A single honest second
operator removes this.

**Per-client versions.** A server could hand each client its own database
version and recognise it later by the version header. zcash-devtool refuses a
version above its own chain tip, and `--compare-with` refuses one that a
second operator does not serve. Without a second operator, a server can still
pick among recent heights to tag clients.

**Counting rounds.** zcash-devtool sends exactly `--rounds` rounds (1 by
default, four queries) on every fetch and leaves what does not fit for the
next fetch. The number of rounds never depends on how many
transactions the wallet wanted. The library's `fetch` (rounds as needed, at
least two) remains for tools that do not care, such as the benchmark.

**Timing.** A wallet that fetches right after a payment arrives reveals that
something arrived, and over weeks those times can be intersected with the
blocks that carried shielded outputs. The rule is that a fetch never
depends on what arrived: zcash-devtool fetches exactly one round per `enhance`
call, to be run after every `sync` whether or not it found anything, so the
fetch reveals only that a sync happened, which lightwalletd already sees.
`--every N` instead fetches on a timer while it runs. Fetching only when
something arrived reveals arrivals; the spec says MUST NOT and the help text
says so.

**Withheld transactions.** A server could leave a transaction out of its
index. The wallet knows from compact blocks where each of its transactions was
mined; a transaction inside the index's height range that the index does not
list is reported as `Missing`, and zcash-devtool warns that the server may be
withholding it. zcash-devtool also refuses a server whose database is more
than 300 blocks behind the wallet's own tip.

**History before Ironwood.** Transactions mined before Ironwood activation are
not in the database. In Blindfetch mode zcash-devtool leaves them pending; it
never falls back to lightwalletd for them unless `--allow-fallback` is given.

**Network address.** PIR does nothing for the IP address. Run the client over
Tor or a relay if that matters. The server does not log addresses. The public
demo is proxied through Vercel, which sees addresses as any proxy does, and
Caddy on the server writes no access log.

## Out of scope

- Transactions mined before Ironwood activation.
- Network observers who watch both the client and other parties.
- Denial of service.
- Bugs in `valar-ypir`. We use its client and server code unchanged.
