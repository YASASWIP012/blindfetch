// The thing Gloss makes: a device mockup on a backdrop. Drawn at 1200×735 logical.
// - drop: dashed drop zone → a screenshot is dragged in → dropped → the frame wraps it
// - steps: device / backdrop / layout changes on beats (escalation: 43 devices, 22 backdrops, 20 layouts)
import React from 'react';
import {EASE, keys, ms, sp, tween, clamp01, drift} from '../../lib/motion';
import {Phone, Laptop, Sheen} from '../../components/Device';
import {Cursor} from '../../components/UI';
import {ScreenProps} from '../types';
import {BACKDROPS, G} from './shared';
import {GlossApp} from './GlossApp';
import {DeskSample} from './DeskSample';

type At = string | number;
type Device = 'iphone' | 'pixel' | 'ipad' | 'macbook';
type Layout = 'single' | 'duo' | 'trio' | '3d' | 'closeup';
type Step = {at: At; device?: Device; backdrop?: number; layout?: Layout};

export const MOCK = {w: 1200, h: 735};

const Tablet: React.FC<{h: number; children: React.ReactNode}> = ({h, children}) => {
  const w = h * 0.76;
  return (
    <div style={{width: w, height: h, borderRadius: h * 0.06, background: '#1b1b1e', padding: h * 0.025, boxShadow: '0 30px 70px rgba(20,20,40,0.22), 0 6px 14px rgba(20,20,40,0.12)'}}>
      <div style={{width: '100%', height: '100%', borderRadius: h * 0.035, overflow: 'hidden', position: 'relative', background: '#fff'}}>
        <div style={{transform: `scale(${(w - h * 0.05) / 393})`, transformOrigin: '0 0', width: 393, height: 852}}>{children}</div>
      </div>
    </div>
  );
};

const DeviceView: React.FC<{d: Device; h: number; variant?: number; sheen?: number}> = ({d, h, variant = 0, sheen = -2}) => {
  if (d === 'macbook')
    return (
      <Laptop width={h * 1.45} shadowK={0.6} sheen={sheen}>
        <DeskSample />
      </Laptop>
    );
  if (d === 'ipad')
    return (
      <Tablet h={h * 1.02}>
        <GlossApp variant={variant} />
      </Tablet>
    );
  return (
    <Phone height={h} tone={d === 'pixel' ? 'silver' : 'graphite'} island={d !== 'pixel'} shadowK={0.7} sheen={sheen}>
      <GlossApp variant={variant} />
    </Phone>
  );
};

export const GlossMockup: React.FC<
  ScreenProps & {
    drop?: boolean;
    device?: Device;
    backdrop?: number;
    layout?: Layout;
    steps?: Step[];
    events?: Record<string, string>;
    variant?: number;
    sheenAt?: At;
  }
