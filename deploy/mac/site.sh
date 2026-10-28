#!/usr/bin/env bash
# Builds the demo site and deploys it to Vercel. Vercel proxies /main and
# /test to the servers, so the public addresses never change even when the
# tunnels behind them do.
# Usage: deploy/mac/site.sh <mainnet-server-url> [testnet-server-url]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MAIN="$1"
TEST="${2:-$1}"
SITE="$ROOT/.site/blindfetch"
rm -rf "$SITE" && mkdir -p "$SITE/pkg"
cp "$ROOT"/web/{index.html,app.js,style.css} "$SITE/"
cp "$ROOT"/web/pkg/bf_wasm.js "$ROOT"/web/pkg/bf_wasm_bg.wasm "$SITE/pkg/"
cat > "$SITE/vercel.json" <<JSON
{
  "rewrites": [
    { "source": "/main/:path*", "destination": "$MAIN/:path*" }
  ],
  "headers": [
    { "source": "/pkg/(.*).wasm", "headers": [{ "key": "Content-Type", "value": "application/wasm" }] },
    { "source": "/(main|test)/(.*)", "headers": [{ "key": "Cache-Control", "value": "no-store" }] }
  ]
}
JSON
if [ -d "$ROOT/.site/.vercel" ]; then cp -R "$ROOT/.site/.vercel" "$SITE/"; fi
cd "$SITE"
npx -y vercel@latest deploy --prod --yes 2>&1 | tail -3
mkdir -p "$ROOT/.site" && cp -R "$SITE/.vercel" "$ROOT/.site/" 2>/dev/null || true
