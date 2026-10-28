// Blindfetch screens, rebuilt from the real tools. Every line of terminal text and
// every number is copied from runs on 28 Oct 2026 (demo/wallet-demo.sh, bf-indexer
// build on blocks 3,529,900 to 3,530,800, the server's /v1/seen log).
import React from 'react';
import {random} from 'remotion';
import {EASE, ms, tween, clamp01, sp} from '../../lib/motion';
import {Check, PopIn, Cursor} from '../../components/UI';
import {Counter} from '../../components/Type';
import {ScreenProps} from '../types';

const INK = '#1D1D1F';
const MUTED = '#6E6E73';
const LINE = 'rgba(0,0,0,0.08)';
const RED = '#FF7A6B';
const GREEN = '#5FD08F';
const MONO = '"Geist Mono", ui-monospace, Menlo, monospace';

/** Lines that type in one by one from `start`, `gap` frames apart. */
const TermLines: React.FC<{f: number; start: number; gap: number; lines: Array<{t: string; c?: string}>; size?: number}> = ({
  f,
  start,
  gap,
  lines,
  size = 30,
}) => (
  <div style={{display: 'flex', flexDirection: 'column', gap: size * 0.42}}>
    {lines.map((l, i) => {
      const at = start + i * gap;
      const shown = f >= at;
      const chars = Math.floor(clamp01((f - at) / Math.max(1, ms(30, 420))) * l.t.length);
      return (
        <div key={i} style={{fontFamily: MONO, fontSize: size, color: l.c ?? '#E7E5DF', whiteSpace: 'pre', opacity: shown ? 1 : 0, minHeight: size * 1.2}}>
          {l.t.slice(0, Math.max(chars, l.c === 'dim' ? l.t.length : chars))}
        </div>
      );
    })}
  </div>
);

const short = (h: string) => `${h.slice(0, 8)}…${h.slice(-6)}`;
const TX = [
  '967ec7938963c93c35bbc4813c76fc63ea0db6a287f8ee99326eee763648ed32',
  '8d9f0e7040eda2052a8cde555e4c2595ae8decff9c17be47f080da7525bef49c',
  'f572a5d07137c72211eafaa348d4d4a922cc2f1c8cd925c21ba6f434077afadf',
];

/** Laptop (1440×900). mode 'stock': three GetTransaction lines name the wallet's
 *  payments. mode 'blindfetch': one round, nothing named, same memos. */
