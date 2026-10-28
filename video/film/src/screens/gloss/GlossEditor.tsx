// Gloss's editor, rebuilt in code at 1440×854 (browser content area) so every part can move.
// Modes map to features: drop → devices → animate → final (end card).
import React from 'react';
import {EASE, keys, ms, sp, tween, clamp01} from '../../lib/motion';
import {Phone, Laptop, Sheen} from '../../components/Device';
import {Cursor, Check} from '../../components/UI';
import {ScreenProps} from '../types';
import {G, BACKDROPS, GlossMark, DeviceGlyph} from './shared';
import {GlossApp} from './GlossApp';
import {DeskSample} from './DeskSample';

// ---- layout constants (screen coords; scenes use these for camera focus + callouts) ----
export const ED = {
  W: 1440,
  H: 854,
  top: 52,
  rail: 64,
  left: 344,
  right: 1136,
  board: {x: 440, y: 172, w: 600, h: 480},
  tile: {x0: 86, y0: 286, w: 76, h: 92, gx: 86, gy: 102},
  swatch: {x0: 1158, y0: 318, w: 82, h: 58, gx: 90, gy: 66},
};
export const boardCenter = {x: ED.board.x + ED.board.w / 2, y: ED.board.y + ED.board.h / 2};

const TILES: Array<{name: string; kind: 'phone' | 'tablet' | 'laptop' | 'watch' | 'monitor'}> = [
  {name: 'iPhone 18 Pro', kind: 'phone'},
  {name: 'iPhone SE', kind: 'phone'},
  {name: 'Pixel 7', kind: 'phone'},
  {name: 'Galaxy S24', kind: 'phone'},
  {name: 'iPad', kind: 'tablet'},
  {name: 'MacBook', kind: 'laptop'},
  {name: 'Watch', kind: 'watch'},
  {name: 'Studio Display', kind: 'monitor'},
  {name: 'Nothing', kind: 'phone'},
];
export const tilePos = (i: number) => ({x: ED.tile.x0 + (i % 3) * ED.tile.gx + ED.tile.w / 2, y: ED.tile.y0 + Math.floor(i / 3) * ED.tile.gy + ED.tile.h / 2});
export const swatchPos = (i: number) => ({x: ED.swatch.x0 + (i % 3) * ED.swatch.gx + ED.swatch.w / 2, y: ED.swatch.y0 + Math.floor(i / 3) * ED.swatch.gy + ED.swatch.h / 2});

type Theme = {bg: string; panel: string; line: string; ink: string; muted: string; canvas: string; chip: string};
const LIGHT: Theme = {bg: G.bg, panel: G.panel, line: G.line, ink: G.ink, muted: G.muted, canvas: '#ECEAE5', chip: '#fff'};
const DARK: Theme = {bg: '#141418', panel: '#1A1A1F', line: '#2A2A31', ink: '#F2F2F5', muted: '#8E8E9A', canvas: '#0F0F13', chip: '#24242A'};

