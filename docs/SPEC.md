# Blindfetch protocol, draft 1

Status: draft, written for ZECATHON 2026. The layout loosely follows a ZIP.

## Problem

A light wallet finds its payments by trial-decrypting compact blocks
(ZIP 307). Compact blocks carry only the first 52 bytes of each note
ciphertext. That is enough to find a note and read its value, and too little
for its memo. To read the memo and record the full transaction, the wallet
asks lightwalletd for the transaction by txid (`GetTransaction`). In
librustzcash this is `TransactionDataRequest::Enhancement`.

Each of those requests names a transaction the wallet received. Whoever runs
the server, or can read its logs, learns which transactions belong to the
client and when it came online to fetch them. Tor hides the address, but the
requests in one session still belong together.

## Overview

A Blindfetch server holds every transaction with a shielded output mined since
Ironwood activation, with its authorizing data zeroed. It cuts them into
fixed-size rows and answers private information retrieval (PIR) queries over
those rows. The wallet already knows where each of its transactions was mined
(height and position in the block) from compact block scanning. It looks the
position up in a public index, which it downloads in full, and fetches the
rows over PIR in rounds with a fixed number of queries. Then it checks every
row against a hash in the index and recomputes the txid of each transaction it
wanted.

The PIR scheme is YPIR in SimplePIR mode, through the `valar-ypir` crate. ZODL
uses the same crate for nullifier lookups in shielded voting, and its authors
had it audited by Zellic.

## Why stripped transactions verify

ZIP 244 splits a v5 transaction into effecting data, which the txid commits
to, and authorizing data, which it does not. ZIP 229 keeps this split for v6
and moves anchors into authorizing data. Proofs and signatures are authorizing
data, and so are transparent script signatures. Note commitments, ephemeral
keys, `encCiphertext` (which carries the memo) and `outCiphertext` are effecting
data.

The indexer replaces these authorizing fields with zeros of the same length:

- Orchard and Ironwood proofs, spend authorization signatures and binding
  signatures
- Sapling spend and output proofs, spend authorization signatures and binding
  signatures
- transparent script signatures

The transaction parser requires canonical proof lengths, so the zeros stay in
place and a run-length code collapses them (`bf_core::zeros`). Encrypted notes
do not compress, so this code only touches zero runs.

The result has the same txid as the mined transaction. A wallet that parses it
and recomputes the txid has a check on every note, memo and value in it. v4
transactions commit to their Sapling proofs, so they are stored unchanged.

Stripped transactions are not valid on chain and must never be broadcast.

## Database

1. Start at the Ironwood activation height (mainnet 3,428,143; testnet
   4,134,000).
2. For each block in order, take every transaction whose compact form has at
   least one Sapling output, Orchard action or Ironwood action.
3. Zero the authorizing data as above and check that the txid is unchanged.
   A transaction the indexer cannot parse is skipped, and wallets see it as not
   covered.
4. Append the encoding to a byte stream in (height, position) order.
5. Cut the stream into rows of 32 KiB. Pad with zero rows to at least 2,048
   rows and up to a power of two (YPIR pads to a power of two internally).

The indexer checks that each block's `prev_hash` matches the block before it.
If it does not, it rewinds 100 blocks and indexes them again. The server stays
10 blocks behind the tip by default.

## Index

For each block with entries, the index lists each transaction's position in
the block and its stored length. Offsets are running sums, so an entry costs
about four bytes. After the entries come the row count and a 16-byte BLAKE2b
hash of every row. The encoding is in `crates/bf-core/src/index.rs`.

The last height in the index is the database version. Queries carry it in an
`x-db-version` header. A server that has moved on answers 409, and the client
downloads the new index.

Every client downloads the whole index, so downloading it reveals nothing
except that the client uses Blindfetch. The index is a function of the chain
and the start height, so two servers built from the same heights publish the
same index. `Blindfetch::compare_with` checks this before any query.

## Queries

A request to `POST /v1/query` carries exactly `batch` YPIR queries (4 by
default), each asking for one row. The server refuses any other count, and
clients refuse servers that ask for fewer than 4. A client that needs fewer
rows fills the round with queries for random rows and shuffles the real ones
among them. A fetch is at least two rounds. A transaction that spans k rows
uses k queries.

The client decodes every answer, dummies included, and checks each row against
its hash in the index. Any mismatch, and any transaction that fails its txid
check, produces the same error, reported only after all planned rounds have
been sent. The wallet then stops fetching real rows from that server. On a
fixed schedule it keeps sending dummy-only ticks to it, so the server cannot
tell which wallet it caught. Before querying, a client can compare the index
digest with a second server (`--compare-with`).

A wallet fetches in ticks (`fetch_tick`) of exactly `rounds` rounds.
Transactions that do not fit wait for the next tick. zcash-devtool ticks on a
timer by default (every 300 seconds, one round per tick), so it sends the same
traffic at the same times whether it received nothing or many payments.
`--once` is an explicit opt-out that reveals when the wallet fetched.

## What the server learns

Per request: the time, the request size and the response size. These are the
same for every request against one database version. The server publishes
exactly this at `GET /v1/seen`, with times rounded to the minute.

It does not learn which rows were read, which transactions, how many were real
or whether any were. With a fixed schedule it also does not learn when payments
arrive.

## Limits

- Transactions mined before Ironwood activation are not covered.
- The newest blocks are covered once the server indexes them (10 blocks behind
  the tip, then a rebuild every 20 blocks).
- The client's IP address reaches the server. Use Tor or a relay if using Zcash
  at all is sensitive.
- `GetStatus` requests, which wallets send for transactions they broadcast
  themselves, still go to lightwalletd. It saw those transactions at broadcast.
- A malicious server can refuse service. With one server and no comparison it
  can tamper with a single transaction and its row hash; see the threat model
  for what that can and cannot reveal.