export const BfTerminal: React.FC<ScreenProps & {mode?: 'stock' | 'blindfetch'}> = ({f, ev, mode = 'stock'}) => {
  const trig = ev('trigger', 1);
  const react = ev('react', 2);
  const pay = ev('payoff', 3);
  const stock = mode === 'stock';
  const prompt = stock ? '$ zcash-devtool wallet enhance' : '$ … enhance --blindfetch $SERVER';
  const body: Array<{t: string; c?: string}> = stock
    ? TX.map((t) => ({t: `GetTransaction ${t.slice(0, 12)}…${t.slice(-6)}`, c: RED}))
    : [
        {t: 'Fetched 3 of 3 transactions'},
        {t: '1 round of 4 queries'},
        {t: '2.1 MiB up, 0.5 MiB down'},
        {t: 'lightwalletd was not asked.', c: GREEN},
      ];
  const tail = stock ? 'The server now has a list of this wallet’s payments.' : 'Same memos as the stock path.';
  return (
    <div style={{width: 1440, height: 900, background: '#16171B', position: 'relative', padding: '64px 76px', boxSizing: 'border-box'}}>
      <div style={{position: 'absolute', top: 0, left: 0, right: 0, height: 40, background: '#202126', display: 'flex', alignItems: 'center', gap: 9, paddingLeft: 18}}>
        {['#FF5F57', '#FEBC2E', '#28C840'].map((c) => <i key={c} style={{width: 13, height: 13, borderRadius: 7, background: c}} />)}
        <span style={{marginLeft: 'auto', marginRight: 'auto', fontFamily: 'Inter', fontSize: 22, color: '#8E8E96', transform: 'translateX(-40px)'}}>mainnet watch-only wallet</span>
      </div>
      <div style={{marginTop: 34}}>
        <div style={{fontFamily: MONO, fontSize: 44, color: '#8E8E96', opacity: f >= trig - 6 ? 1 : 0.35}}>{prompt}</div>
        <div style={{marginTop: 34}}>
          <TermLines f={f} start={react} gap={ms(30, 620)} lines={body} size={50} />
        </div>
        {!stock && (
          <div style={{marginTop: 40, display: 'flex', flexDirection: 'column', gap: 14}}>
            {[...TX.slice(0, 1), TX[2], TX[1]].map((t, i) => (
              <PopIn key={t} f={f} at={pay + i * ms(30, 140)}>
                <div style={{display: 'flex', gap: 36, fontFamily: MONO, fontSize: 42, color: '#E7E5DF'}}>
                  <span>{short(t)}</span>
                  <span style={{color: i < 2 ? '#E8A65C' : '#8E8E96'}}>{i < 2 ? 'Memo::Future(f5)' : 'Memo::Empty ×4'}</span>
                </div>
              </PopIn>
            ))}
          </div>
        )}
        <div style={{marginTop: stock ? 64 : 36, fontFamily: 'Inter', fontSize: 46, fontWeight: 650, color: stock ? '#FFB3A8' : '#B8F0CF', opacity: clamp01(tween(f, pay + ms(30, 300), pay + ms(30, 700)))}}>
          {tail}
        </div>
      </div>
    </div>
  );
};

/** Laptop (1440×900). The indexer zeroes what the txid does not cover: a size
 *  counter drops from 120.1 MB to 22.8 MB while each row keeps its txid. */
export const BfIndex: React.FC<ScreenProps> = ({f, ev}) => {
  const react = ev('react', 2);
  const pay = ev('payoff', 3);
  const rows = [
    ['Notes and memos', 'kept', 0.38],
    ['Proofs', 'zeroed', 0.44],
    ['Signatures', 'zeroed', 0.1],
  ] as const;
  const shrink = clamp01(tween(f, react, react + ms(30, 1600), 0, 1, EASE.premium));
  return (
    <div style={{width: 1440, height: 900, background: '#F5F5F7', fontFamily: 'Inter', color: INK, padding: '64px 72px', boxSizing: 'border-box'}}>
      <div style={{fontSize: 30, fontWeight: 600, letterSpacing: '.08em', textTransform: 'uppercase', color: MUTED}}>12,969 mainnet transactions</div>
      <div style={{display: 'flex', gap: 40, marginTop: 36}}>
        <div style={{flex: 1, background: '#fff', borderRadius: 26, boxShadow: `0 0 0 1px ${LINE}`, padding: 40}}>
          {rows.map(([name, what, share], i) => {
            const zeroed = what === 'zeroed';
            const w = zeroed ? share * (1 - shrink * 0.97) : share;
            return (
              <div key={name} style={{marginBottom: 44}}>
                <div style={{display: 'flex', justifyContent: 'space-between', fontSize: 40, fontWeight: 650}}>
                  <span>{name}</span>
                  <span style={{color: zeroed ? (shrink > 0.5 ? '#B3261E' : MUTED) : '#1F7A3D', fontFamily: MONO, fontSize: 32}}>{shrink > 0.5 || !zeroed ? what : ''}</span>
                </div>
                <div style={{marginTop: 14, height: 26, borderRadius: 13, background: 'rgba(0,0,0,0.05)'}}>
                  <div style={{height: 26, width: `${w * 100 * 2.1}%`, maxWidth: '100%', borderRadius: 9, background: zeroed ? '#C9C9CF' : INK}} />
                </div>
              </div>
            );
            void i;
          })}
        </div>
        <div style={{width: 520, display: 'flex', flexDirection: 'column', gap: 24}}>
          <div style={{background: '#fff', borderRadius: 26, boxShadow: `0 0 0 1px ${LINE}`, padding: 40}}>
            <div style={{fontSize: 32, color: MUTED}}>stored</div>
            <div style={{fontSize: 120, fontWeight: 750, letterSpacing: '-.045em', fontFamily: 'Inter', marginTop: 4, whiteSpace: 'nowrap'}}>
              <Counter f={f} from={120.1} to={22.8} f0={react} f1={react + ms(30, 1600)} decimals={1} suffix=" MB" />
            </div>
            <div style={{fontSize: 32, color: MUTED, marginTop: 6}}>from 120.1 MB mined</div>
          </div>
          <div style={{display: 'flex', alignItems: 'center', gap: 22, background: '#fff', borderRadius: 26, boxShadow: `0 0 0 1px ${LINE}`, padding: '30px 36px', opacity: clamp01(tween(f, pay, pay + ms(30, 300)))}}>
            <Check f={f} at={pay} size={64} bg="#1F7A3D" />
            <div style={{fontSize: 40, fontWeight: 700}}>Every txid unchanged</div>
          </div>
        </div>
      </div>
    </div>
  );
};