export const GlossEditor: React.FC<ScreenProps & {mode?: 'drop' | 'devices' | 'animate' | 'final'; events?: Record<string, string>; theme?: 'light' | 'dark'}> = ({f, ev, fps, dark, mode = 'drop', events = {}, theme}) => {
  dark = theme ? theme === 'dark' : dark;
  const T = dark ? DARK : LIGHT;
  // event names can be remapped per use: events={{trigger: 'kf1'}}
  const E = (k: string, fb: number) => ev(events[k] ?? k, fb);
  const trig = E('trigger', 1);
  const react = E('react', 2);
  const pay = E('payoff', 3);

  // ---------- which device / backdrop is active ----------
  let activeTile = 0;
  type Dev = 'phone' | 'pixel' | 'tablet' | 'laptop';
  let device = 'phone' as Dev;
  let swapAt = -999;
  if (mode === 'devices') {
    const order: Array<[number, Dev, number]> = [
      [trig, 'pixel', 2],
      [react, 'tablet', 4],
      [pay, 'laptop', 5],
    ];
    for (const [t, d, tile] of order) if (f >= t) [device, activeTile, swapAt] = [d, tile, t];
  }
  const backdrop = mode === 'animate' ? BACKDROPS[4] : mode === 'final' ? BACKDROPS[1] : BACKDROPS[0];

  // ---------- drop mode choreography ----------
  const dropped = mode !== 'drop' || f >= trig;
  const framed = mode !== 'drop' ? 1 : clamp01(sp(f, react, fps, 0.7, 2.2));
  const bdReveal = mode !== 'drop' ? 1 : tween(f, react - 2, react + ms(fps, 700), 0, 1, EASE.move);
  const toast = mode === 'drop' ? clamp01(tween(f, pay, pay + ms(fps, 300), 0, 1, EASE.premium)) : 0;
  if (mode === 'drop' && f >= pay) activeTile = 0;

  // ---------- animate mode keyframes (the "camera" inside the mockup) ----------
  const az = mode === 'animate' ? keys(f, [[trig, 1], [trig + ms(fps, 700), 1.22, EASE.camera], [react, 1.22], [react + ms(fps, 700), 1.08, EASE.camera]]) : 1;
  const ary = mode === 'animate' ? keys(f, [[react, 0], [react + ms(fps, 800), -28, EASE.camera], [pay + ms(fps, 200), -28], [pay + ms(fps, 900), -16, EASE.camera]]) : mode === 'final' ? -14 : 0;
  const arx = mode === 'animate' ? keys(f, [[react, 0], [react + ms(fps, 800), 12, EASE.camera]]) : mode === 'final' ? 8 : 0;
  const apx = mode === 'animate' ? keys(f, [[trig, 0], [trig + ms(fps, 700), -30, EASE.camera], [react, -30], [react + ms(fps, 700), 0, EASE.camera]]) : 0;

  // ---------- device swap pop ----------
  const swapP = mode === 'devices' && swapAt > -999 ? sp(f, swapAt, fps, 0.6, 2.6) : 1;
  const devScale = mode === 'devices' ? 0.86 + 0.14 * swapP : 1;

  const boardDevice = () => {
    const sheen = mode === 'final' ? tween(f, 0, ms(fps, 1600), -1, 1, EASE.move) : -2;
    if (device === 'laptop') {
      return (
        <Laptop width={430} shadowK={0.5}>
          <DeskSample />
        </Laptop>
      );
    }
    if (device === 'tablet') {
      return (
        <div style={{width: 300, height: 400, borderRadius: 26, background: '#1b1b1e', padding: 12, boxShadow: '0 20px 50px rgba(0,0,0,0.18)'}}>
          <div style={{width: '100%', height: '100%', borderRadius: 16, overflow: 'hidden', position: 'relative'}}>
            <div style={{transform: 'scale(0.705)', transformOrigin: '0 0', width: 393, height: 852}}>
              <GlossApp />
            </div>
          </div>
        </div>
      );
    }
    if (mode === 'final') {
      return (
        <div style={{display: 'flex', gap: 26, alignItems: 'center'}}>
          <Phone height={380} shadowK={0.45} sheen={sheen}>
            <GlossApp />
          </Phone>
          <div style={{transform: 'translateY(30px)'}}>
            <Phone height={380} shadowK={0.45} tone="silver" sheen={sheen - 0.2}>
              <GlossApp variant={2} />
            </Phone>
          </div>
        </div>
      );
    }
    return (
      <Phone height={400} shadowK={0.5} tone={device === 'pixel' ? 'silver' : 'graphite'} island={device !== 'pixel'} sheen={mode === 'animate' ? tween(f, pay, pay + ms(fps, 800), -1, 1, EASE.move) : -2}>
        <GlossApp variant={mode === 'animate' ? 1 : 0} />
      </Phone>
    );
  };

  // ---------- cursor paths ----------
  let cursor: React.ReactNode = null;
  if (mode === 'drop') {
    cursor = (
      <Cursor
        f={f}
        path={[
          [0, 250, 900],
          [trig - 3, boardCenter.x - 10, boardCenter.y + 10],
          [react + 6, boardCenter.x - 10, boardCenter.y + 10],
          [pay + ms(fps, 500), 1290, 610],
        ]}
        clicks={[]}
        color={T.ink}
      />
    );
  } else if (mode === 'devices') {
    const p2 = tilePos(2), p4 = tilePos(4), p5 = tilePos(5);
    cursor = (
      <Cursor
        f={f}
        path={[
          [-20, 420, 700],
          [trig - 2, p2.x, p2.y],
          [react - 2, p4.x, p4.y],
          [pay - 2, p5.x, p5.y],
          [pay + ms(fps, 600), p5.x + 60, p5.y + 120],
        ]}
        clicks={[trig, react, pay]}
        color={T.ink}
      />
    );
  } else if (mode === 'animate') {
    cursor = (
      <Cursor
        f={f}
        path={[
          [-10, 900, 780],
          [trig - 2, 610, 764],
          [react - 2, 770, 790],
          [pay - 2, 1388, 26],
          [pay + ms(fps, 700), 1330, 120],
        ]}
        clicks={[trig, react, pay]}
        color="#ffffff"
      />
    );
  }

  // drag thumbnail (drop mode, before the drop)
  let thumb: React.ReactNode = null;
  if (mode === 'drop' && f < trig + ms(fps, 320)) {
    const cx = keys(f, [[0, 250, EASE.move], [trig - 3, boardCenter.x - 10, EASE.move]]);
    const cy = keys(f, [[0, 900, EASE.move], [trig - 3, boardCenter.y + 10, EASE.move]]);
    const land = tween(f, trig, trig + ms(fps, 320), 0, 1, EASE.premium);
    const phoneScreen = {w: 400 * 0.4847 * 0.916, h: 400 * 0.4847 * 0.916 * (852 / 393)};
    const w = 120 + (phoneScreen.w - 120) * land;
    const scale = w / 393;
    const x = cx - 24 + (boardCenter.x - phoneScreen.w / 2 - (cx - 24)) * land;
    const y = cy - 40 + (boardCenter.y - phoneScreen.h / 2 - (cy - 40)) * land;
    thumb = (
      <div style={{position: 'absolute', left: x, top: y, width: 393 * scale, height: 852 * scale, borderRadius: 30 * scale + 6 * (1 - land), overflow: 'hidden', transform: `rotate(${-7 * (1 - land)}deg)`, boxShadow: `0 ${18 * (1 - land) + 4}px ${40 * (1 - land) + 10}px rgba(0,0,0,${0.22 * (1 - land) + 0.05})`, zIndex: 40}}>
        <div style={{transform: `scale(${scale})`, transformOrigin: '0 0'}}>
          <GlossApp />
        </div>
      </div>
    );
  }

  // ---------- layout ----------
  const canvasBottom = mode === 'animate' ? 700 : ED.H;
  return (
    <div style={{position: 'relative', width: ED.W, height: ED.H, background: T.bg, fontFamily: 'Inter', color: T.ink, overflow: 'hidden'}}>
      {/* top bar */}
      <div style={{position: 'absolute', left: 0, top: 0, width: ED.W, height: ED.top, display: 'flex', alignItems: 'center', padding: '0 18px', gap: 12, background: T.panel, borderBottom: `1px solid ${T.line}`}}>
        <GlossMark size={26} />
        <span style={{fontWeight: 760, fontSize: 18, letterSpacing: '-0.03em'}}>gloss</span>
        <span style={{width: 1, height: 20, background: T.line, margin: '0 6px'}} />
        <span style={{fontSize: 14, color: T.muted}}>Untitled mockup</span>
        <div style={{flex: 1, display: 'flex', justifyContent: 'center'}}>
          <div style={{display: 'flex', gap: 2, padding: 3, borderRadius: 10, background: dark ? '#202026' : '#EFEDE8'}}>
            {['Design', 'Annotate', 'Animate'].map((t) => {
              const on = (mode === 'animate' && t === 'Animate') || (mode !== 'animate' && t === 'Design');
              return (
                <div key={t} style={{padding: '6px 16px', borderRadius: 8, fontSize: 13, fontWeight: 600, background: on ? T.chip : 'transparent', color: on ? T.ink : T.muted, boxShadow: on ? '0 1px 2px rgba(0,0,0,0.08)' : undefined}}>
                  {t}
                </div>
              );
            })}
          </div>
        </div>
        <div style={{position: 'relative', padding: '8px 18px', borderRadius: 999, background: dark ? '#F2F2F5' : G.ink, color: dark ? G.ink : '#fff', fontSize: 14, fontWeight: 700, transform: `scale(${mode === 'animate' ? keys(f, [[pay - 2, 1], [pay, 0.92, EASE.premium], [pay + 8, 1, EASE.premium]]) : 1})`}}>
          Export
        </div>
      </div>

      {/* left rail */}
      <div style={{position: 'absolute', left: 0, top: ED.top, width: ED.rail, bottom: 0, background: T.panel, borderRight: `1px solid ${T.line}`, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 18, paddingTop: 18}}>
        {['Device', 'Canvas', 'Text', 'Motion'].map((t, i) => (
          <div key={t} style={{display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4}}>
            <div style={{width: 34, height: 34, borderRadius: 10, background: i === 0 ? G.blueSoft : 'transparent', border: i === 0 ? 'none' : `1.5px solid ${T.line}`}} />
            <span style={{fontSize: 10, color: T.muted}}>{t}</span>
          </div>
        ))}
      </div>

      {/* left panel: devices */}
      <div style={{position: 'absolute', left: ED.rail, top: ED.top, width: ED.left - ED.rail, bottom: 0, background: T.panel, borderRight: `1px solid ${T.line}`, padding: '18px 20px'}}>
        <div style={{fontSize: 15, fontWeight: 700}}>Device</div>
        <div style={{fontSize: 12, color: T.muted, marginTop: 2}}>{TILES[activeTile].name}</div>
        <div style={{marginTop: 14, height: 38, borderRadius: 10, background: dark ? '#F2F2F5' : G.ink, color: dark ? G.ink : '#fff', display: 'flex', alignItems: 'center', justifyContent: 'center', fontSize: 13, fontWeight: 650}}>Add image or video</div>
        <div style={{marginTop: 14, display: 'flex', gap: 4, padding: 3, borderRadius: 9, background: dark ? '#202026' : '#EFEDE8'}}>
          {['Portrait', 'Landscape'].map((t, i) => (
            <div key={t} style={{flex: 1, textAlign: 'center', padding: '5px 0', borderRadius: 7, fontSize: 12, fontWeight: 600, background: i === 0 ? T.chip : 'transparent', color: i === 0 ? T.ink : T.muted}}>
              {t}
            </div>
          ))}
        </div>
        <div style={{marginTop: 18, fontSize: 11, fontWeight: 650, letterSpacing: '0.08em', color: T.muted, textTransform: 'uppercase'}}>All devices · 43</div>
      </div>
      {TILES.map((t, i) => {
        const p = tilePos(i);
        const on = i === activeTile && (mode !== 'drop' || f >= pay);
        return (
          <div key={t.name} style={{position: 'absolute', left: p.x - ED.tile.w / 2, top: p.y - ED.tile.h / 2, width: ED.tile.w, height: ED.tile.h, borderRadius: 12, background: on ? (dark ? '#22263a' : '#EEF3FF') : T.chip, boxShadow: on ? `0 0 0 2px ${G.blue}` : `0 0 0 1px ${T.line}`, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 6}}>
            <div style={{height: 46, display: 'flex', alignItems: 'center'}}>
              <DeviceGlyph kind={t.kind} size={46} color={on ? G.blue : dark ? '#c9c9d2' : '#2a2a30'} />
            </div>
            <span style={{fontSize: 10, color: on ? G.blue : T.muted, fontWeight: 600, whiteSpace: 'nowrap'}}>{t.name}</span>
          </div>
        );
      })}

      {/* canvas */}
      <div style={{position: 'absolute', left: ED.left, top: ED.top, width: ED.right - ED.left, height: canvasBottom - ED.top, background: T.canvas, backgroundImage: `radial-gradient(${dark ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.07)'} 1px, transparent 1px)`, backgroundSize: '18px 18px'}} />
      <div style={{position: 'absolute', left: ED.board.x, top: ED.board.y - (mode === 'animate' ? 40 : 0), width: ED.board.w, height: ED.board.h, borderRadius: 16, overflow: 'hidden', background: dark ? '#1c1c22' : '#F7F6F3', boxShadow: '0 1px 2px rgba(0,0,0,0.06), 0 20px 50px rgba(0,0,0,0.10)'}}>
        <div style={{position: 'absolute', inset: 0, background: backdrop, clipPath: `circle(${bdReveal * 420}px at 50% 50%)`}} />
        {mode === 'drop' && !dropped ? (
          <div style={{position: 'absolute', inset: 28, borderRadius: 14, border: `2px dashed ${dark ? '#3a3a44' : '#CFCBC3'}`, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 10, color: T.muted}}>
            <div style={{width: 46, height: 46, borderRadius: 12, background: G.blueSoft, display: 'grid', placeItems: 'center'}}>
              <div style={{width: 16, height: 16, borderLeft: `3px solid ${G.blue}`, borderTop: `3px solid ${G.blue}`, transform: 'rotate(45deg) translate(3px, 3px)'}} />
            </div>
            <div style={{fontSize: 16, fontWeight: 650, color: T.ink}}>Drop an image or video</div>
            <div style={{fontSize: 13}}>PNG, JPG, MP4 · stays in your browser</div>
          </div>
        ) : null}
        <div style={{position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', perspective: 1600}}>
          <div style={{transform: `translateX(${apx}px) scale(${az * devScale}) rotateX(${arx}deg) rotateY(${ary}deg)`, transformStyle: 'preserve-3d', opacity: mode === 'drop' ? (framed > 0.01 ? 1 : 0) : 1}}>
            <div style={{transform: mode === 'drop' ? `scale(${1.1 - 0.1 * framed})` : undefined, opacity: mode === 'drop' ? clamp01(framed * 2) : 1}}>{boardDevice()}</div>
          </div>
        </div>
        {mode === 'final' ? <Sheen pos={tween(f, ms(fps, 300), ms(fps, 1800), -1, 1, EASE.move)} strength={0.18} /> : null}
      </div>
      {/* screenshot that was dropped sits on the board until the frame wraps it */}
      {mode === 'drop' && f >= trig && framed < 0.5 ? thumb : null}
      {mode === 'drop' && f < trig ? thumb : null}

      {/* toast (drop payoff) */}
      {toast > 0 ? (
        <div style={{position: 'absolute', left: boardCenter.x, top: ED.board.y - 52, transform: `translate(-50%, ${(1 - toast) * -10}px)`, opacity: toast, display: 'flex', alignItems: 'center', gap: 10, padding: '8px 16px 8px 8px', borderRadius: 999, background: T.chip, boxShadow: '0 6px 20px rgba(0,0,0,0.12)', fontSize: 14, fontWeight: 650, whiteSpace: 'nowrap'}}>
          <Check f={f} at={pay} size={26} bg={G.blue} />
          Framed in iPhone 18 Pro
        </div>
      ) : null}

      {/* right panel: style */}
      <div style={{position: 'absolute', left: ED.right, top: ED.top, right: 0, bottom: 0, background: T.panel, borderLeft: `1px solid ${T.line}`, padding: '18px 22px'}}>
        <div style={{fontSize: 15, fontWeight: 700}}>Style</div>
        <div style={{marginTop: 14, fontSize: 11, fontWeight: 650, letterSpacing: '0.08em', color: T.muted, textTransform: 'uppercase'}}>Layout · 20</div>
        <div style={{marginTop: 10, display: 'flex', gap: 6}}>
          {['Single', 'Duo', 'Trio', '3D'].map((t, i) => {
            const on = (mode === 'final' && i === 1) || (mode === 'animate' && i === 3) || (mode !== 'final' && mode !== 'animate' && i === 0);
            return (
              <div key={t} style={{padding: '6px 11px', borderRadius: 8, fontSize: 12, fontWeight: 600, background: on ? G.blue : T.chip, color: on ? '#fff' : T.muted, boxShadow: on ? undefined : `0 0 0 1px ${T.line}`}}>
                {t}
              </div>
            );
          })}
        </div>
        <div style={{marginTop: 20, fontSize: 11, fontWeight: 650, letterSpacing: '0.08em', color: T.muted, textTransform: 'uppercase'}}>Backdrop · 22</div>
        <div style={{marginTop: 10, display: 'flex', gap: 12, fontSize: 12, fontWeight: 600}}>
          {['Silk', 'Mesh', 'Landscape', 'Studio'].map((t, i) => (
            <span key={t} style={{color: i === 0 ? T.ink : T.muted, borderBottom: i === 0 ? `2px solid ${G.blue}` : undefined, paddingBottom: 3}}>
              {t}
            </span>
          ))}
        </div>
      </div>
      {BACKDROPS.slice(0, 6).map((bgc, i) => {
        const p = swatchPos(i);
        const on = (mode === 'animate' && i === 4) || (mode === 'final' && i === 1) || ((mode === 'drop' || mode === 'devices') && i === 0 && dropped);
        return <div key={i} style={{position: 'absolute', left: p.x - ED.swatch.w / 2, top: p.y - ED.swatch.h / 2, width: ED.swatch.w, height: ED.swatch.h, borderRadius: 10, background: bgc, boxShadow: on ? `0 0 0 2px ${T.panel}, 0 0 0 4px ${G.blue}` : `0 0 0 1px ${T.line}`}} />;
      })}
      <div style={{position: 'absolute', left: ED.right + 22, top: 470, width: ED.W - ED.right - 44}}>
        {[
          ['Shadow', 0.7],
          ['Noise', 0.25],
          ['Vignette', 0.4],
        ].map(([t, v]) => (
          <div key={t as string} style={{marginBottom: 16}}>
            <div style={{display: 'flex', justifyContent: 'space-between', fontSize: 12, color: T.muted, fontWeight: 600}}>
              <span>{t}</span>
              <span>{Math.round((v as number) * 100)}</span>
            </div>
            <div style={{marginTop: 7, height: 4, borderRadius: 4, background: T.line}}>
              <div style={{width: `${(v as number) * 100}%`, height: 4, borderRadius: 4, background: G.blue}} />
            </div>
          </div>
        ))}
      </div>

      {/* timeline (animate) */}
      {mode === 'animate' ? <Timeline f={f} fps={fps} trig={trig} react={react} pay={pay} T={T} /> : null}
      {mode === 'animate' && f >= pay ? (
        <div style={{position: 'absolute', right: 18, top: 60, width: 220, borderRadius: 14, background: T.chip, boxShadow: '0 16px 40px rgba(0,0,0,0.35)', padding: 10, opacity: clamp01(tween(f, pay, pay + ms(fps, 200), 0, 1)), transform: `translateY(${(1 - clamp01(tween(f, pay, pay + ms(fps, 260), 0, 1, EASE.premium))) * -8}px)`}}>
          {[
            ['PNG', '2×'],
            ['MP4', '60 fps'],
            ['WebM', 'transparent'],
          ].map(([a, b], i) => (
            <div key={a} style={{display: 'flex', justifyContent: 'space-between', alignItems: 'center', padding: '9px 10px', borderRadius: 9, background: i === 1 ? G.blue : 'transparent', color: i === 1 ? '#fff' : T.ink, fontSize: 13, fontWeight: 650}}>
              <span>{a}</span>
              <span style={{opacity: 0.75, fontWeight: 500}}>{b}</span>
            </div>
          ))}
        </div>
      ) : null}

      {cursor}
    </div>
  );
};

const Timeline: React.FC<{f: number; fps: number; trig: number; react: number; pay: number; T: Theme}> = ({f, fps, trig, react, pay, T}) => {
  const x0 = ED.left + 90;
  const x1 = ED.right - 30;
  const t0 = trig - ms(fps, 300);
  const t1 = pay + ms(fps, 900);
  const px = (fr: number) => x0 + ((fr - t0) / (t1 - t0)) * (x1 - x0);
  const head = px(Math.min(t1, Math.max(t0, f)));
  const tracks: Array<[string, number[]]> = [
    ['Zoom', [trig, trig + ms(fps, 700), react + ms(fps, 700)]],
    ['Rotate', [react, react + ms(fps, 800)]],
    ['Tilt 3D', [react, react + ms(fps, 800), pay + ms(fps, 900)]],
  ];
  return (
    <div style={{position: 'absolute', left: ED.left, top: 700, width: ED.right - ED.left, height: ED.H - 700, background: T.panel, borderTop: `1px solid ${T.line}`}}>
      {tracks.map(([name, ks], r) => (
        <div key={name} style={{position: 'absolute', left: 0, top: 18 + r * 40, width: '100%', height: 30}}>
          <span style={{position: 'absolute', left: 16, top: 7, fontSize: 12, color: T.muted, fontWeight: 600}}>{name}</span>
          <div style={{position: 'absolute', left: x0 - ED.left, right: 30, top: 14, height: 2, background: T.line}} />
          {ks.map((k, i) => {
            const lit = f >= k;
            const pop = lit ? clamp01(sp(f, k, fps, 0.5, 3)) : 0;
            return <div key={i} style={{position: 'absolute', left: px(k) - ED.left - 7, top: 8, width: 14, height: 14, transform: `rotate(45deg) scale(${0.8 + 0.3 * pop})`, background: lit ? G.blue : T.chip, border: `2px solid ${lit ? G.blue : T.muted}`, borderRadius: 2}} />;
          })}
        </div>
      ))}
      <div style={{position: 'absolute', left: head - ED.left - 1, top: 6, width: 2, bottom: 10, background: '#ff5a3c'}} />
      <div style={{position: 'absolute', left: head - ED.left - 6, top: 0, width: 12, height: 10, borderRadius: 3, background: '#ff5a3c'}} />
    </div>
  );
};
