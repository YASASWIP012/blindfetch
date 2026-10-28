# Blindfetch protocol, draft 1

Status: draft, written for ZECATHON 2026. Format loosely follows a ZIP.

## Problem

A light wallet learns about payments by trial-decrypting compact blocks
(ZIP 307). Compact blocks carry only the first 52 bytes of each note
ciphertext, which is enough to find a note and read its value, but not its
memo. To read the memo, and to record the full transaction, the wallet asks
lightwalletd for the transaction by txid (`GetTransaction`). In
librustzcash this is `TransactionDataRequest::Enhancement`.

That request names a transaction the wallet received. Whoever runs the
server, or can read its logs, learns which transactions belong to the
client's IP address, and when the wallet came online to fetch them. Tor hides
the address but not that the requests in one session belong together.

## Overview

A Blindfetch server holds every transaction with a shielded output mined since
Ironwood activation, with Orchard and Ironwood proofs removed. It cuts them
into fixed-size rows and answers private information retrieval (PIR) queries
over those rows. The wallet already knows where each of its transactions was
mined (height and position in the block) from compact block scanning. It looks
the position up in a public index, which it downloads in full, and fetches the
rows over PIR in rounds of a fixed number of queries. It then recomputes the
txid of what it received and compares it to the txid it expected.

The PIR scheme is YPIR in SimplePIR mode, through the `valar-ypir` crate,
which is the implementation ZODL uses for shielded voting nullifier lookups and
which Zellic audited.

## Why stripped transactions verify

ZIP 244 splits a v5 transaction into effecting data, which the txid commits
to, and authorizing data, which it does not. ZIP 229 keeps this split for v6
and moves anchors into authorizing data. Proofs and signatures are authorizing
data. Note commitments, ephemeral keys, `encCiphertext` (which carries the
memo) and `outCiphertext` are effecting data.

So a transaction with its proofs removed has the same txid as the mined
transaction. A wallet that parses the stripped bytes and recomputes the txid
gets a check on every note, memo and value in it. A server that alters a memo,
serves a different transaction or serves bytes from another position fails
that check.

Stripped transactions are not valid on chain and must never be broadcast. A
wallet that stores them should treat the stored bytes as a record, not as
something to relay.

## Database

1. Start at the Ironwood activation height (mainnet 3,428,143; testnet
   4,134,000).
2. For each block in order, take every transaction whose compact form has at
   least one Sapling output, Orchard action or Ironwood action.
3. Replace each Orchard and Ironwood bundle's proof with an empty proof.
   Leave everything else as mined. Check the txid is unchanged.
4. Append the encoding to a byte stream, in (height, position) order.
5. Cut the stream into rows of 4,096 bytes. Pad with zero rows to at least
   2,048 rows and to a power of two.

The server stays a fixed number of blocks behind the tip (10 by default) so
that a reorg never rewrites rows a client already used.

## Index

The index lists, for each block with entries, each transaction's position in
the block and its stripped length. Offsets in the stream are running sums, so
an entry costs about four bytes. The encoding is in `crates/bf-core/src/index.rs`.

The last height in the index is the database version. Queries carry it in an
`x-db-version` header; a server that has moved on answers 409 and the client
downloads the new index.

Every client downloads the whole index, so downloading it says nothing about
the client except that it uses Blindfetch.

## Queries

A request to `POST /v1/query` carries exactly `batch` YPIR queries (8 by
default), each asking for one row. The server refuses any other count. A
client that needs fewer rows fills the round with queries for random rows. A
client that needs more sends more rounds. A transaction that spans k rows
uses k queries.

PIR hides which row each query asks for. The fixed count hides how many were
real. A client that wants to hide when it receives payments, not only which
ones, sends rounds on a timer, real or not (`cover_round` in `bf-client`).

## What the server learns

Per request: the time, the request size and the response size, which are
the same for every request against one database version. The server publishes
exactly this at `GET /v1/seen`.

It does not learn which rows were read, which transactions, how many were real
or whether any were.

## Limits

- Transactions mined before Ironwood activation are not covered. Wallets with
  older history still enhance it over lightwalletd, or use Tor for it.
- The newest blocks (inside the confirmation window) are not covered until the
  server catches up.
- The client's IP address reaches the server. Use Tor or an HTTP relay if the
  fact of using Zcash is itself sensitive.
- `GetStatus` requests, which wallets send for transactions they broadcast
  themselves, still go to lightwalletd. The server saw those transactions at
  broadcast anyway.