/** Laptop (1440×900). The database as rows; four queries light four rows; only
 *  the viewer sees which one was real. The server panel shows what it learns. */
export const BfRows: React.FC<ScreenProps> = ({f, ev, fps}) => {
  const trig = ev('trigger', 1);
  const react = ev('react', 2);
  const pay = ev('payoff', 3);
  const cols = 18;
  const rowsN = 12;
  const picks = [21, 88, 130, 191];
  const real = 130;
  const cell = 38;
  const gap = 8;
  return (
    <div style={{width: 1440, height: 900, background: '#F5F5F7', fontFamily: 'Inter', color: INK, padding: '64px 72px', boxSizing: 'border-box', display: 'flex', gap: 48}}>
      <div style={{flex: 1}}>
        <div style={{fontSize: 30, fontWeight: 600, letterSpacing: '.08em', textTransform: 'uppercase', color: MUTED}}>Rows of the database</div>
        <div style={{position: 'relative', marginTop: 30, display: 'grid', gridTemplateColumns: `repeat(${cols}, ${cell}px)`, gap}}>
          {Array.from({length: cols * rowsN}).map((_, i) => {
            const k = picks.indexOf(i);
            const lit = k >= 0 ? clamp01(tween(f, react + k * ms(fps, 120), react + k * ms(fps, 120) + ms(fps, 260))) : 0;
            const isReal = i === real && f >= pay;
            const base = 0.07 + 0.05 * random(`c${i}`);
            return (
              <div
                key={i}
                style={{
                  width: cell,
                  height: cell,
                  borderRadius: 5,
                  background: lit > 0 ? `rgba(29,29,31,${0.15 + 0.85 * lit})` : `rgba(0,0,0,${base})`,
                  boxShadow: isReal ? '0 0 0 5px #E8A65C' : 'none',
                }}
              />
            );
          })}
        </div>
        <div style={{marginTop: 30, fontSize: 36, fontWeight: 600, color: INK, opacity: clamp01(tween(f, pay, pay + ms(fps, 300)))}}>
          <span style={{display: 'inline-block', width: 26, height: 26, borderRadius: 6, boxShadow: '0 0 0 4px #E8A65C', marginRight: 18, verticalAlign: '-3px'}} />
          Only the wallet knows the real one.
        </div>
      </div>
      <div style={{width: 440, display: 'flex', flexDirection: 'column', gap: 18}}>
        <div style={{fontSize: 30, fontWeight: 600, letterSpacing: '.08em', textTransform: 'uppercase', color: MUTED}}>Server sees</div>
        {[0, 1, 2, 3].map((i) => (
          <PopIn key={i} f={f} at={trig + i * ms(fps, 110)}>
            <div style={{background: '#16171B', color: '#E7E5DF', borderRadius: 16, padding: '20px 24px', fontFamily: MONO, fontSize: 34}}>
              query · 532 KiB
            </div>
          </PopIn>
        ))}
        <div style={{marginTop: 8, background: '#fff', borderRadius: 20, boxShadow: `0 0 0 1px ${LINE}`, padding: 28, opacity: clamp01(tween(f, pay, pay + ms(fps, 300)))}}>
          <div style={{fontSize: 36, fontWeight: 700}}>Same size, every time</div>
          <div style={{fontSize: 30, color: MUTED, marginTop: 8}}>No row number, no txid.</div>
        </div>
      </div>
    </div>
  );
};

