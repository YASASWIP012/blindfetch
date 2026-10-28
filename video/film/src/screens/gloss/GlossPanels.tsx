// Small Gloss panels for the long film: template cards, the stats strip, pricing, caption icon.
import React from 'react';
import {EASE, ms, tween, clamp01, sp} from '../../lib/motion';
import {Phone, Laptop} from '../../components/Device';
import {Counter} from '../../components/Type';
import {Check} from '../../components/UI';
import {ScreenProps} from '../types';
import {BACKDROPS, G, GlossMark} from './shared';
import {GlossApp} from './GlossApp';
import {DeskSample} from './DeskSample';

type At = string | number;

// Real Gloss template names and sizes (from the product site).
export const TEMPLATES: Array<{name: string; size: string; aspect: number; kind: 'phone' | 'duo' | 'laptop' | 'trio'}> = [
  {name: 'Product Hunt', size: '1270 × 760', aspect: 1270 / 760, kind: 'duo'},
  {name: 'App Store', size: '1290 × 2796', aspect: 0.62, kind: 'phone'},
  {name: 'Open Graph', size: '1200 × 630', aspect: 1200 / 630, kind: 'laptop'},
  {name: 'X / Twitter', size: '1600 × 900', aspect: 16 / 9, kind: 'trio'},
  {name: 'Instagram 4:5', size: '1080 × 1350', aspect: 0.8, kind: 'phone'},
  {name: 'LinkedIn', size: '1200 × 627', aspect: 1200 / 627, kind: 'laptop'},
  {name: 'Dribbble', size: '1600 × 1200', aspect: 4 / 3, kind: 'duo'},
  {name: 'Google Play', size: '1024 × 500', aspect: 1024 / 500, kind: 'trio'},
  {name: 'Website 16:9', size: '1920 × 1080', aspect: 16 / 9, kind: 'laptop'},
  {name: 'App promo video', size: 'MP4 · 60 fps', aspect: 16 / 9, kind: 'phone'},
];

