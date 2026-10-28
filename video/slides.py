# -*- coding: utf-8 -*-
"""Renders the video's frames as 1920x1080 PNGs with headless Chrome.
Terminal text is copied from real runs (demo/wallet-demo.sh and the
enhance --every test of 28 Oct 2026)."""
import pathlib, subprocess, html

CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
OUT = pathlib.Path("frames")
OUT.mkdir(exist_ok=True)

CSS = """
*{box-sizing:border-box}
body{margin:0;width:1920px;height:1080px;background:#0f1013;color:#ecebe6;
  font-family:Inter,system-ui,-apple-system,sans-serif;overflow:hidden}
.pad{padding:96px 120px}
.kicker{font:600 22px/1 ui-monospace,Menlo,monospace;letter-spacing:.14em;text-transform:uppercase;color:#e39a45}
h1{font-size:132px;letter-spacing:-.03em;margin:28px 0 0}
h2{font-size:58px;letter-spacing:-.02em;margin:22px 0 36px;font-weight:700;max-width:1600px;line-height:1.12}
p.lede{font-size:36px;color:#b9bcc4;max-width:1500px;line-height:1.35;margin:0}
.term{background:#16181d;border:1px solid #2a2d33;border-radius:18px;padding:34px 40px;
  font:27px/1.55 ui-monospace,"JetBrains Mono",Menlo,monospace;white-space:pre;color:#cfd2d8}
.term .dim{color:#7d828c}.term .hot{color:#ff7a6b}.term .ok{color:#59d18f}.term .acc{color:#e39a45}
.row{display:flex;gap:28px;align-items:stretch}
.box{flex:1;background:#16181d;border:1px solid #2a2d33;border-radius:18px;padding:30px 34px}
.box h3{margin:0 0 10px;font-size:30px}.box p{margin:0;font-size:26px;color:#b9bcc4;line-height:1.4}
.big{font:700 64px/1 ui-monospace,Menlo,monospace;color:#ecebe6;margin:8px 0 6px}
.arrow{align-self:center;font-size:54px;color:#e39a45}
.foot{position:absolute;left:120px;bottom:70px;font:24px ui-monospace,Menlo,monospace;color:#7d828c}
"""

def page(body):
    return f"<!doctype html><html><head><meta charset='utf-8'><style>{CSS}</style></head><body>{body}</body></html>"

def render(name, body):
    f = OUT / f"{name}.html"
    f.write_text(page(body))
    subprocess.run([CHROME, "--headless=new", "--disable-gpu", "--hide-scrollbars",
                    "--window-size=1920,1080", f"--screenshot={OUT / (name + '.png')}",
                    f"file://{f.resolve()}"], check=True, capture_output=True)
    print("rendered", name)

def term(text):
    return f"<div class='term'>{text}</div>"

E = html.escape

render("title", """<div class='pad' style='padding-top:300px'>
<div class='kicker'>ZECATHON 2026 · Core &amp; Tooling</div>
<h1>Blindfetch</h1>
<p class='lede' style='margin-top:30px'>Zcash light wallets read their memos without telling the server which transactions are theirs.</p>
</div>""")

T1 = term(
"<span class='dim'>$ zcash-devtool wallet -w vault enhance</span>\n"
"<span class='hot'>lightwalletd GetTransaction 967ec7938963c93c35bbc4813c76fc63ea0db6a287f8ee99326eee763648ed32</span>\n"
"<span class='hot'>lightwalletd GetTransaction 8d9f0e7040eda2052a8cde555e4c2595ae8decff9c17be47f080da7525bef49c</span>\n"
"<span class='hot'>lightwalletd GetTransaction f572a5d07137c72211eafaa348d4d4a922cc2f1c8cd925c21ba6f434077afadf</span>")
render("s1", f"""<div class='pad'>
<div class='kicker'>Today, on mainnet</div>
<h2>To read memos, the wallet names its own transactions</h2>
{T1}
<p class='lede' style='margin-top:34px'>A watch-only wallet restored from a viewing key its owners published for audit.<br>Three requests, three of its transactions, one IP address.</p>
</div>""")

render("s2", """<div class='pad'>
<div class='kicker'>How it works</div>
<h2>Every shielded transaction since Ironwood, served over PIR</h2>
<div class='row'>
 <div class='box'><h3>Index the chain</h3><p>Every transaction with a shielded output since block 3,428,143.</p></div>
 <div class='arrow'>→</div>
 <div class='box'><h3>Zero what the txid skips</h3><div class='big'>120 → 23 MB</div><p>Proofs and signatures are outside the txid (ZIP 244, ZIP 229). Same txid after.</p></div>
 <div class='arrow'>→</div>
 <div class='box'><h3>32 KiB rows over YPIR</h3><p>The PIR scheme ZODL uses for shielded voting, from the Zellic-audited <code>valar-ypir</code>.</p></div>
</div>
<p class='lede' style='margin-top:48px'>The wallet downloads the whole index, finds its rows from where its transactions were mined, and asks for them in fixed rounds of four encrypted queries.</p>
</div>""")

T2 = term(
"<span class='dim'>$ zcash-devtool wallet -w vault enhance --blindfetch $SERVER</span>\n"
"Blindfetch: 12969 transactions, heights 3529900..=3530800, 2048 rows, 4 queries per round\n"
"Fetched 3 of 3 transactions in 2 round(s) of 4 queries: 4.3 MiB up, 0.9 MiB down, 1467 ms\n"
"<span class='ok'>lightwalletd was not asked for any of them.</span>\n\n"
"<span class='dim'>Both wallets now hold the same transactions and memos:</span>\n"
"967ec793…ed32   <span class='acc'>Memo::Future(f5)</span>\n"
"f572a5d0…afadf  <span class='acc'>Memo::Future(f5)</span>\n"
"8d9f0e70…f49c   Memo::Empty ×4")
render("s3", f"""<div class='pad'>
<div class='kicker'>Same wallet, through Blindfetch</div>
<h2>Same memos. lightwalletd is never asked.</h2>
{T2}
</div>""")

render("s5", f"""<div class='pad'>
<div class='kicker'>Fixed schedule · fail closed</div>
<h2>A tick with three payments looks like a tick with none</h2>
<div class='row'>
 <div class='box'><h3>Tick 1: fetched 3 transactions</h3>
 <div class='term' style='border:0;padding:16px 0;font-size:24px'>query  4 queries  in 2,228,276  out 491,540
query  4 queries  in 2,228,276  out 491,540</div></div>
 <div class='box'><h3>Tick 2: fetched nothing</h3>
 <div class='term' style='border:0;padding:16px 0;font-size:24px'>query  4 queries  in 2,228,276  out 491,540
query  4 queries  in 2,228,276  out 491,540</div></div>
</div>
<p class='lede' style='margin-top:44px'>The server's own log for both ticks. Every answer, dummies included, is checked against a hash of its row in the index. A server that fails once is locked out for good.</p>
</div>""")

render("s6", """<div class='pad' style='padding-top:250px'>
<div class='kicker'>Open source · mainnet · audited PIR</div>
<h1 style='font-size:110px'>Blindfetch</h1>
<p class='lede' style='margin-top:30px'>Try it: blindfetch.vercel.app<br>Code: github.com/YASASWIP012/blindfetch</p>
</div>""")
