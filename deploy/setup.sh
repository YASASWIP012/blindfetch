#!/usr/bin/env bash
# Sets up a Blindfetch server on a fresh Ubuntu 24.04 box.
# Usage: deploy/setup.sh <host-ip> [ssh-key]
set -euo pipefail

HOST_IP="$1"
KEY="${2:-$HOME/.ssh/id_ed25519_blindfetch}"
SSH="ssh -i $KEY -o StrictHostKeyChecking=accept-new ubuntu@$HOST_IP"
DOMAIN="$(echo "$HOST_IP" | tr '.' '-').sslip.io"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

echo "Copying source to $HOST_IP"
rsync -az --delete -e "ssh -i $KEY" \
  --exclude target --exclude data --exclude web/node_modules --exclude 'vendor/*/target' \
  "$ROOT/" "ubuntu@$HOST_IP:/home/ubuntu/blindfetch/"

$SSH bash -s -- "$DOMAIN" <<'REMOTE'
set -euo pipefail
DOMAIN="$1"
sudo apt-get update -qq
sudo apt-get install -y -qq build-essential pkg-config libssl-dev protobuf-compiler debian-keyring debian-archive-keyring apt-transport-https curl rsync

if ! command -v caddy >/dev/null; then
  curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' | sudo gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg
  curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' | sudo tee /etc/apt/sources.list.d/caddy-stable.list >/dev/null
  sudo apt-get update -qq && sudo apt-get install -y -qq caddy
fi

if [ ! -x "$HOME/.cargo/bin/cargo" ]; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
fi
. "$HOME/.cargo/env"

cd ~/blindfetch
cargo build --release -p bf-indexer -p bf-server

# Oracle's Ubuntu images ship an iptables policy that drops 80/443.
sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 80 -j ACCEPT || true
sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 443 -j ACCEPT || true
sudo sh -c 'iptables-save > /etc/iptables/rules.v4' 2>/dev/null || true

sed "s/__HOST__/$DOMAIN/" deploy/Caddyfile.tmpl | sudo tee /etc/caddy/Caddyfile >/dev/null
sudo systemctl reload caddy || sudo systemctl restart caddy

sudo cp deploy/blindfetch@.service /etc/systemd/system/
sudo systemctl daemon-reload
for unit in indexer-main indexer-test; do sudo systemctl enable --now "blindfetch@$unit"; done
echo "Indexers started. Start the servers once data/<net>/index.bin exists:"
echo "  sudo systemctl enable --now blindfetch@server-main blindfetch@server-test"
echo "https://$DOMAIN"
REMOTE
