# Deploying a Blindfetch server

Tested on Ubuntu 24.04, x86_64 (Azure B2as_v2: 2 vCPU, 8 GB), which serves the
full mainnet database with `--low-memory` (see `server-main.env`).

    ./deploy/setup.sh <host-ip> [user]

It installs Rust and Caddy, copies `data/main` if you have one (otherwise the
indexer builds it from lightwalletd), builds `bf-indexer` and `bf-server` on
the box, starts the mainnet indexer and server under systemd, and puts Caddy
in front with a certificate for `<ip>.sslip.io`:

- `https://<ip>.sslip.io/main/v1/...` mainnet

The indexer rebuilds the database every 200 blocks. With `--low-memory` the
server then exits and systemd starts a fresh one on the new files, so the
server is down for a few minutes about every four hours.

Caddy is configured without access logs, so client addresses are not written
anywhere.
