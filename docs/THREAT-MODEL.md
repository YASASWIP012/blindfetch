# Threat model

## Who we defend against

The operator of the server a wallet reads from, or anyone who can read that
server's logs or traffic: a hosting company, a subpoena, an attacker who got
in. Today that is the lightwalletd operator. With Blindfetch it is the
Blindfetch operator too.

## What a stock light wallet reveals to lightwalletd

| Request | Reveals |
|---|---|
| `GetBlockRange` (compact blocks) | Nothing about which notes are yours: every wallet downloads the same blocks. It does reveal the wallet's birthday height and when it syncs. |
| `GetTransaction(txid)` for enhancement | **Which transactions are yours**, one request per received transaction, all from one IP and one session. |
| `GetTransaction(txid)` for status | Transactions you sent. The server saw them at broadcast. |
| `SendTransaction` | Transactions you sent. |
| `GetTaddressTxids` | Your transparent addresses. |

Blindfetch replaces the second row.

## What a Blindfetch server learns

- That an IP address downloaded the index and sent requests. Every request
  against one database version has the same size in both directions, and
  carries exactly `batch` queries.
- The time of each request.

## What it does not learn

- Which rows were asked for. This is the PIR guarantee: YPIR queries are
  LWE ciphertexts, and the server's view of a query for row 5 is
  indistinguishable from a query for row 5,000 without breaking the
  underlying lattice assumption.
- How many queries in a round were real. Dummies are queries for random rows,
  built and encrypted the same way.
- Which transactions. Rows hold packed transactions in chain order, and the
  index is public and identical for everyone.

## Attacks and answers

**The server serves a wrong or altered transaction.** The wallet recomputes
the txid from what it received and compares it to the txid it found while
scanning compact blocks. Memos, notes and values are all under the txid.
Proofs are not, but the wallet does not need them; the chain already
verified them.

**The server serves a different index to one client.** A crafted index could
point a client at rows that reveal something only if row choice leaked, and it
does not. At worst the client fetches the wrong bytes and the txid check
fails. A wallet can also compare index hashes across servers.

**The server counts rounds.** A wallet that receives many payments needs more
rounds. A wallet that sends rounds on a timer, real or dummy, removes this
signal. `bf-client` exposes `cover_round` for that. Without it, the server
learns an upper bound: "this client fetched at most 8k rows".

**Timing.** A wallet that enhances right after a payment lands reveals that
something arrived, though not what. Rounds on a timer close this too.

**Network address.** PIR does nothing for the IP address. Run the client over
Tor or an HTTP relay if that matters. The server never writes client
addresses to its logs; the reverse proxy in our deployment is configured not
to either.

**Database version as a fingerprint.** Every client is told the same version
and must use it; the server refuses queries for any other.

## Out of scope

- Transactions mined before Ironwood activation.
- Network-level adversaries who watch both the client and other parties.
- Bugs in `valar-ypir`. Its authors had it audited by Zellic; we use its
  client and server code unchanged.
