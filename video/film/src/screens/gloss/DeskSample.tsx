// A sample desktop "screenshot" (fake data) for laptop/monitor mockups inside Gloss.
import React from 'react';

export const DeskSample: React.FC = () => (
  <div style={{width: 1440, height: 900, background: '#F7F8FB', fontFamily: 'Inter', color: '#14151a', display: 'flex'}}>
    <div style={{width: 240, background: '#fff', borderRight: '1px solid #ebedf2', padding: 28}}>
      <div style={{fontWeight: 780, fontSize: 24, letterSpacing: '-0.03em'}}>Northstar</div>
      {['Overview', 'Projects', 'Launches', 'Team', 'Settings'].map((t, i) => (
        <div key={t} style={{marginTop: i === 0 ? 36 : 10, padding: '12px 14px', borderRadius: 10, background: i === 0 ? '#EEF3FF' : 'transparent', color: i === 0 ? '#245AE6' : '#5c6070', fontWeight: 600, fontSize: 17}}>
          {t}
        </div>
      ))}
    </div>
    <div style={{flex: 1, padding: 44}}>
      <div style={{fontSize: 44, fontWeight: 760, letterSpacing: '-0.035em'}}>Launch overview</div>
      <div style={{display: 'flex', gap: 22, marginTop: 30}}>
        {[
          ['12.4k', 'Visitors', '#245AE6'],
          ['1,382', 'Sign-ups', '#27AE60'],
          ['11.2%', 'Conversion', '#F2994A'],
        ].map(([a, b, c]) => (
          <div key={b} style={{flex: 1, background: '#fff', borderRadius: 20, padding: 26, boxShadow: '0 1px 2px rgba(0,0,0,0.04), 0 10px 30px rgba(20,30,80,0.06)'}}>
            <div style={{fontSize: 16, color: '#7b7f8c', fontWeight: 600}}>{b}</div>
            <div style={{fontSize: 46, fontWeight: 780, letterSpacing: '-0.04em', marginTop: 6}}>{a}</div>
            <div style={{marginTop: 14, height: 6, borderRadius: 6, background: '#eef0f5'}}>
              <div style={{width: '64%', height: 6, borderRadius: 6, background: c}} />
            </div>
          </div>
        ))}
      </div>
      <div style={{marginTop: 26, background: '#fff', borderRadius: 20, height: 420, padding: 26, boxShadow: '0 1px 2px rgba(0,0,0,0.04), 0 10px 30px rgba(20,30,80,0.06)', display: 'flex', alignItems: 'flex-end', gap: 18}}>
        {[38, 52, 46, 70, 64, 88, 80, 96, 90, 112, 120, 134].map((h, i) => (
          <div key={i} style={{flex: 1, height: h * 2.6, borderRadius: 10, background: i > 8 ? '#245AE6' : '#DBE5FF'}} />
        ))}
      </div>
    </div>
  </div>
);
