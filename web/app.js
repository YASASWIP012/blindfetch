import init, { Session } from './pkg/bf_wasm.js';

// On the deployed site the servers sit behind the same origin. Locally they
// run on their own ports.
const LOCAL = ['localhost', '127.0.0.1'].includes(location.hostname);
const SERVERS = LOCAL
  ? { main: 'http://127.0.0.1:8484', test: 'http://127.0.0.1:8485' }
  : { main: '/main', test: '/test' };

// If the same-origin proxy is down, server.json names the server's own
// address and the page talks to it directly.
async function getInfo() {
  try {
    const r = await fetch(`${base()}/v1/info`);
    if (r.ok) return await r.json();
  } catch (_) { /* try the direct address */ }
  if (LOCAL) throw new Error('no server on this machine');
  const direct = (await (await fetch('server.json', { cache: 'no-store' })).json())[net];
  if (!direct || direct === SERVERS[net]) throw new Error('the server is not answering');
  SERVERS[net] = direct;
  return (await fetch(`${base()}/v1/info`)).json();
}

const $ = (id) => document.getElementById(id);
// Everything shown comes from the server or the chain; escape it.
const esc = (v) => String(v).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
let net = 'main';
let session = null;
let seenTimer = null;
let batch = 4;

const fmtBytes = (n) => n >= 1048576 ? `${(n / 1048576).toFixed(1)} MiB` : n >= 1024 ? `${(n / 1024).toFixed(0)} KiB` : `${n} B`;
const fmtNum = (n) => n.toLocaleString('en-US');
const base = () => SERVERS[net];

async function connect() {
  session = null;
  $('go').disabled = true;
  $('result').hidden = true;
  $('stock').textContent = 'Fetch a transaction to see it.';
  $('status').textContent = 'Downloading the index…';
  try {
    const t0 = performance.now();
    const info = await getInfo();
    const indexBuf = new Uint8Array(await (await fetch(`${base()}/v1/index`)).arrayBuffer());
    session = new Session(JSON.stringify(info), indexBuf);
    batch = info.batch;
    const ms = Math.round(performance.now() - t0);
    $('status').textContent = `Index downloaded in ${ms} ms. Every visitor downloads all of it, so this told the server nothing.`;
    $('stats').innerHTML = [
      ['Transactions', fmtNum(info.transactions)],
      ['Heights', `${fmtNum(info.first_height)} to ${fmtNum(info.last_height)}`],
      ['Rows', `${fmtNum(info.num_rows)} × ${fmtBytes(info.row_bytes)}`],
      ['Index', fmtBytes(indexBuf.length)],
      ['Queries per round', info.batch],
    ].map(([k, v]) => `<div><dt>${esc(k)}</dt><dd>${esc(v)}</dd></div>`).join('');
    renderRecent();
    $('go').disabled = false;
    // ?auto fetches the newest transaction straight away, for links that
    // should show the whole round trip without a click.
    if (new URLSearchParams(location.search).has('auto')) fetchPrivately();
  } catch (e) {
    $('status').textContent = `Could not reach the server: ${e.message || e}. It runs on a laptop and may be asleep; the README shows how to run your own.`;
  }
  pollSeen();
}

function renderRecent() {
  const list = session.recent(6);
  $('recent').innerHTML = '<span class="label">Recent transactions</span>' + list.map((e) =>
    `<button class="chip" data-h="${esc(e.height)}" data-i="${esc(e.tx_index)}">${esc(fmtNum(e.height))} · #${esc(e.tx_index)} · ${esc(fmtBytes(e.len))}</button>`
  ).join('');
  for (const b of $('recent').querySelectorAll('button')) {
    b.onclick = () => { $('h').value = b.dataset.h; $('i').value = b.dataset.i; $('txid').value = ''; };
  }
  if (list.length && !$('h').value) { $('h').value = list[0].height; $('i').value = list[0].tx_index; }
}

