// The "user's screenshot" that goes inside Gloss mockups: a believable sample app (fake data).
import React from 'react';
import {ScreenProps} from '../types';

const Status: React.FC<{dark?: boolean}> = ({dark}) => (
  <div style={{height: 54, display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '0 34px 0 40px', fontFamily: 'Inter', fontWeight: 600, fontSize: 17, color: dark ? '#fff' : '#111'}}>
    <span>9:41</span>
    <span style={{display: 'flex', gap: 6, alignItems: 'center'}}>
      <span style={{width: 18, height: 11, borderRadius: 2, background: dark ? '#fff' : '#111', opacity: 0.9}} />
      <span style={{width: 25, height: 12, borderRadius: 4, border: `1.5px solid ${dark ? '#fff' : '#111'}`, padding: 1.5, display: 'inline-block'}}>
        <span style={{display: 'block', width: '75%', height: '100%', borderRadius: 2, background: dark ? '#fff' : '#111'}} />
      </span>
    </span>
  </div>
);

const TASKS = [
  {c: '#245AE6', t: 'Ship onboarding v2', s: 'Design · 10:00'},
  {c: '#F2994A', t: 'Review App Store copy', s: 'Launch · 11:30'},
  {c: '#27AE60', t: 'Lunch with Priya', s: 'Personal · 13:00'},
  {c: '#9B51E0', t: 'Record demo video', s: 'Launch · 15:00'},
];

