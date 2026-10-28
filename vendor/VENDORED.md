# Vendored code

`zcash-devtool/` is https://github.com/zcash/zcash-devtool at commit
`0b3436bb8d235c67cd36b800cb45a7b1b489c415` (MIT OR Apache-2.0), with one change: `wallet enhance` gains
`--blindfetch <url>`, which fetches received transactions over PIR instead of
by txid. The change alone is in `patches/zcash-devtool-blindfetch.patch`, ready to
apply upstream.