async function fetchPrivately() {
  const h = parseInt($('h').value, 10);
  const i = parseInt($('i').value, 10);
  if (!Number.isFinite(h) || !Number.isFinite(i)) return;
  $('go').disabled = true;
  const out = $('result');
  out.hidden = false;
  out.innerHTML = `<p class="muted">Building ${batch} queries in your browser…</p>`;
  try {
    const t0 = performance.now();
    const body = session.buildRound(h, i);
    const t1 = performance.now();
    const resp = await fetch(`${base()}/v1/query`, {
      method: 'POST',
      headers: { 'x-db-version': String(session.dbVersion()) },
      body,
    });
    if (resp.status === 409) { await connect(); throw new Error('the server rebuilt its database; try again'); }
    if (!resp.ok) throw new Error(await resp.text());
    const answer = new Uint8Array(await resp.arrayBuffer());
    const t2 = performance.now();
    const tx = session.openRound(answer, $('txid').value || undefined);
    const t3 = performance.now();

    const check = tx.matches_expected === null || tx.matches_expected === undefined
      ? '<span class="muted">No expected txid given. Paste one to check it.</span>'
      : tx.matches_expected ? '<span class="ok">Matches the txid you gave.</span>' : '<span class="bad">Does not match the txid you gave.</span>';
    out.innerHTML = `
      <dl class="stats">
        <div><dt>Sent</dt><dd>${batch} queries, ${fmtBytes(body.length)}</dd></div>
        <div><dt>Received</dt><dd>${fmtBytes(answer.length)}</dd></div>
        <div><dt>Queries built</dt><dd>${Math.round(t1 - t0)} ms</dd></div>
        <div><dt>Round trip</dt><dd>${Math.round(t2 - t1)} ms</dd></div>
        <div><dt>Opened</dt><dd>${Math.round(t3 - t2)} ms</dd></div>
      </dl>
      <p><strong>txid</strong> <code class="wrapcode">${esc(tx.txid)}</code></p>
      <p>${check}</p>
      <p class="muted">${esc(tx.version)}, ${esc(fmtBytes(tx.stripped_bytes))} stored. ${esc(tx.ironwood_actions)} Ironwood action(s), ${esc(tx.orchard_actions)} Orchard, ${esc(tx.sapling_outputs)} Sapling output(s), ${esc(tx.transparent_inputs)} transparent in, ${esc(tx.transparent_outputs)} out.</p>
      ${tx.ironwood_ciphertexts.length ? `<p class="muted">Encrypted notes, which a wallet would now decrypt for the memo:</p><pre class="log">${tx.ironwood_ciphertexts.map((c, k) => `action ${k}: ${esc(c)}…`).join('\n')}</pre>` : ''}`;
    $('stock').textContent =
      `rpc GetTransaction(TxFilter)\n  hash: ${tx.txid}\n\nThe server now knows this wallet\nreceived ${tx.txid.slice(0, 16)}…`;
    pollSeen(true);
  } catch (e) {
    out.textContent = '';
    const msg = document.createElement('p');
    msg.className = 'bad';
    msg.textContent = String(e.message || e);
    out.appendChild(msg);
  } finally {
    $('go').disabled = false;
  }
}

async function pollSeen(once) {
  clearTimeout(seenTimer);
  try {
    const rows = await (await fetch(`${base()}/v1/seen`)).json();
    const lines = rows.slice(0, 12).map((r) => {
      const t = new Date(r.at * 1000).toISOString().slice(11, 16);
      return r.kind === 'query'
        ? `${t}  query  ${r.queries} queries  in ${fmtBytes(r.bytes_in)}  out ${fmtBytes(r.bytes_out)}`
        : `${t}  index  out ${fmtBytes(r.bytes_out)}`;
    });
    $('seen').textContent = lines.length ? lines.join('\n') : 'No requests yet.';
  } catch {
    $('seen').textContent = 'Server unreachable.';
  }
  if (!once) seenTimer = setTimeout(pollSeen, 5000);
}

$('go').onclick = fetchPrivately;

await init();
connect();
