// Theme showcase — shows all 12 themes + 5 use-case palettes in a grid
function ThemeShowcase({ currentTheme, currentUseCase, onThemeChange, onUseCaseChange }) {
  const themes = [
    { k: 'classic', l: 'Classic', d: 'Clean, minimal, professional' },
    { k: 'glass', l: 'Glass', d: 'Frosted glass with blur' },
    { k: 'bento', l: 'Bento', d: 'Japanese-inspired grid' },
    { k: 'brutalist', l: 'Brutalist', d: 'Raw, bold, high-contrast' },
    { k: 'neon', l: 'Neon', d: 'Dark with vibrant glow' },
    { k: 'warm', l: 'Warm', d: 'Earthy, soft gradients' },
    { k: 'liquid', l: 'Liquid', d: 'Fluid, organic shapes' },
    { k: 'mono', l: 'Mono', d: 'Grayscale + single accent' },
    { k: 'ocean', l: 'Ocean', d: 'Deep blue maritime' },
    { k: 'forest', l: 'Forest', d: 'Green earth tones' },
    { k: 'synthwave', l: 'Synthwave', d: 'Retro 80s gradient' },
    { k: 'zen', l: 'Zen', d: 'Whitespace-first minimal' },
  ];
  const useCases = [
    { k: 'company', l: 'Company', hue: 175, c: 'Teal · universal' },
    { k: 'residential', l: 'Residential', hue: 155, c: 'Emerald · home' },
    { k: 'shared', l: 'Shared', hue: 290, c: 'Violet · coworking' },
    { k: 'rental', l: 'Rental', hue: 260, c: 'Indigo · commercial' },
    { k: 'personal', l: 'Personal', hue: 12, c: 'Rose · individual' },
  ];

  return (
    <div style={{ padding: 28, display: 'flex', flexDirection: 'column', gap: 24 }}>
      <div>
        <h1 style={{ fontSize: 28, fontWeight: 800, letterSpacing: '-0.03em' }}>Theme & use-case system</h1>
        <p style={{ fontSize: 14, color: 'var(--theme-text-muted)', marginTop: 6 }}>
          12 themes × 5 use-case palettes = 60 visual combinations. Hue shifts are OKLCH for perceptual uniformity.
        </p>
      </div>

      {/* Use cases */}
      <div>
        <h3 style={{ fontSize: 13, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.05em', color: 'var(--theme-text-muted)', marginBottom: 10 }}>
          Use-case palette — drives primary color
        </h3>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5,1fr)', gap: 10 }}>
          {useCases.map((u) => {
            const selected = currentUseCase === u.k;
            return (
              <button key={u.k} onClick={() => onUseCaseChange(u.k)} className="card" style={{
                padding: 16, textAlign: 'left',
                border: '2px solid', borderColor: selected ? `oklch(0.7 0.15 ${u.hue})` : 'var(--theme-border)',
                cursor: 'pointer',
              }}>
                <div style={{ display: 'flex', gap: 3, marginBottom: 10 }}>
                  {[0.95, 0.85, 0.7, 0.55, 0.4, 0.25].map((L) => (
                    <span key={L} style={{ flex: 1, height: 18, borderRadius: 3, background: `oklch(${L} 0.14 ${u.hue})` }}/>
                  ))}
                </div>
                <div style={{ fontSize: 13, fontWeight: 700 }}>{u.l}</div>
                <div style={{ fontSize: 11, color: 'var(--theme-text-muted)', marginTop: 2 }}>{u.c}</div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Themes */}
      <div>
        <h3 style={{ fontSize: 13, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.05em', color: 'var(--theme-text-muted)', marginBottom: 10 }}>
          Design theme — {themes.length} presets
        </h3>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4,1fr)', gap: 10 }}>
          {themes.map((t) => {
            const selected = currentTheme === t.k;
            return (
              <button key={t.k} onClick={() => onThemeChange(t.k)} className="card" style={{
                padding: 14, textAlign: 'left',
                border: '2px solid', borderColor: selected ? 'var(--color-primary-500)' : 'var(--theme-border)',
                cursor: 'pointer', display: 'flex', flexDirection: 'column', gap: 10,
              }}>
                <ThemePreview themeKey={t.k} />
                <div>
                  <div style={{ fontSize: 13, fontWeight: 700 }}>{t.l}</div>
                  <div style={{ fontSize: 11, color: 'var(--theme-text-muted)', marginTop: 1 }}>{t.d}</div>
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Component samples */}
      <div>
        <h3 style={{ fontSize: 13, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.05em', color: 'var(--theme-text-muted)', marginBottom: 10 }}>
          Component samples · current combination
        </h3>
        <div className="card" style={{ padding: 24, display: 'flex', flexDirection: 'column', gap: 20 }}>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8 }}>
            <button className="btn btn-primary">Primary</button>
            <button className="btn btn-secondary">Secondary</button>
            <button className="btn btn-ghost">Ghost</button>
            <button className="btn btn-primary btn-sm">Small</button>
            <button className="btn btn-icon"><Icon name="plus" size={14}/></button>
          </div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
            {['success','warning','error','info','primary','gray'].map((b) => (
              <span key={b} className={`badge badge-${b}`}>{b}</span>
            ))}
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3,1fr)', gap: 10 }}>
            <div className="input">Input value</div>
            <div className="input" style={{ opacity: 0.5 }}>Disabled</div>
            <div className="input" style={{ borderColor: 'var(--color-primary-500)', boxShadow: '0 0 0 3px color-mix(in oklch, var(--color-primary-500) 20%, transparent)' }}>Focused</div>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3,1fr)', gap: 10 }}>
            <div className="stat-card" style={{ padding: 14 }}>
              <div style={{ fontSize: 11, color: 'var(--theme-text-muted)' }}>Bookings</div>
              <div style={{ fontSize: 22, fontWeight: 800, fontVariantNumeric: 'tabular-nums' }}>247</div>
            </div>
            <div className="glass-card" style={{ padding: 14 }}>
              <div style={{ fontSize: 11, color: 'var(--theme-text-muted)' }}>glass-card</div>
              <div style={{ fontSize: 22, fontWeight: 800, fontVariantNumeric: 'tabular-nums' }}>45</div>
            </div>
            <div className="card" style={{ padding: 14 }}>
              <div style={{ fontSize: 11, color: 'var(--theme-text-muted)' }}>card</div>
              <div style={{ fontSize: 22, fontWeight: 800, fontVariantNumeric: 'tabular-nums' }}>12.4</div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

function ThemePreview({ themeKey }) {
  // Each theme gets a distinct tiny preview
  const styles = {
    classic: { bg: 'var(--theme-bg)', card: 'var(--theme-card-bg)', accent: 'var(--color-primary-500)', border: 'var(--theme-border)' },
    glass: { bg: 'linear-gradient(135deg, #c7d2fe, #bae6fd)', card: 'rgba(255,255,255,0.6)', accent: 'var(--color-primary-500)', border: 'rgba(255,255,255,0.8)' },
    bento: { bg: '#fef3c7', card: '#fff', accent: '#f97316', border: '#1f2937' },
    brutalist: { bg: '#fff', card: '#fde047', accent: '#000', border: '#000' },
    neon: { bg: '#020617', card: '#0f172a', accent: '#22d3ee', border: '#22d3ee' },
    warm: { bg: 'linear-gradient(135deg, #fef3c7, #fed7aa)', card: '#fff', accent: '#d97706', border: '#fbbf24' },
    liquid: { bg: 'linear-gradient(120deg, #ddd6fe, #fbcfe8)', card: '#fff', accent: '#a855f7', border: 'rgba(168,85,247,0.3)' },
    mono: { bg: '#fafafa', card: '#fff', accent: '#18181b', border: '#e4e4e7' },
    ocean: { bg: 'linear-gradient(180deg, #0c4a6e, #1e3a8a)', card: '#075985', accent: '#38bdf8', border: '#0284c7' },
    forest: { bg: '#f0fdf4', card: '#fff', accent: '#15803d', border: '#86efac' },
    synthwave: { bg: 'linear-gradient(180deg, #4c1d95, #ec4899)', card: 'rgba(0,0,0,0.4)', accent: '#f0abfc', border: '#f0abfc' },
    zen: { bg: '#fafaf9', card: '#fff', accent: '#57534e', border: '#e7e5e4' },
  }[themeKey] || {};
  const text = ['neon','ocean','synthwave'].includes(themeKey) ? '#fff' : '#0f172a';
  return (
    <div style={{
      height: 90, borderRadius: 8, padding: 10, background: styles.bg,
      border: themeKey === 'brutalist' ? `2px solid ${styles.border}` : `1px solid ${styles.border}`,
      display: 'flex', flexDirection: 'column', gap: 6, color: text, overflow: 'hidden',
      position: 'relative',
    }}>
      <div style={{
        padding: '4px 8px', background: styles.card,
        borderRadius: themeKey === 'brutalist' ? 0 : 4,
        border: themeKey === 'brutalist' ? `2px solid ${styles.border}` : 'none',
        backdropFilter: themeKey === 'glass' ? 'blur(8px)' : 'none',
        fontSize: 9, fontWeight: 700,
        boxShadow: themeKey === 'brutalist' ? `3px 3px 0 ${styles.border}` : 'none',
      }}>Dashboard</div>
      <div style={{ display: 'flex', gap: 4 }}>
        <span style={{ padding: '2px 6px', fontSize: 9, fontWeight: 700, borderRadius: themeKey === 'brutalist' ? 0 : 3, background: styles.accent, color: ['brutalist','mono','zen','forest','warm','bento'].includes(themeKey) ? '#fff' : text === '#fff' ? '#000' : '#fff' }}>Book</span>
        <span style={{ padding: '2px 6px', fontSize: 9, fontWeight: 500, borderRadius: themeKey === 'brutalist' ? 0 : 3, background: styles.card, border: themeKey === 'brutalist' ? `1.5px solid ${styles.border}` : 'none' }}>Cancel</span>
      </div>
      <div style={{ fontSize: 8, opacity: 0.8 }}>Good morning, Florian</div>
      <svg viewBox="0 0 100 20" style={{ width: '100%', height: 18 }}>
        <path d="M0 15 Q 20 10, 35 12 T 70 6 T 100 8" fill="none" stroke={styles.accent} strokeWidth="1.5" strokeLinecap="round"/>
      </svg>
    </div>
  );
}

window.ThemeShowcase = ThemeShowcase;