/** Browser (1440×854). The live page: click Fetch privately, the round goes out,
 *  the transaction comes back, the server log gets one line. */
export const BfPage: React.FC<ScreenProps> = ({f, ev, fps}) => {
  const trig = ev('trigger', 1);
  const react = ev('react', 2);
  const pay = ev('payoff', 3);
  const press = f >= trig && f < trig + 6 ? 0.97 : 1;
  const stat = (k: string, v: string, at: number) => (
    <div style={{borderTop: `1px solid ${LINE}`, paddingTop: 10, opacity: clamp01(tween(f, at, at + ms(fps, 260)))}}>
      <div style={{fontSize: 25, color: MUTED}}>{k}</div>
      <div style={{fontFamily: MONO, fontSize: 32, marginTop: 4}}>{v}</div>
    </div>
  );
  return (
    <div style={{width: 1440, height: 854, background: '#F5F5F7', fontFamily: 'Inter', color: INK, padding: '48px 64px', boxSizing: 'border-box', position: 'relative'}}>
      <div style={{textAlign: 'center'}}>
        <div style={{fontSize: 23, fontWeight: 600, letterSpacing: '.11em', textTransform: 'uppercase', color: MUTED}}>Try it here</div>
        <div style={{fontSize: 75, fontWeight: 700, letterSpacing: '-.04em', marginTop: 8}}>Fetch any mainnet transaction.</div>
      </div>
      <div style={{display: 'flex', gap: 22, marginTop: 34}}>
        <div style={{width: 470, background: '#fff', borderRadius: 20, boxShadow: `0 0 0 1px ${LINE}`, padding: 26, display: 'flex', flexDirection: 'column', gap: 16}}>
          {[
            ['Height', '3533662'],
            ['Position in block', '7'],
          ].map(([k, v]) => (
            <div key={k}>
              <div style={{fontSize: 25, fontWeight: 550, color: '#515154'}}>{k}</div>
              <div style={{marginTop: 6, height: 64, borderRadius: 12, background: '#FAFAFB', boxShadow: 'inset 0 0 0 1px rgba(0,0,0,.14)', fontFamily: MONO, fontSize: 28, display: 'flex', alignItems: 'center', paddingLeft: 14}}>{v}</div>
            </div>
          ))}
          <div style={{marginTop: 8, height: 72, borderRadius: 36, background: INK, color: '#fff', display: 'grid', placeItems: 'center', fontSize: 26, fontWeight: 600, transform: `scale(${press})`}}>Fetch privately</div>
        </div>
        <div style={{flex: 1, background: '#fff', borderRadius: 20, boxShadow: `0 0 0 1px ${LINE}`, padding: 26, display: 'flex', flexDirection: 'column', gap: 18}}>
          <div style={{display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 16}}>
            {stat('Sent', '4 queries, 2.1 MiB', react)}
            {stat('Received', '480 KiB', react + ms(fps, 120))}
            {stat('Queries built', '184 ms', react + ms(fps, 240))}
            {stat('Opened', '55 ms', react + ms(fps, 360))}
          </div>
          <div style={{fontSize: 28, opacity: clamp01(tween(f, pay, pay + ms(fps, 280)))}}>
            <b>txid</b> <span style={{fontFamily: MONO, fontSize: 25}}>f810f80b5b79669efeecc1f500e6aa6b…3a931</span>
          </div>
          <div style={{fontSize: 26, color: MUTED, opacity: clamp01(tween(f, pay + ms(fps, 150), pay + ms(fps, 430)))}}>V6, 13 Ironwood actions, 11 KiB stored.</div>
          <div style={{display: 'flex', gap: 16, marginTop: 4}}>
            <div style={{flex: 1}}>
              <div style={{fontSize: 25, fontWeight: 600}}>A stock lightwalletd would get</div>
              <div style={{marginTop: 8, borderRadius: 10, background: '#F5F5F7', padding: 14, fontFamily: MONO, fontSize: 23, color: '#B3261E', minHeight: 150, opacity: clamp01(tween(f, pay, pay + ms(fps, 300)))}}>
                GetTransaction(TxFilter)
                <br />
                hash: f810f80b5b79669e…
              </div>
            </div>
            <div style={{flex: 1}}>
              <div style={{fontSize: 25, fontWeight: 600}}>This server’s whole log</div>
              <div style={{marginTop: 8, borderRadius: 10, background: '#F5F5F7', padding: 14, fontFamily: MONO, fontSize: 23, color: '#515154', minHeight: 150}}>
                <div style={{opacity: clamp01(tween(f, pay + ms(fps, 200), pay + ms(fps, 500)))}}>17:09 query 4 queries in 2.1 MiB out 480 KiB</div>
                <div>17:09 index out 47 KiB</div>
              </div>
            </div>
          </div>
        </div>
      </div>
      <Cursor f={f} path={[[trig - ms(fps, 700), 1100, 760], [trig, 300, 560]]} clicks={[trig]} />
    </div>
  );
};

