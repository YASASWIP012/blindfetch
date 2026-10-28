#!/usr/bin/env bash
# Sets up a Blindfetch server on a fresh Ubuntu 24.04 box.
# Usage: deploy/setup.sh <host-ip> [user] [ssh-key]
# SKIP_DATA=1 copies and builds only, for when data/main is copied separately.
# Copies an existing data/main if there is one, so the box serves at once
# and the indexer only has to follow new blocks.
set -euo pipefail

HOST_IP="$1"
U="${2:-azureuser}"
KEY="${3:-$HOME/.ssh/id_ed25519_blindfetch}"
SSH="ssh -i $KEY -o StrictHostKeyChecking=accept-new $U@$HOST_IP"
DOMAIN="$(echo "$HOST_IP" | tr '.' '-').sslip.io"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "Copying source to $HOST_IP"
rsync -az --delete -e "ssh -i $KEY -o StrictHostKeyChecking=accept-new" \
  --exclude target --exclude '/data*' --exclude web/node_modules --exclude 'vendor/*/target' \
  --exclude video --exclude .site --exclude .git \
  "$ROOT/" "$U@$HOST_IP:/home/$U/blindfetch/"

if [ -z "${SKIP_DATA:-}" ] && [ -f "$ROOT/data/main/index.bin" ]; then
  echo "Copying data/main"
  rsync -az --partial -e "ssh -i $KEY" "$ROOT/data/main/" "$U@$HOST_IP:/home/$U/blindfetch/data/main/"
fi

$SSH bash -s -- "$DOMAIN" "$U" "${SKIP_DATA:-}" <<'REMOTE'
set -euo pipefail
DOMAIN="$1"; U="$2"; SKIP_DATA="$3"
sudo apt-get update -qq
sudo apt-get install -y -qq build-essential pkg-config libssl-dev protobuf-compiler debian-keyring debian-archive-keyring apt-transport-https curl rsync

if ! command -v caddy >/dev/null; then
  curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
  curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list >/dev/null
  sudo apt-get update -qq && sudo apt-get install -y -qq caddy
fi

# The server holds the whole database in memory; swap covers rebuilds.
if [ ! -f /swapfile ]; then
  sudo fallocate -l 4G /swapfile && sudo chmod 600 /swapfile && sudo mkswap /swapfile && sudo swapon /swapfile
  echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab >/dev/null
fi

if [ ! -x "$HOME/.cargo/bin/cargo" ]; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi
. "$HOME/.cargo/env"

cd ~/blindfetch
cargo build --release -p bf-indexer -p bf-server

sed "s/__HOST__/$DOMAIN/" deploy/Caddyfile.tmpl | sudo tee /etc/caddy/Caddyfile >/dev/null
sudo systemctl reload caddy || sudo systemctl restart caddy

sed "s#/home/ubuntu#/home/$U#g; s#^User=ubuntu#User=$U#" deploy/blindfetch@.service | sudo tee /etc/systemd/system/blindfetch@.service >/dev/null
sudo systemctl daemon-reload
# With SKIP_DATA the database is still being copied; start the services after.
if [ -n "$SKIP_DATA" ]; then echo "Services not started (SKIP_DATA)"; exit 0; fi
sudo systemctl enable --now blindfetch@indexer-main
if [ -f data/main/index.bin ]; then sudo systemctl enable --now blindfetch@server-main; fi
echo "https://$DOMAIN/main/v1/info"
REMOTE