> = ({f, ev, b, fps, drop = false, device = 'iphone', backdrop = 0, layout = 'single', steps = [], events = {}, variant = 0, sheenAt}) => {
  const at = (a: At) => (typeof a === 'string' ? ev(events[a] ?? a) : b(a));
  const W = MOCK.w;
  const H = MOCK.h;

  // ---------- current state from steps ----------
  let st = {device, backdrop, layout};
  let prevBackdrop = backdrop;
  let devAt = -1e6;
  let bdAt = -1e6;
  let layAt = -1e6;
  for (const s of steps) {
    const t = at(s.at);
    if (f < t) break;
    if (s.device && s.device !== st.device) [st.device, devAt] = [s.device, t];
    if (s.backdrop !== undefined && s.backdrop !== st.backdrop) [prevBackdrop, st.backdrop, bdAt] = [st.backdrop, s.backdrop, t];
    if (s.layout && s.layout !== st.layout) [st.layout, layAt] = [s.layout, t];
  }
  const swap = Math.min(sp(f, devAt, fps, 0.6, 2.6), 1.2);
  const lay = sp(f, layAt, fps, 0.7, 2.2);
  const bdP = tween(f, bdAt, bdAt + ms(fps, 420), 0, 1, EASE.move);
  const sheen = sheenAt !== undefined ? tween(f, at(sheenAt), at(sheenAt) + ms(fps, 900), -1, 1, EASE.move) : -2;

  // ---------- drop flow ----------
  const tDrag = drop ? at('drag') : -1e6;
  const tDrop = drop ? at('drop') : -1e6;
  const tFrame = drop ? at('framed') : -1e6;
  const framed = drop ? clamp01(sp(f, tFrame, fps, 0.72, 2.0)) : 1;
  const reveal = drop ? tween(f, tFrame - 2, tFrame + ms(fps, 760), 0, 1, EASE.move) : 1;
  const zoneOut = drop ? tween(f, tDrop, tDrop + ms(fps, 260), 0, 1, EASE.exit) : 1;

  const phoneH = st.layout === 'closeup' ? 980 : st.layout === 'trio' ? 520 : st.layout === 'duo' ? 560 : 590;
  const R = Math.hypot(W, H) / 2 + 4;

  const devices = () => {
    const pop = 0.88 + 0.12 * swap;
    const L = st.layout;
    if (L === 'duo')
      return (
        <div style={{display: 'flex', alignItems: 'center', gap: 0, transform: `scale(${0.92 + 0.08 * lay})`}}>
          <div style={{transform: `translateX(${60 * lay}px) translateY(18px) rotate(-5deg)`, zIndex: 1}}>
            <DeviceView d="iphone" h={phoneH} variant={2} sheen={sheen} />
          </div>
          <div style={{transform: `translateX(${-60 * lay}px) translateY(-14px) rotate(4deg)`, zIndex: 2}}>
            <DeviceView d={st.device} h={phoneH} variant={variant} sheen={sheen} />
          </div>
        </div>
      );
    if (L === 'trio')
      return (
        <div style={{display: 'flex', alignItems: 'center', transform: `scale(${0.92 + 0.08 * lay})`}}>
          <div style={{transform: `translateX(${70 * lay}px) rotate(-7deg) translateY(30px)`, zIndex: 1}}>
            <DeviceView d="iphone" h={phoneH * 0.92} variant={1} />
          </div>
          <div style={{zIndex: 3}}>
            <DeviceView d={st.device} h={phoneH} variant={variant} sheen={sheen} />
          </div>
          <div style={{transform: `translateX(${-70 * lay}px) rotate(7deg) translateY(30px)`, zIndex: 1}}>
            <DeviceView d="iphone" h={phoneH * 0.92} variant={2} />
          </div>
        </div>
      );
    if (L === '3d')
      return (
        <div style={{perspective: 1500}}>
          <div style={{transform: `rotateY(${-30 * lay + drift(f, 'm3d', 2, 200)}deg) rotateX(${14 * lay}deg) rotateZ(${4 * lay}deg)`, transformStyle: 'preserve-3d'}}>
            <DeviceView d={st.device} h={phoneH} variant={variant} sheen={sheen} />
          </div>
        </div>
      );
    if (L === 'closeup')
      return (
        <div style={{transform: `translateY(${280 * lay}px) rotate(${-8 * lay}deg)`}}>
          <DeviceView d={st.device} h={phoneH} variant={variant} sheen={sheen} />
        </div>
      );
    return (
      <div style={{transform: `scale(${pop})`}}>
        <DeviceView d={st.device} h={st.device === 'macbook' ? 400 : st.device === 'ipad' ? 540 : phoneH} variant={variant} sheen={sheen} />
      </div>
    );
  };

  // screenshot thumbnail being dragged in (drop flow)
  const phoneScreen = {w: phoneH * 0.4847 * 0.916, h: phoneH * 0.4847 * 0.916 * (852 / 393)};
  let thumb: React.ReactNode = null;
  let cursor: React.ReactNode = null;
  if (drop && f < tFrame + ms(fps, 200)) {
    const cx = keys(f, [[tDrag, W + 140], [tDrop - 2, W / 2 + 10, EASE.move]]);
    const cy = keys(f, [[tDrag, H + 200], [tDrop - 2, H / 2 + 20, EASE.move]]);
    const land = tween(f, tDrop, tDrop + ms(fps, 340), 0, 1, EASE.premium);
    const w0 = 170;
    const w = w0 + (phoneScreen.w - w0) * land;
    const sc = w / 393;
    const x = cx - 40 + (W / 2 - phoneScreen.w / 2 - (cx - 40)) * land;
    const y = cy - 70 + (H / 2 - phoneScreen.h / 2 - (cy - 70)) * land;
    const shadowK = 1 - land;
    thumb = (
      <div style={{position: 'absolute', left: x, top: y, width: 393 * sc, height: 852 * sc, borderRadius: 34 * sc + 8 * shadowK, overflow: 'hidden', transform: `rotate(${-8 * (1 - land)}deg)`, boxShadow: `0 ${30 * shadowK + 6}px ${60 * shadowK + 16}px rgba(20,20,50,${0.25 * shadowK + 0.06})`, opacity: 1 - clamp01((f - tFrame) / 6)}}>
        <div style={{transform: `scale(${sc})`, transformOrigin: '0 0'}}>
          <GlossApp variant={variant} />
        </div>
      </div>
    );
    cursor = <Cursor f={f} path={[[tDrag, W + 100, H + 140], [tDrop - 2, W / 2 + 34, H / 2 + 62], [tFrame + ms(fps, 600), W - 120, H - 40]]} size={40} color={G.ink} />;
  }

  return (
    <div style={{position: 'relative', width: W, height: H, overflow: 'hidden', background: '#F7F6F3', fontFamily: 'Inter'}}>
      {/* backdrop: previous, then the new one wiping in from the centre */}
      <div style={{position: 'absolute', inset: 0, background: BACKDROPS[prevBackdrop % BACKDROPS.length], clipPath: drop ? `circle(${R * reveal}px at 50% 50%)` : undefined}} />
      <div style={{position: 'absolute', inset: 0, background: BACKDROPS[st.backdrop % BACKDROPS.length], clipPath: `circle(${R * Math.min(bdP, reveal)}px at 50% 50%)`}} />
      {drop && zoneOut < 1 ? (
        <div style={{position: 'absolute', inset: 44, borderRadius: 26, border: `3px dashed #CFCBC3`, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 16, opacity: 1 - zoneOut, color: G.muted}}>
          <div style={{width: 76, height: 76, borderRadius: 20, background: G.blueSoft, display: 'grid', placeItems: 'center'}}>
            <svg width="38" height="38" viewBox="0 0 24 24" fill="none" stroke={G.blue} strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round">
              <path d="M12 16V4M6 10l6-6 6 6M4 20h16" />
            </svg>
          </div>
          <div style={{fontSize: 30, fontWeight: 650, color: G.ink, letterSpacing: '-0.02em'}}>Drop an image or video</div>
          <div style={{fontSize: 21}}>PNG · JPG · MP4 — it never leaves your browser</div>
        </div>
      ) : null}
      <div style={{position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', opacity: drop ? (f >= tFrame - 1 ? 1 : 0) : 1, transform: drop ? `scale(${1.08 - 0.08 * framed})` : undefined}}>{devices()}</div>
      {thumb}
      {cursor}
      <Sheen pos={tween(f, drop ? tFrame : -1e6, (drop ? tFrame : -1e6) + ms(fps, 1100), -1, 1, EASE.move)} strength={0.16} />
    </div>
  );
};