/** Laptop (1440×900), dark. Two ticks of the fixed schedule leave identical log
 *  lines; then a tampered server is refused. */
export const BfTicks: React.FC<ScreenProps> = ({f, ev, fps}) => {
  const trig = ev('trigger', 1);
  const react = ev('react', 2);
  const pay = ev('payoff', 3);
  const line = 'in 2,228,276 B  out 491,540 B';
  const block = (title: string, at: number, sub: string) => (
    <PopIn f={f} at={at}>
      <div style={{background: '#202126', borderRadius: 18, padding: '26px 30px'}}>
        <div style={{fontFamily: 'Inter', fontSize: 40, fontWeight: 700, color: '#F2F0EB'}}>{title}</div>
        <div style={{fontFamily: 'Inter', fontSize: 32, color: '#8E8E96', marginTop: 6}}>{sub}</div>
        <div style={{fontFamily: MONO, fontSize: 36, color: '#E7E5DF', marginTop: 18}}>{line}</div>
      </div>
    </PopIn>
  );
  const s = sp(f, pay, fps);
  return (
    <div style={{width: 1440, height: 900, background: '#16171B', padding: '70px 84px', boxSizing: 'border-box', display: 'flex', flexDirection: 'column', gap: 26}}>
      <div style={{fontFamily: 'Inter', fontSize: 30, fontWeight: 600, letterSpacing: '.08em', textTransform: 'uppercase', color: '#8E8E96'}}>The server’s log</div>
      {block('Tick 1', trig, 'the wallet fetched 3 transactions')}
      {block('Tick 2', react, 'the wallet fetched nothing')}
      <div style={{marginTop: 18, opacity: clamp01(s), transform: `translateY(${(1 - clamp01(s)) * 18}px)`, background: '#2A1715', borderRadius: 18, padding: '26px 30px', fontFamily: 'Inter', fontSize: 38, fontWeight: 650, color: '#FFB3A8'}}>
        A tampered server: refused before any query.
      </div>
    </div>
  );
};

export const SCREENS_BLINDFETCH = {BfTerminal, BfIndex, BfRows, BfPage, BfTicks};