/** A template thumbnail: backdrop + devices + label chip. Logical 640×440. */
export const GlossTemplate: React.FC<ScreenProps & {variant?: number; label?: boolean}> = ({variant = 0, label = true}) => {
  const t = TEMPLATES[variant % TEMPLATES.length];
  const bd = BACKDROPS[(variant * 3 + 1) % BACKDROPS.length];
  const ph = 300;
  let body: React.ReactNode;
  if (t.kind === 'laptop')
    body = (
      <Laptop width={360} shadowK={0.4}>
        <DeskSample />
      </Laptop>
    );
  else if (t.kind === 'duo')
    body = (
      <div style={{display: 'flex'}}>
        <div style={{transform: 'translateX(30px) rotate(-6deg)'}}>
          <Phone height={ph} shadowK={0.35}>
            <GlossApp variant={2} />
          </Phone>
        </div>
        <div style={{transform: 'translateX(-30px) rotate(5deg) translateY(-10px)'}}>
          <Phone height={ph} shadowK={0.35}>
            <GlossApp variant={0} />
          </Phone>
        </div>
      </div>
    );
  else if (t.kind === 'trio')
    body = (
      <div style={{display: 'flex', alignItems: 'center'}}>
        {[1, 0, 2].map((v, i) => (
          <div key={i} style={{transform: `translateX(${(1 - i) * 26}px) rotate(${(i - 1) * 6}deg) translateY(${i === 1 ? -8 : 14}px)`, zIndex: i === 1 ? 2 : 1}}>
            <Phone height={i === 1 ? ph * 0.92 : ph * 0.8} shadowK={0.35}>
              <GlossApp variant={v} />
            </Phone>
          </div>
        ))}
      </div>
    );
  else
    body = (
      <Phone height={ph * 1.08} shadowK={0.35}>
        <GlossApp variant={variant % 3} />
      </Phone>
    );
  return (
    <div style={{position: 'relative', width: 640, height: 440, background: bd, overflow: 'hidden', fontFamily: 'Inter'}}>
      <div style={{position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center', transform: 'translateY(16px)'}}>{body}</div>
      {label ? (
        <div style={{position: 'absolute', left: 18, top: 16, display: 'flex', gap: 8, alignItems: 'center', padding: '7px 12px', borderRadius: 999, background: 'rgba(255,255,255,0.85)', fontSize: 17, fontWeight: 650, color: G.ink}}>
          {t.name}
          <span style={{fontWeight: 500, color: G.muted, fontSize: 15}}>{t.size}</span>
        </div>
      ) : null}
    </div>
  );
};

/** The stats strip from the Gloss site, counting up. Logical 1400×220. */
export const GlossStats: React.FC<ScreenProps & {at?: At; events?: Record<string, string>}> = ({f, ev, b, fps, at = 0}) => {
  const t0 = typeof at === 'string' ? ev(at) : b(at);
  const items: Array<[number, string, string]> = [
    [43, '', 'device frames'],
    [20, '', 'layouts, incl. 3D'],
    [24, '', 'templates'],
    [22, '', 'wallpapers'],
    [100, '+', 'canvas sizes'],
    [0, '', 'files uploaded'],
  ];
  return (
    <div style={{width: 1400, height: 220, display: 'flex', alignItems: 'center', fontFamily: 'Inter', color: G.ink, background: '#fff'}}>
      {items.map(([v, suf, lab], i) => {
        const s = t0 + ms(fps, 90) * i;
        const p = clamp01(tween(f, s, s + ms(fps, 400), 0, 1, EASE.premium));
        return (
          <div key={i} style={{flex: 1, textAlign: 'center', borderLeft: i ? `1px solid ${G.line}` : undefined, opacity: p, transform: `translateY(${(1 - p) * 16}px)`}}>
            <div style={{fontSize: 66, fontWeight: 760, letterSpacing: '-0.04em', color: i === 5 ? G.blue : G.ink}}>
              <Counter f={f} from={i === 5 ? 9 : 0} to={v} f0={s} f1={s + ms(fps, 1100)} suffix={suf} />
            </div>
            <div style={{fontSize: 21, color: G.muted, marginTop: 4}}>{lab}</div>
          </div>
        );
      })}
    </div>
  );
};

/** Free vs Pro card. Logical 560×640. tier 'free' | 'pro'. Checks tick in from `at`. */
export const GlossPrice: React.FC<ScreenProps & {tier?: 'free' | 'pro'; at?: At}> = ({f, ev, b, fps, tier = 'pro', at = 0}) => {
  const t0 = typeof at === 'string' ? ev(at) : b(at);
  const pro = tier === 'pro';
  const rows = pro
    ? ['No watermark', 'Video export, up to 60 fps', '4× and 6K exports', 'Unlimited drafts & themes', 'Commercial use']
    : ['All 43 device frames', 'All layouts & templates', 'All wallpapers', 'Annotation tools', 'Small “Made with Gloss” mark'];
  return (
    <div style={{width: 560, height: 640, padding: 44, fontFamily: 'Inter', color: pro ? '#fff' : G.ink, background: pro ? 'linear-gradient(160deg, #2c63f0 0%, #1a46c4 100%)' : '#fff', boxSizing: 'border-box', position: 'relative'}}>
      <div style={{fontSize: 24, fontWeight: 650, opacity: 0.85}}>{pro ? 'Pro' : 'Free'}</div>
      <div style={{display: 'flex', alignItems: 'baseline', gap: 12, marginTop: 10}}>
        <div style={{fontSize: 92, fontWeight: 780, letterSpacing: '-0.05em'}}>{pro ? '$29' : '$0'}</div>
        <div style={{fontSize: 24, opacity: 0.75}}>{pro ? 'once' : 'forever'}</div>
      </div>
      <div style={{fontSize: 19, opacity: 0.75, marginTop: 4}}>{pro ? 'Launch price · first 100 customers, then $99' : 'No sign-up. Nothing uploaded.'}</div>
      <div style={{marginTop: 34, display: 'flex', flexDirection: 'column', gap: 18}}>
        {rows.map((r, i) => {
          const s = t0 + ms(fps, 140) * i;
          const p = clamp01(tween(f, s, s + ms(fps, 300), 0, 1, EASE.premium));
          return (
            <div key={i} style={{display: 'flex', alignItems: 'center', gap: 14, fontSize: 23, fontWeight: 520, opacity: 0.3 + 0.7 * p}}>
              <Check f={f} at={s} size={30} bg={pro ? 'rgba(255,255,255,0.22)' : G.blueSoft} color={pro ? '#fff' : G.blue} />
              {r}
            </div>
          );
        })}
      </div>
      <div style={{position: 'absolute', left: 44, right: 44, bottom: 40, height: 66, borderRadius: 999, background: pro ? '#fff' : G.ink, color: pro ? G.blue : '#fff', display: 'grid', placeItems: 'center', fontSize: 23, fontWeight: 700, transform: `scale(${pro ? 1 + 0.03 * Math.max(0, Math.sin(clamp01(sp(f, t0 + ms(fps, 900), fps, 0.5, 2)) * Math.PI)) : 1})`}}>
        {pro ? 'Get lifetime access' : 'Open the editor'}
      </div>
    </div>
  );
};

/** Caption icon: the Gloss mark. */
export const GlossIcon: React.FC<Partial<ScreenProps>> = () => <GlossMark size={54} />;
