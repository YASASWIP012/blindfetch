#!/usr/bin/env bash
# Runs Blindfetch on this Mac and keeps it reachable:
#   - one indexer per network, following the chain
#   - one server per network, on 8484 (mainnet) and 8485 (testnet)
#   - one Cloudflare quick tunnel per server
#   - redeploys the Vercel site whenever a tunnel address changes
# and keeps the Mac awake while it runs. Logs go to ~/Library/Logs/blindfetch.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="$ROOT/target/release"
LOGS="$HOME/Library/Logs/blindfetch"
STATE="$ROOT/.site/tunnels"
mkdir -p "$LOGS" "$STATE"
export PATH="/opt/homebrew/bin:$PATH"

# Keep the Mac awake for as long as this script runs.
caffeinate -dimsu -w $$ &

port() { [ "$1" = main ] && echo 8484 || echo 8485; }

running() { pgrep -f "$1" >/dev/null; }

start_indexer() {
  local net=$1
  running "bf-indexer --network $net --data $ROOT/data sync" && return
  nohup "$BIN/bf-indexer" --network "$net" --data "$ROOT/data" sync --follow --rebuild-every 20 \
    >>"$LOGS/indexer-$net.log" 2>&1 &
}

start_server() {
  local net=$1
  [ -f "$ROOT/data/$net/index.bin" ] || return
  running "bf-server --data $ROOT/data/$net" && return
  nohup "$BIN/bf-server" --data "$ROOT/data/$net" --listen "127.0.0.1:$(port $net)" \
    >>"$LOGS/server-$net.log" 2>&1 &
}

# Starts a tunnel if none is running and prints its public address.
tunnel() {
  local net=$1 log="$LOGS/tunnel-$net.log"
  if ! running "cloudflared tunnel --no-autoupdate --url http://127.0.0.1:$(port $net)"; then
    : >"$log"
    nohup cloudflared tunnel --no-autoupdate --url "http://127.0.0.1:$(port $net)" >>"$log" 2>&1 &
    for _ in $(seq 60); do grep -q 'trycloudflare.com' "$log" && break; sleep 1; done
  fi
  grep -o 'https://[a-z0-9-]*\.trycloudflare\.com' "$log" | tail -1
}

while true; do
  for net in main test; do start_indexer "$net"; start_server "$net"; done
  main_url=$(tunnel main)
  test_url=$( [ -f "$ROOT/data/test/index.bin" ] && tunnel test || echo "$main_url")
  current="$main_url $test_url"
  if [ -n "$main_url" ] && [ "$current" != "$(cat "$STATE/current" 2>/dev/null)" ]; then
    echo "$(date -u) tunnels: $current" >>"$LOGS/site.log"
    if "$ROOT/deploy/mac/site.sh" "$main_url" "$test_url" >>"$LOGS/site.log" 2>&1; then
      echo "$current" >"$STATE/current"
    fi
  fi
  sleep 30
done
