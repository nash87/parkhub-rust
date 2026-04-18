// Marketing landing page — refresh of infracraft-demos/index.html
function Landing() {
  return (
    <div style={{ minHeight: '100%' }}>
      {/* Top nav */}
      <header style={{
        position: 'sticky', top: 0, zIndex: 10,
        backdropFilter: 'blur(12px)',
        background: 'color-mix(in oklch, var(--theme-bg) 80%, transparent)',
        borderBottom: '1px solid var(--theme-border-subtle)',
        padding: '14px 48px',
        display: 'flex', alignItems: 'center', gap: 24,
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
          <ParkHubLogo size={28}/>
          <div style={{ fontWeight: 800, fontSize: 17, letterSpacing: '-0.025em' }}>ParkHub</div>
          <span className="badge badge-gray" style={{ fontSize: 10, fontWeight: 600 }}>OSS · self-host</span>
        </div>
        <nav style={{ display: 'flex', gap: 20, fontSize: 13, fontWeight: 500, color: 'var(--theme-text-muted)' }}>
          {['Features','Editions','Demos','Docs','Pricing'].map((l) => (
            <a key={l} href="#" style={{ color: 'inherit', textDecoration: 'none' }}>{l}</a>
          ))}
        </nav>
        <div style={{ flex: 1 }}/>
        <a href="#" className="btn btn-ghost btn-sm"><Icon name="github" size={14}/> Star on GitHub</a>
        <a href="#" className="btn btn-primary btn-sm">Try demo <Icon name="arrow" size={12}/></a>
      </header>

      {/* Hero */}
      <section style={{
        position: 'relative', padding: '72px 48px 60px',
        display: 'grid', gridTemplateColumns: '1.2fr 1fr', gap: 48, alignItems: 'center',
        overflow: 'hidden',
      }}>
        {/* bg pattern */}
        <div aria-hidden style={{
          position: 'absolute', inset: 0, zIndex: 0, opacity: 0.4,
          background: 'radial-gradient(circle at 15% 20%, color-mix(in oklch, var(--color-primary-500) 15%, transparent), transparent 40%), radial-gradient(circle at 85% 80%, color-mix(in oklch, var(--color-accent-500) 10%, transparent), transparent 40%)',
          pointerEvents: 'none',
        }}/>

        <div style={{ position: 'relative', zIndex: 1 }}>
          <span style={{
            display: 'inline-flex', alignItems: 'center', gap: 6,
            padding: '4px 10px', borderRadius: 999,
            background: 'color-mix(in oklch, var(--color-primary-500) 10%, transparent)',
            color: 'var(--color-primary-700)', fontSize: 11, fontWeight: 700,
            textTransform: 'uppercase', letterSpacing: '0.06em', marginBottom: 18,
          }}>
            <span className="pulse-dot" style={{ width: 6, height: 6, borderRadius: '50%', background: 'var(--color-primary-500)' }}/>
            v4.13.0 released · Rust + PHP
          </span>
          <h1 style={{
            fontSize: 56, fontWeight: 800, lineHeight: 1.02,
            letterSpacing: '-0.035em', textWrap: 'balance',
          }}>
            Self-hosted parking management,{' '}
            <span style={{
              background: 'linear-gradient(120deg, var(--color-primary-600), var(--color-accent-600))',
              WebkitBackgroundClip: 'text', backgroundClip: 'text', color: 'transparent',
            }}>built to last.</span>
          </h1>
          <p style={{
            fontSize: 17, color: 'var(--theme-text-muted)', lineHeight: 1.55,
            marginTop: 16, maxWidth: 560, textWrap: 'pretty',
          }}>
            Two editions — <strong style={{ color: 'var(--theme-text)' }}>Rust</strong> for minimal ops and a single binary,{' '}
            <strong style={{ color: 'var(--theme-text)' }}>PHP</strong> for shared hosting and Laravel shops. Same React frontend, same features, GDPR compliant, MIT licensed.
          </p>
          <div style={{ display: 'flex', gap: 10, marginTop: 26, flexWrap: 'wrap' }}>
            <a href="#" className="btn btn-primary" style={{ padding: '12px 18px', fontSize: 14 }}>
              <Icon name="rocket" size={16}/> Try the live demo
            </a>
            <a href="#" className="btn btn-secondary" style={{ padding: '12px 18px', fontSize: 14 }}>
              <Icon name="download" size={16}/> Install in 5 minutes
            </a>
          </div>
          <div style={{ marginTop: 28, display: 'flex', gap: 32, fontSize: 12, color: 'var(--theme-text-muted)', flexWrap: 'wrap' }}>
            {[
              {i: 'shield', l: 'GDPR Art. 15/17 compliant'},
              {i: 'key', l: '2FA + WebAuthn + SSO'},
              {i: 'globe', l: '10 languages'},
              {i: 'sparkle', l: '12 themes'},
            ].map((x) => (
              <span key={x.l} style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
                <Icon name={x.i} size={13}/>{x.l}
              </span>
            ))}
          </div>
        </div>

        {/* Product preview */}
        <div style={{ position: 'relative', zIndex: 1, transform: 'perspective(1200px) rotateY(-8deg) rotateX(4deg)' }}>
          <div style={{
            borderRadius: 16, overflow: 'hidden', boxShadow: '0 40px 80px -20px rgba(0,0,0,0.3), 0 0 0 1px var(--theme-border)',
            background: 'var(--theme-card-bg)',
          }}>
            {/* fake chrome */}
            <div style={{ padding: '8px 12px', background: 'var(--theme-bg-muted)', borderBottom: '1px solid var(--theme-border)', display: 'flex', gap: 6 }}>
              {['#ef4444','#eab308','#22c55e'].map((c,i) => (
                <span key={i} style={{ width: 10, height: 10, borderRadius: '50%', background: c }}/>
              ))}
              <div style={{ flex: 1, textAlign: 'center', fontSize: 10, color: 'var(--theme-text-muted)', fontFamily: 'ui-monospace, Menlo, monospace' }}>
                demo.parkhub.app/dashboard
              </div>
            </div>
            {/* mini dashboard */}
            <div style={{ padding: 18 }}>
              <div style={{ fontSize: 15, fontWeight: 700, marginBottom: 10 }}>Good morning, Florian</div>
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3,1fr)', gap: 8, marginBottom: 12 }}>
                {[['Active','3','chart-line'],['Credits','45','coins'],['CO₂','12.4kg','leaf']].map(([l,v,i]) => (
                  <div key={l} style={{ padding: 10, borderRadius: 8, background: 'var(--theme-bg-muted)' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 10, color: 'var(--theme-text-muted)' }}>
                      <Icon name={i} size={11}/>{l}
                    </div>
                    <div style={{ fontSize: 17, fontWeight: 800, marginTop: 3, fontVariantNumeric: 'tabular-nums' }}>{v}</div>
                  </div>
                ))}
              </div>
              <div style={{ padding: 10, borderRadius: 8, background: 'color-mix(in oklch, var(--color-primary-500) 6%, transparent)' }}>
                <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 8 }}>Weekly activity</div>
                <svg viewBox="0 0 200 40" style={{ width: '100%', height: 34 }}>
                  <path d="M0 30 Q 30 20, 50 22 T 100 15 T 150 8 T 200 12" fill="none" stroke="var(--color-primary-600)" strokeWidth="2" strokeLinecap="round"/>
                </svg>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Editions compare */}
      <section style={{ padding: '48px 48px 24px' }}>
        <div style={{ textAlign: 'center', maxWidth: 620, margin: '0 auto 36px' }}>
          <h2 style={{ fontSize: 32, fontWeight: 800, letterSpacing: '-0.03em' }}>Two editions. Same product.</h2>
          <p style={{ fontSize: 15, color: 'var(--theme-text-muted)', marginTop: 8, textWrap: 'pretty' }}>
            Pick the stack that fits your ops. The frontend, features, and data model are identical.
          </p>
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 20, maxWidth: 1100, margin: '0 auto' }}>
          {[
            { name: 'Rust edition', tag: 'Axum · embedded redb', desc: 'Single binary, no database to run. 12 MB container. Deploy on a $5 VPS.',
              pros: ['Single static binary','Embedded redb DB','Sub-millisecond latency','Lowest ops footprint'],
              repo: 'nash87/parkhub-rust', accent: 'primary', icon: 'lightning' },
            { name: 'PHP edition', tag: 'Laravel 13 · MySQL/SQLite', desc: 'Runs on any shared hosting with PHP 8.4. Familiar stack, familiar ops.',
              pros: ['Works on shared hosting','MySQL or SQLite','Laravel 13 ecosystem','Composer install'],
              repo: 'nash87/parkhub-php', accent: 'accent', icon: 'server' },
          ].map((ed) => (
            <div key={ed.name} className="card" style={{ padding: 28, position: 'relative', overflow: 'hidden' }}>
              <div aria-hidden style={{
                position: 'absolute', top: -40, right: -40, width: 180, height: 180, borderRadius: '50%',
                background: `color-mix(in oklch, var(--color-${ed.accent}-500) 8%, transparent)`,
              }}/>
              <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 14, position: 'relative' }}>
                <div style={{
                  width: 44, height: 44, borderRadius: 11,
                  background: `color-mix(in oklch, var(--color-${ed.accent}-500) 15%, transparent)`,
                  color: `var(--color-${ed.accent}-700)`,
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                }}>
                  <Icon name={ed.icon} size={22} weight={2}/>
                </div>
                <div>
                  <div style={{ fontSize: 18, fontWeight: 800, letterSpacing: '-0.02em' }}>{ed.name}</div>
                  <div style={{ fontSize: 12, color: 'var(--theme-text-muted)', fontFamily: 'ui-monospace, Menlo, monospace' }}>{ed.tag}</div>
                </div>
              </div>
              <p style={{ fontSize: 13, color: 'var(--theme-text-muted)', lineHeight: 1.5, marginBottom: 14 }}>{ed.desc}</p>
              <ul style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', flexDirection: 'column', gap: 6 }}>
                {ed.pros.map((p) => (
                  <li key={p} style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13 }}>
                    <Icon name="check" size={14} style={{ color: `var(--color-${ed.accent}-600)` }} weight={2.5}/>{p}
                  </li>
                ))}
              </ul>
              <div style={{ display: 'flex', gap: 8, marginTop: 18 }}>
                <a href="#" className="btn btn-primary btn-sm">Live demo</a>
                <a href="#" className="btn btn-secondary btn-sm"><Icon name="github" size={13}/> {ed.repo}</a>
              </div>
            </div>
          ))}
        </div>
      </section>

      {/* Features */}
      <section style={{ padding: '48px 48px', background: 'var(--theme-bg-muted)' }}>
        <div style={{ textAlign: 'center', marginBottom: 32 }}>
          <h2 style={{ fontSize: 28, fontWeight: 800, letterSpacing: '-0.03em' }}>Everything you need. Out of the box.</h2>
          <p style={{ fontSize: 14, color: 'var(--theme-text-muted)', marginTop: 6 }}>No addons, no paywalls, no telemetry.</p>
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: 12, maxWidth: 1100, margin: '0 auto' }}>
          {[
            {i:'qr', t:'QR check-in', d:'Sub-second scan. Offline-capable. Apple Wallet export.'},
            {i:'swap', t:'Swap requests', d:'Users trade slots with automatic credit balancing.'},
            {i:'users', t:'Guest passes', d:'Time-limited QR codes for visitors, contractors, deliveries.'},
            {i:'shield', t:'2FA + WebAuthn', d:'Passkeys, TOTP, and hardware keys. Argon2id at rest.'},
            {i:'calendar', t:'iCal sync', d:'Push to Google, Apple, Outlook. Import from any source.'},
            {i:'bell', t:'Push + webhooks', d:'Web Push, email, Slack, Discord — or your own HTTP endpoint.'},
            {i:'chart-line', t:'Analytics + CSV', d:'Occupancy, utilization, lots, users. All exportable.'},
            {i:'globe', t:'10 languages', d:'EN, DE, FR, ES, IT, JA, PL, PT, TR, ZH — all first-class.'},
            {i:'sparkle', t:'12 themes', d:'Classic, Glass, Bento, Brutalist, Neon, Warm, Mono…'},
            {i:'leaf', t:'CO₂ tracking', d:'Per-user impact. Commutes avoided. Shareable summaries.'},
            {i:'key', t:'SSO', d:'OAuth 2.0, SAML, OIDC with provider-agnostic profiles.'},
            {i:'database', t:'GDPR tools', d:'Art. 15 export, Art. 17 erase — one click per user.'},
          ].map((f) => (
            <div key={f.t} style={{
              padding: 16, borderRadius: 12, background: 'var(--theme-card-bg)',
              border: '1px solid var(--theme-border-subtle)',
            }}>
              <div style={{
                width: 32, height: 32, borderRadius: 8,
                background: 'color-mix(in oklch, var(--color-primary-500) 10%, transparent)',
                color: 'var(--color-primary-700)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                marginBottom: 10,
              }}>
                <Icon name={f.i} size={16} weight={2}/>
              </div>
              <div style={{ fontSize: 14, fontWeight: 700, marginBottom: 4 }}>{f.t}</div>
              <div style={{ fontSize: 12, color: 'var(--theme-text-muted)', lineHeight: 1.45, textWrap: 'pretty' }}>{f.d}</div>
            </div>
          ))}
        </div>
      </section>

      {/* Architecture */}
      <section style={{ padding: '48px 48px' }}>
        <div style={{ textAlign: 'center', marginBottom: 32 }}>
          <h2 style={{ fontSize: 28, fontWeight: 800, letterSpacing: '-0.03em' }}>One frontend, two backends.</h2>
        </div>
        <div style={{ maxWidth: 900, margin: '0 auto', padding: 28, borderRadius: 16, background: 'var(--theme-card-bg)', border: '1px solid var(--theme-border)' }}>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr auto 1fr auto 1fr', alignItems: 'center', gap: 18 }}>
            <div style={{ textAlign: 'center' }}>
              <div style={{ padding: 18, borderRadius: 12, background: 'color-mix(in oklch, var(--color-primary-500) 8%, transparent)', border: '1px dashed var(--color-primary-400)' }}>
                <Icon name="code" size={28} style={{ color: 'var(--color-primary-700)' }}/>
                <div style={{ fontSize: 13, fontWeight: 700, marginTop: 8 }}>parkhub-web</div>
                <div style={{ fontSize: 11, color: 'var(--theme-text-muted)' }}>Astro 6 + React 19 + Tailwind 4</div>
              </div>
            </div>
            <Icon name="arrow" size={22} style={{ color: 'var(--theme-text-muted)' }}/>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 10, alignItems: 'center' }}>
              <div style={{ padding: '10px 14px', borderRadius: 8, background: 'var(--theme-bg-muted)', fontSize: 12, fontWeight: 600, fontFamily: 'ui-monospace, Menlo, monospace' }}>
                REST + WebSocket
              </div>
              <div style={{ fontSize: 11, color: 'var(--theme-text-muted)' }}>identical contract</div>
            </div>
            <Icon name="arrow" size={22} style={{ color: 'var(--theme-text-muted)' }}/>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              <div style={{ padding: 14, borderRadius: 10, background: 'color-mix(in oklch, var(--color-primary-500) 6%, transparent)', textAlign: 'center' }}>
                <div style={{ fontSize: 13, fontWeight: 700 }}>Rust backend</div>
                <div style={{ fontSize: 11, color: 'var(--theme-text-muted)' }}>Axum · redb</div>
              </div>
              <div style={{ padding: 14, borderRadius: 10, background: 'color-mix(in oklch, var(--color-accent-500) 8%, transparent)', textAlign: 'center' }}>
                <div style={{ fontSize: 13, fontWeight: 700 }}>PHP backend</div>
                <div style={{ fontSize: 11, color: 'var(--theme-text-muted)' }}>Laravel 13 · MySQL</div>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* CTA */}
      <section style={{ padding: '48px 48px 72px', textAlign: 'center' }}>
        <h2 style={{ fontSize: 32, fontWeight: 800, letterSpacing: '-0.03em' }}>Ship parking, not infrastructure.</h2>
        <p style={{ fontSize: 15, color: 'var(--theme-text-muted)', marginTop: 8, maxWidth: 540, margin: '8px auto 24px' }}>
          MIT licensed. No vendor lock-in. Install on your own server in 5 minutes.
        </p>
        <div style={{ display: 'inline-flex', gap: 10 }}>
          <a href="#" className="btn btn-primary" style={{ padding: '12px 20px' }}>
            <Icon name="rocket" size={16}/> Try the demo
          </a>
          <a href="#" className="btn btn-secondary" style={{ padding: '12px 20px' }}>
            <Icon name="github" size={16}/> Clone the repo
          </a>
        </div>
      </section>

      <footer style={{ padding: '24px 48px', borderTop: '1px solid var(--theme-border-subtle)', fontSize: 12, color: 'var(--theme-text-muted)', display: 'flex', justifyContent: 'space-between', flexWrap: 'wrap', gap: 12 }}>
        <div>© 2026 ParkHub · MIT License · Built by <a href="https://github.com/nash87" style={{ color: 'inherit' }}>Florian</a></div>
        <div style={{ display: 'flex', gap: 16 }}>
          <a href="#" style={{ color: 'inherit' }}>Impressum</a>
          <a href="#" style={{ color: 'inherit' }}>Datenschutz</a>
          <a href="#" style={{ color: 'inherit' }}>Status</a>
        </div>
      </footer>
    </div>
  );
}

window.Landing = Landing;
