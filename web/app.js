import init, { Session } from './pkg/bf_wasm.js';

// On the deployed site the servers sit behind the same origin. Locally they
// run on their own ports.
const LOCAL = ['localhost', '127.0.0.1'].includes(location.hostname);
const SERVERS = LOCAL
  ? { main: 'http://127.0.0.1:8484', test: 'http://127.0.0.1:8485' }
  : { main: '/main', test: '/test' };
const REPO = 'https://github.com/YASASWIP012/blindfetch';

const $ = (id) => document.getElementById(id);
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
    const info = await (await fetch(`${base()}/v1/info`)).json();
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
    ].map(([k, v]) => `<div><dt>${k}</dt><dd>${v}</dd></div>`).join('');
    renderRecent();
    $('go').disabled = false;
  } catch (e) {
    $('status').textContent = `Could not reach the ${net === 'main' ? 'mainnet' : 'testnet'} server: ${e.message || e}`;
  }
  pollSeen();
}

function renderRecent() {
  const list = session.recent(6);
  $('recent').innerHTML = '<span class="muted">Recent:</span> ' + list.map((e) =>
    `<button class="chip" data-h="${e.height}" data-i="${e.tx_index}">${fmtNum(e.height)} · #${e.tx_index} · ${fmtBytes(e.len)}</button>`
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
      <p><strong>txid</strong> <code class="wrapcode">${tx.txid}</code></p>
      <p>${check}</p>
      <p class="muted">${tx.version}, ${fmtBytes(tx.stripped_bytes)} without proofs. ${tx.ironwood_actions} Ironwood action(s), ${tx.orchard_actions} Orchard, ${tx.sapling_outputs} Sapling output(s), ${tx.transparent_inputs} transparent in, ${tx.transparent_outputs} out.</p>
      ${tx.ironwood_ciphertexts.length ? `<p class="muted">Encrypted notes, which a wallet would now decrypt for the memo:</p><pre class="log">${tx.ironwood_ciphertexts.map((c, k) => `action ${k}: ${c}…`).join('\n')}</pre>` : ''}`;
    $('stock').textContent =
      `rpc GetTransaction(TxFilter)\n  hash: ${tx.txid}\n\nThe server now knows this wallet\nreceived ${tx.txid.slice(0, 16)}…`;
    pollSeen(true);
  } catch (e) {
    out.innerHTML = `<p class="bad">${e.message || e}</p>`;
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
        ? `${t}  POST /v1/query  ${r.queries} queries  in ${fmtBytes(r.bytes_in)}  out ${fmtBytes(r.bytes_out)}`
        : `${t}  GET  /v1/index  out ${fmtBytes(r.bytes_out)}`;
    });
    $('seen').textContent = lines.length ? lines.join('\n') : 'No requests yet.';
  } catch {
    $('seen').textContent = 'Server unreachable.';
  }
  if (!once) seenTimer = setTimeout(pollSeen, 5000);
}

for (const b of document.querySelectorAll('.seg button')) {
  b.onclick = () => {
    for (const o of document.querySelectorAll('.seg button')) o.classList.toggle('on', o === b);
    net = b.dataset.net;
    connect();
  };
}
$('go').onclick = fetchPrivately;
$('repo').href = REPO;

await init();
connect();
