// Phosphor-style line icons. Only the ones we need. 20px viewBox-aware.
// Usage: <Icon name="car" size={16} />
const ICONS = {
  car: <><path d="M4 17h16M6 17v-5l2-5h8l2 5v5M6 12h12M8 17v2M16 17v2M8 14a1 1 0 100-2 1 1 0 000 2zM16 14a1 1 0 100-2 1 1 0 000 2z"/></>,
  'car-simple': <><rect x="3" y="10" width="18" height="8" rx="2"/><path d="M6 10l2-5h8l2 5M7 18v1.5M17 18v1.5"/></>,
  home: <><path d="M3 10l9-7 9 7v10a1 1 0 01-1 1h-5v-7h-6v7H4a1 1 0 01-1-1V10z"/></>,
  calendar: <><rect x="3" y="5" width="18" height="16" rx="2"/><path d="M3 9h18M8 3v4M16 3v4"/></>,
  'calendar-check': <><rect x="3" y="5" width="18" height="16" rx="2"/><path d="M3 9h18M8 3v4M16 3v4M8 14l3 3 5-5"/></>,
  'calendar-plus': <><rect x="3" y="5" width="18" height="16" rx="2"/><path d="M3 9h18M8 3v4M16 3v4M12 12v6M9 15h6"/></>,
  clock: <><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></>,
  coins: <><circle cx="9" cy="9" r="6"/><circle cx="15" cy="15" r="6"/></>,
  'map-pin': <><path d="M12 22s-7-7-7-12a7 7 0 1114 0c0 5-7 12-7 12z"/><circle cx="12" cy="10" r="3"/></>,
  check: <><path d="M5 12l5 5L20 7"/></>,
  x: <><path d="M5 5l14 14M19 5L5 19"/></>,
  arrow: <><path d="M5 12h14M13 6l6 6-6 6"/></>,
  back: <><path d="M19 12H5M11 6l-6 6 6 6"/></>,
  chevron: <><path d="M9 6l6 6-6 6"/></>,
  list: <><path d="M4 6h16M4 12h16M4 18h16"/></>,
  search: <><circle cx="11" cy="11" r="7"/><path d="M21 21l-4.35-4.35"/></>,
  bell: <><path d="M6 8a6 6 0 1112 0c0 7 3 9 3 9H3s3-2 3-9zM10 21a2 2 0 004 0"/></>,
  settings: <><circle cx="12" cy="12" r="3"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"/></>,
  user: <><circle cx="12" cy="8" r="4"/><path d="M4 21c0-4.4 3.6-8 8-8s8 3.6 8 8"/></>,
  users: <><circle cx="9" cy="8" r="4"/><path d="M2 21c0-4 3-7 7-7s7 3 7 7M16 3.5a4 4 0 010 7.5M22 21c0-3.5-2.5-6.3-6-6.9"/></>,
  qr: <><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/><path d="M14 14h3v3h-3zM20 14v3M14 20h3M20 17v4"/></>,
  lightning: <><path d="M13 2L4 14h7l-1 8 9-12h-7l1-8z"/></>,
  leaf: <><path d="M11 20A7 7 0 014 13V5h8a7 7 0 017 7v1a7 7 0 01-8 7zM5 19l14-14"/></>,
  star: <><path d="M12 3l2.9 5.9 6.5.95-4.7 4.6 1.1 6.5L12 17.9l-5.8 3.05 1.1-6.5L2.6 9.85l6.5-.95L12 3z"/></>,
  trend: <><path d="M3 17l6-6 4 4 8-8M14 7h7v7"/></>,
  gauge: <><path d="M12 14l5-5M3 12a9 9 0 0118 0v7H3v-7z"/></>,
  wrench: <><path d="M14.7 6.3a4 4 0 00-5.4 5.4L3 18v3h3l6.3-6.3a4 4 0 005.4-5.4l-2.5 2.5-2.9-.1-.1-2.9 2.5-2.5z"/></>,
  grid: <><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/></>,
  sun: <><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"/></>,
  moon: <><path d="M21 13A9 9 0 1111 3a7 7 0 0010 10z"/></>,
  swap: <><path d="M7 4l-4 4 4 4M3 8h14M17 20l4-4-4-4M21 16H7"/></>,
  motorcycle: <><circle cx="6" cy="17" r="3"/><circle cx="18" cy="17" r="3"/><path d="M9 17h6l-2-6h3M13 11l-3-4h4"/></>,
  wheelchair: <><circle cx="11" cy="5" r="2"/><path d="M11 8v5h6l2 6M8 11a6 6 0 108 8"/></>,
  download: <><path d="M12 3v12M7 10l5 5 5-5M4 21h16"/></>,
  github: <><path d="M12 2a10 10 0 00-3.16 19.5c.5.08.68-.22.68-.48v-1.7c-2.78.6-3.37-1.34-3.37-1.34-.45-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.6.07-.6 1 .07 1.53 1.03 1.53 1.03.9 1.52 2.34 1.08 2.91.83.09-.65.35-1.08.63-1.33-2.22-.25-4.55-1.1-4.55-4.94 0-1.1.4-2 1.04-2.7-.1-.26-.45-1.28.1-2.67 0 0 .85-.27 2.78 1.02a9.65 9.65 0 015.06 0c1.93-1.29 2.78-1.02 2.78-1.02.55 1.39.2 2.41.1 2.67.65.7 1.04 1.6 1.04 2.7 0 3.85-2.34 4.69-4.57 4.93.36.31.68.92.68 1.85v2.74c0 .27.18.58.69.48A10 10 0 0012 2z"/></>,
  shield: <><path d="M12 3l8 3v6c0 5-3.5 8.5-8 9-4.5-.5-8-4-8-9V6l8-3z"/></>,
  globe: <><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3a13 13 0 010 18M12 3a13 13 0 000 18"/></>,
  code: <><path d="M8 7l-5 5 5 5M16 7l5 5-5 5M14 4l-4 16"/></>,
  plus: <><path d="M12 5v14M5 12h14"/></>,
  minus: <><path d="M5 12h14"/></>,
  filter: <><path d="M3 5h18l-7 8v6l-4 2v-8L3 5z"/></>,
  rocket: <><path d="M4 20l4-4M15 7l2 2M10 14a6 6 0 0111-5l-7 7c-3-2-4-2-4-2zM5 12a4 4 0 00-1 7 4 4 0 007-1"/></>,
  sparkle: <><path d="M12 3l2 6 6 2-6 2-2 6-2-6-6-2 6-2 2-6zM19 4l.7 2 2 .7-2 .7L19 10l-.7-1.6-2-.7 2-.7L19 4z"/></>,
  crown: <><path d="M4 8l4 4 4-6 4 6 4-4v10H4V8z"/></>,
  trophy: <><path d="M8 4h8v6a4 4 0 11-8 0V4zM4 4h4v3a2 2 0 01-2 2 2 2 0 01-2-2V4zM16 4h4v3a2 2 0 01-2 2 2 2 0 01-2-2V4zM10 16h4v2l2 2H8l2-2v-2z"/></>,
  printer: <><path d="M7 8V3h10v5M5 8h14a2 2 0 012 2v6h-4v4H7v-4H3v-6a2 2 0 012-2zM7 16h10"/></>,
  copy: <><rect x="8" y="8" width="13" height="13" rx="2"/><path d="M16 8V5a2 2 0 00-2-2H5a2 2 0 00-2 2v9a2 2 0 002 2h3"/></>,
  heart: <><path d="M12 20s-7-4.5-7-10a4 4 0 017-3 4 4 0 017 3c0 5.5-7 10-7 10z"/></>,
  eye: <><path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/></>,
  'eye-off': <><path d="M3 3l18 18M10.6 6.2A9 9 0 0112 6c6 0 10 6 10 6a16.5 16.5 0 01-3 3.6M6.6 6.6A16 16 0 002 12s4 7 10 7a9 9 0 004.4-1.1M9.9 9.9a3 3 0 004.2 4.2"/></>,
  book: <><path d="M4 4h12a4 4 0 014 4v12H8a4 4 0 01-4-4V4zM4 16a4 4 0 014-4h12"/></>,
  database: <><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M3 5v14a9 3 0 0018 0V5M3 12a9 3 0 0018 0"/></>,
  server: <><rect x="3" y="4" width="18" height="7" rx="1"/><rect x="3" y="13" width="18" height="7" rx="1"/><path d="M7 8h.01M7 17h.01"/></>,
  key: <><circle cx="8" cy="15" r="4"/><path d="M11 12l10-10M17 6l3 3M15 8l3 3"/></>,
  'trending-up': <><path d="M3 17l6-6 4 4 8-8M14 7h7v7"/></>,
  'chart-line': <><path d="M3 3v18h18M7 15l4-4 3 3 6-6"/></>,
  menu: <><path d="M4 6h16M4 12h16M4 18h16"/></>,
  info: <><circle cx="12" cy="12" r="9"/><path d="M12 8v.01M12 11v5"/></>,
};

function Icon({ name, size = 18, weight = 1.75, className = '', style }) {
  const ic = ICONS[name];
  if (!ic) return <span style={{width:size,height:size,display:'inline-block',border:'1px dashed currentColor',opacity:0.4,fontSize:10,textAlign:'center'}}>?</span>;
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none"
         stroke="currentColor" strokeWidth={weight}
         strokeLinecap="round" strokeLinejoin="round"
         className={className} style={style} aria-hidden="true">
      {ic}
    </svg>
  );
}

window.Icon = Icon;
