#!/usr/bin/env bash
# A real mainnet wallet reads its memos two ways.
#
# The wallet is watch-only, restored from a viewing key that its owners
# published for public audit (the Tacit project's mainnet auction vault, lot 2,
# test-vectors/mainnet-lot-2.json in their repository). It received two deposits
# with memos at heights 3,529,984 and 3,529,988.
#
# Needs a Blindfetch server covering those heights, e.g.
#   bf-indexer --network main --data data-demo --from 3529900 sync --to 3530800
#   bf-indexer --network main --data data-demo --from 3529900 build
#   bf-server  --data data-demo/main --listen 127.0.0.1:8486
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DEVTOOL="$ROOT/target/release/zcash-devtool"
SERVER="${BLINDFETCH:-http://127.0.0.1:8486}"
UFVK="uview1qlg45kytg7erw0c3g4hj69splxkvuwdg8eyvzaw6kan6vrg8vfzj8wru65actlv62q00cfvltpyejhvnzm83hspal7uu5j99nechldazpx3upwt6wa9ya4tevwekntd5g53f3s44q3sln0pf2q4vuuqr253vtmv6vrlmrp6hmlntun2x5n4mecsssqvgp"
W="$(mktemp -d)"
trap 'rm -rf "$W"' EXIT

echo "Restoring a watch-only wallet from the public viewing key and scanning compact blocks"
"$DEVTOOL" wallet -w "$W/a" init-fvk --name public-vault --fvk "$UFVK" --birthday 3529970 -s zecrocks >/dev/null 2>&1
"$DEVTOOL" wallet -w "$W/a" sync -s zecrocks >/dev/null 2>&1
cp -R "$W/a" "$W/b"

echo
echo "1. Stock: ask lightwalletd for each transaction by txid"
"$DEVTOOL" wallet -w "$W/a" enhance -s zecrocks 2>/dev/null

echo
echo "2. Blindfetch: the same transactions over PIR (one fixed fetch; a real wallet runs on a timer)"
"$DEVTOOL" wallet -w "$W/b" enhance -s zecrocks --blindfetch "$SERVER" --once 2>/dev/null

echo
memos() { "$DEVTOOL" wallet -w "$1" list-tx 2>/dev/null | grep -E '^[0-9a-f]{64}|Memo'; }
if diff <(memos "$W/a") <(memos "$W/b") >/dev/null; then
  echo "Both wallets now hold the same transactions and memos:"
  memos "$W/b"
else
  echo "The two wallets differ"; exit 1
fi
