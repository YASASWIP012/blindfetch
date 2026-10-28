# Deploying a Blindfetch server

Tested on Ubuntu 24.04, arm64 (Oracle Cloud Always Free, Ampere A1).

    ./deploy/setup.sh <host-ip>

It installs Rust and Caddy, builds `bf-indexer` and `bf-server` on the box,
starts one indexer and one server per network under systemd, and puts Caddy
in front with a certificate for `<ip>.sslip.io`:

- `https://<ip>.sslip.io/main/v1/...` mainnet
- `https://<ip>.sslip.io/test/v1/...` testnet

Caddy is configured without access logs, so client addresses are not written
anywhere.
