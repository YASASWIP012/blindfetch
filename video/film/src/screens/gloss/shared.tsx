import React from 'react';

export const G = {
  bg: '#F4F2EE',
  panel: '#FBFAF8',
  line: '#E6E3DD',
  ink: '#16161A',
  muted: '#676773',
  blue: '#245AE6',
  blueSoft: '#DBE5FF',
};

export const BACKDROPS = [
  'linear-gradient(140deg, #cfdcff 0%, #e9dcff 48%, #ffe5d4 100%)', // silk dawn
  'radial-gradient(120% 90% at 20% 15%, #9fc0ff 0%, rgba(159,192,255,0) 55%), radial-gradient(90% 80% at 85% 85%, #ffc2d9 0%, rgba(255,194,217,0) 60%), #eef1ff', // mesh
  'linear-gradient(180deg, #ffb28a 0%, #ff8fa3 40%, #7d6bd6 100%)', // landscape dusk
  'radial-gradient(80% 70% at 50% 40%, #fafafa 0%, #dcdcdf 100%)', // studio
  'linear-gradient(135deg, #0f2a6b 0%, #245ae6 55%, #8fb0ff 100%)', // cobalt
  'linear-gradient(160deg, #d9f2e6 0%, #bfe3ff 100%)', // mint
  'radial-gradient(100% 80% at 70% 20%, #3a3a46 0%, #111116 70%)', // midnight
  'linear-gradient(135deg, #fff1d6 0%, #ffd6c2 50%, #f7c6e0 100%)', // paper peach
];

export const GlossMark: React.FC<{size: number}> = ({size}) => (
  <svg width={size} height={size} viewBox="0 0 32 32">
    <defs>
      <linearGradient id="gm-bg" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stopColor="#5b8cff" />
        <stop offset="1" stopColor="#245ae6" />
      </linearGradient>
      <linearGradient id="gm-shine" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stopColor="#fff" stopOpacity="0.45" />
        <stop offset="1" stopColor="#fff" stopOpacity="0" />
      </linearGradient>
    </defs>
    <path d="M16 1c9.6 0 15 5.4 15 15s-5.4 15-15 15S1 25.6 1 16 6.4 1 16 1Z" fill="url(#gm-bg)" />
    <path d="M16 2.2c7.6 0 12.2 3 13.3 9.1C25 9.6 20.7 8.9 16 8.9S7 9.6 2.7 11.3C3.8 5.2 8.4 2.2 16 2.2Z" fill="url(#gm-shine)" />
    <rect x="10" y="7" width="12" height="20" rx="3.6" fill="#16161a" opacity="0.16" />
    <rect x="10" y="6" width="12" height="20" rx="3.6" fill="#fff" />
    <rect x="11.6" y="7.6" width="8.8" height="16.8" rx="2.2" fill="#dbe5ff" />
    <path d="M11.6 17.4 L20.4 9.2 V12.6 L11.6 20.8 Z" fill="#fff" opacity="0.9" />
    <path d="M11.6 22.2 L20.4 14 V15.2 L11.6 23.4 Z" fill="#fff" opacity="0.6" />
  </svg>
);

/** Simple silhouette icons for the device picker. */
export const DeviceGlyph: React.FC<{kind: 'phone' | 'tablet' | 'laptop' | 'watch' | 'monitor'; size: number; color?: string}> = ({kind, size, color = '#16161A'}) => {
  const s = size;
  if (kind === 'phone') return <div style={{width: s * 0.42, height: s * 0.82, borderRadius: s * 0.1, border: `${s * 0.045}px solid ${color}`, background: '#fff'}} />;
  if (kind === 'tablet') return <div style={{width: s * 0.66, height: s * 0.86, borderRadius: s * 0.08, border: `${s * 0.045}px solid ${color}`, background: '#fff'}} />;
  if (kind === 'watch') return <div style={{width: s * 0.44, height: s * 0.52, borderRadius: s * 0.14, border: `${s * 0.05}px solid ${color}`, background: '#fff'}} />;
  if (kind === 'monitor')
    return (
      <div style={{display: 'flex', flexDirection: 'column', alignItems: 'center'}}>
        <div style={{width: s * 0.86, height: s * 0.52, borderRadius: s * 0.04, border: `${s * 0.045}px solid ${color}`, background: '#fff'}} />
        <div style={{width: s * 0.1, height: s * 0.14, background: color}} />
      </div>
    );
  return (
    <div style={{display: 'flex', flexDirection: 'column', alignItems: 'center'}}>
      <div style={{width: s * 0.74, height: s * 0.48, borderRadius: s * 0.04, border: `${s * 0.045}px solid ${color}`, background: '#fff'}} />
      <div style={{width: s * 0.92, height: s * 0.06, borderRadius: s * 0.03, background: color}} />
    </div>
  );
};