export const GlossApp: React.FC<Partial<ScreenProps> & {variant?: number}> = ({variant = 0}) => {
  if (variant === 1) return <FitnessApp />;
  if (variant === 2) return <MusicApp />;
  return (
    <div style={{position: 'relative', width: 393, height: 852, background: 'linear-gradient(180deg, #F3F6FF 0%, #FFFFFF 40%)', fontFamily: 'Inter', color: '#121216'}}>
      <Status />
      <div style={{padding: '20px 26px 0'}}>
        <div style={{fontSize: 15, color: '#6b6f7b', fontWeight: 500}}>Tuesday, 14 October</div>
        <div style={{fontSize: 34, fontWeight: 760, letterSpacing: '-0.035em', lineHeight: 1.08, marginTop: 6}}>
          Good morning,
          <br />
          <span style={{color: '#245AE6'}}>Maya.</span>
        </div>
        <div style={{marginTop: 22, display: 'flex', gap: 12}}>
          {[
            ['4', 'tasks'],
            ['2h', 'focus'],
            ['76%', 'week'],
          ].map(([a, b]) => (
            <div key={b} style={{flex: 1, background: '#fff', borderRadius: 18, padding: '14px 14px', boxShadow: '0 1px 2px rgba(0,0,0,0.05), 0 6px 18px rgba(30,40,90,0.06)'}}>
              <div style={{fontSize: 24, fontWeight: 740, letterSpacing: '-0.03em'}}>{a}</div>
              <div style={{fontSize: 13, color: '#7a7e8a', marginTop: 2}}>{b}</div>
            </div>
          ))}
        </div>
        <div style={{marginTop: 26, fontSize: 18, fontWeight: 700, letterSpacing: '-0.02em'}}>Today</div>
        <div style={{marginTop: 12, display: 'flex', flexDirection: 'column', gap: 10}}>
          {TASKS.map((t, i) => (
            <div key={i} style={{display: 'flex', alignItems: 'center', gap: 14, background: '#fff', borderRadius: 18, padding: '15px 16px', boxShadow: '0 1px 2px rgba(0,0,0,0.05), 0 6px 18px rgba(30,40,90,0.05)'}}>
              <div style={{width: 24, height: 24, borderRadius: 8, border: `2.5px solid ${t.c}`, background: i === 0 ? t.c : 'transparent'}} />
              <div style={{flex: 1}}>
                <div style={{fontSize: 16, fontWeight: 620, letterSpacing: '-0.01em', textDecoration: i === 0 ? 'line-through' : undefined, color: i === 0 ? '#9a9da8' : undefined}}>{t.t}</div>
                <div style={{fontSize: 13, color: '#8a8d98', marginTop: 2}}>{t.s}</div>
              </div>
            </div>
          ))}
        </div>
      </div>
      <div style={{position: 'absolute', left: 26, right: 26, bottom: 34, height: 64, borderRadius: 32, background: '#121216', display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#fff', fontWeight: 650, fontSize: 17}}>+ New task</div>
    </div>
  );
};

const FitnessApp: React.FC = () => (
  <div style={{width: 393, height: 852, background: '#0E0F13', color: '#fff', fontFamily: 'Inter'}}>
    <Status dark />
    <div style={{padding: '24px 26px'}}>
      <div style={{fontSize: 15, color: '#8b8f9c'}}>Activity</div>
      <div style={{fontSize: 34, fontWeight: 760, letterSpacing: '-0.035em', marginTop: 4}}>Morning run</div>
      <div style={{margin: '34px auto 0', width: 250, height: 250, borderRadius: '50%', background: 'conic-gradient(#C6FF4D 0 72%, #23252c 72% 100%)', display: 'grid', placeItems: 'center'}}>
        <div style={{width: 206, height: 206, borderRadius: '50%', background: '#0E0F13', display: 'grid', placeItems: 'center', textAlign: 'center'}}>
          <div>
            <div style={{fontSize: 56, fontWeight: 780, letterSpacing: '-0.05em'}}>7.2</div>
            <div style={{fontSize: 15, color: '#8b8f9c'}}>km of 10</div>
          </div>
        </div>
      </div>
      <div style={{display: 'flex', gap: 12, marginTop: 40}}>
        {[
          ['5:12', 'pace'],
          ['38m', 'time'],
          ['412', 'kcal'],
        ].map(([a, b]) => (
          <div key={b} style={{flex: 1, background: '#1a1c22', borderRadius: 18, padding: 14}}>
            <div style={{fontSize: 22, fontWeight: 720}}>{a}</div>
            <div style={{fontSize: 13, color: '#8b8f9c'}}>{b}</div>
          </div>
        ))}
      </div>
    </div>
  </div>
);

const MusicApp: React.FC = () => (
  <div style={{width: 393, height: 852, background: 'linear-gradient(180deg, #FFDCCB 0%, #FFF5EF 55%)', fontFamily: 'Inter', color: '#1b1412'}}>
    <Status />
    <div style={{padding: '30px 30px'}}>
      <div style={{width: 333, height: 333, borderRadius: 26, background: 'linear-gradient(135deg, #ff7a59, #ffb36b 50%, #7d6bd6)', boxShadow: '0 20px 50px rgba(255,122,89,0.35)'}} />
      <div style={{marginTop: 30, fontSize: 28, fontWeight: 760, letterSpacing: '-0.03em'}}>Golden Hour</div>
      <div style={{fontSize: 17, color: '#8a6f66', marginTop: 4}}>Lumen Park</div>
      <div style={{marginTop: 28, height: 5, borderRadius: 5, background: 'rgba(0,0,0,0.1)'}}>
        <div style={{width: '42%', height: 5, borderRadius: 5, background: '#1b1412'}} />
      </div>
      <div style={{display: 'flex', justifyContent: 'center', gap: 44, marginTop: 34, alignItems: 'center'}}>
        <div style={{width: 0, height: 0, borderTop: '14px solid transparent', borderBottom: '14px solid transparent', borderRight: '20px solid #1b1412'}} />
        <div style={{width: 76, height: 76, borderRadius: '50%', background: '#1b1412', display: 'grid', placeItems: 'center'}}>
          <div style={{display: 'flex', gap: 8}}>
            <div style={{width: 8, height: 26, background: '#fff', borderRadius: 2}} />
            <div style={{width: 8, height: 26, background: '#fff', borderRadius: 2}} />
          </div>
        </div>
        <div style={{width: 0, height: 0, borderTop: '14px solid transparent', borderBottom: '14px solid transparent', borderLeft: '20px solid #1b1412'}} />
      </div>
    </div>
  </div>
);
