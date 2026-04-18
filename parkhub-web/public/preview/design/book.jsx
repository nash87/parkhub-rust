// Book a spot — 3 step flow: Lot → Slot+Time → Confirm
const { useState: useStateB } = React;

function Stepper({ step }) {
  const steps = [
    { n: 1, label: 'Choose lot', icon: 'map-pin' },
    { n: 2, label: 'Select slot & time', icon: 'grid' },
    { n: 3, label: 'Confirm', icon: 'check' },
  ];
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 24 }}>
      {steps.map((s, i) => {
        const isDone = step > s.n;
        const isActive = step === s.n;
        return (
          <React.Fragment key={s.n}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
              <div style={{
                width: 32, height: 32, borderRadius: '50%',
                background: isDone ? 'var(--color-primary-600)'
                  : isActive ? 'color-mix(in oklch, var(--color-primary-500) 15%, transparent)'
                  : 'var(--theme-bg-muted)',
                color: isDone ? '#fff'
                  : isActive ? 'var(--color-primary-700)'
                  : 'var(--theme-text-faint)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                fontSize: 13, fontWeight: 700,
                border: isActive ? '2px solid var(--color-primary-500)' : '2px solid transparent',
                transition: 'all 200ms',
              }}>
                {isDone ? <Icon name="check" size={16} weight={2.5} /> : s.n}
              </div>
              <div>
                <div style={{ fontSize: 11, color: 'var(--theme-text-faint)', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                  Step {s.n}
                </div>
                <div style={{ fontSize: 13, fontWeight: 600, color: isActive || isDone ? 'var(--theme-text)' : 'var(--theme-text-muted)' }}>
                  {s.label}
                </div>
              </div>
            </div>
            {i < steps.length - 1 && (
              <div style={{
                flex: 1, height: 2, margin: '0 8px',
                background: step > s.n ? 'var(--color-primary-500)' : 'var(--theme-border)',
                transition: 'background 300ms',
              }}/>
            )}
          </React.Fragment>
        );
      })}
    </div>
  );
}

function LotCard({ lot, selected, onSelect }) {
  const pct = Math.round((lot.occupied / lot.total) * 100);
  const full = lot.occupied === lot.total;
  return (
    <button onClick={onSelect} disabled={full} style={{
      textAlign: 'left', padding: 18, borderRadius: 14, border: '2px solid',
      borderColor: selected ? 'var(--color-primary-500)' : 'var(--theme-border)',
      background: selected ? 'color-mix(in oklch, var(--color-primary-500) 6%, var(--theme-card-bg))' : 'var(--theme-card-bg)',
      display: 'flex', flexDirection: 'column', gap: 12,
      opacity: full ? 0.55 : 1, cursor: full ? 'not-allowed' : 'pointer',
      transition: 'all 150ms', position: 'relative', width: '100%',
      boxShadow: selected ? 'var(--shadow-sm)' : 'var(--shadow-xs)',
    }} onMouseEnter={e => { if (!full && !selected) e.currentTarget.style.borderColor = 'var(--color-primary-300)'; }}
       onMouseLeave={e => { if (!full && !selected) e.currentTarget.style.borderColor = 'var(--theme-border)'; }}>
      {selected && (
        <span style={{
          position: 'absolute', top: 12, right: 12,
          width: 22, height: 22, borderRadius: '50%', background: 'var(--color-primary-600)',
          color: '#fff', display: 'flex', alignItems: 'center', justifyContent: 'center',
        }}>
          <Icon name="check" size={12} weight={3}/>
        </span>
      )}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
        <div style={{
          width: 44, height: 44, borderRadius: 10,
          background: `color-mix(in oklch, var(--color-primary-500) 10%, transparent)`,
          color: 'var(--color-primary-700)',
          display: 'flex', alignItems: 'center', justifyContent: 'center',
        }}>
          <Icon name="map-pin" size={20} />
        </div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <div style={{ fontSize: 15, fontWeight: 700, letterSpacing: '-0.01em' }}>{lot.name}</div>
          <div style={{ fontSize: 12, color: 'var(--theme-text-muted)', marginTop: 2 }}>{lot.address}</div>
        </div>
      </div>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
        {lot.features.map((f) => (
          <span key={f.label} style={{
            display: 'inline-flex', alignItems: 'center', gap: 4,
            padding: '3px 8px', borderRadius: 999, fontSize: 11, fontWeight: 500,
            background: 'var(--theme-bg-muted)', color: 'var(--theme-text-muted)',
          }}>
            <Icon name={f.icon} size={11}/>{f.label}
          </span>
        ))}
      </div>
      <div>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 4 }}>
          <span style={{ fontSize: 11, color: 'var(--theme-text-muted)', fontWeight: 600 }}>Availability</span>
          <span style={{ fontSize: 12, fontWeight: 700, fontVariantNumeric: 'tabular-nums', color: full ? 'var(--color-danger)' : 'var(--theme-text)' }}>
            {lot.total - lot.occupied} / {lot.total} free
          </span>
        </div>
        <div style={{ height: 6, borderRadius: 3, background: 'var(--theme-bg-muted)', overflow: 'hidden' }}>
          <div style={{
            height: '100%', width: `${pct}%`,
            background: pct > 90 ? 'var(--color-danger)' : pct > 70 ? 'var(--color-warning)' : 'var(--color-primary-500)',
            transition: 'width 300ms',
          }}/>
        </div>
      </div>
    </button>
  );
}

function SlotGrid({ slots, selected, onSelect }) {
  return (
    <div style={{
      display: 'grid', gap: 6,
      gridTemplateColumns: 'repeat(10, minmax(0,1fr))',
    }}>
      {slots.map((s) => {
        const disabled = s.status === 'booked' || s.status === 'blocked';
        const isSel = selected === s.id;
        const bg = isSel ? 'var(--color-primary-600)'
          : s.status === 'free' ? 'var(--theme-bg-muted)'
          : s.status === 'booked' ? 'color-mix(in oklch, var(--color-danger) 12%, transparent)'
          : s.status === 'ev' ? 'color-mix(in oklch, var(--color-success) 12%, transparent)'
          : s.status === 'accessible' ? 'color-mix(in oklch, var(--color-info) 12%, transparent)'
          : 'var(--theme-bg-muted)';
        const col = isSel ? '#fff'
          : s.status === 'free' ? 'var(--theme-text)'
          : s.status === 'booked' ? 'var(--color-danger)'
          : s.status === 'ev' ? 'var(--color-success)'
          : s.status === 'accessible' ? 'var(--color-info)'
          : 'var(--theme-text-faint)';
        return (
          <button key={s.id} onClick={() => !disabled && onSelect(s.id)} disabled={disabled}
            style={{
              aspectRatio: '1/1.2', borderRadius: 8, background: bg, color: col,
              fontSize: 11, fontWeight: 700, fontVariantNumeric: 'tabular-nums',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              border: isSel ? '2px solid var(--color-primary-700)' : '1px solid transparent',
              cursor: disabled ? 'not-allowed' : 'pointer',
              transition: 'all 100ms', position: 'relative',
              boxShadow: isSel ? 'var(--shadow-sm)' : 'none',
            }}>
            {s.id}
            {s.status === 'ev' && <span style={{ position: 'absolute', top: 2, right: 2, fontSize: 8 }}>⚡</span>}
            {s.status === 'accessible' && <span style={{ position: 'absolute', top: 2, right: 2, fontSize: 8 }}>♿</span>}
          </button>
        );
      })}
    </div>
  );
}

function TimePresets({ value, onChange }) {
  const opts = [
    { v: '2h', label: '2h', sub: '3 credits' },
    { v: '4h', label: '4h', sub: '5 credits' },
    { v: '8h', label: 'Full day', sub: '8 credits', popular: true },
    { v: 'week', label: 'This week', sub: '32 credits' },
  ];
  return (
    <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4,1fr)', gap: 8 }}>
      {opts.map((o) => (
        <button key={o.v} onClick={() => onChange(o.v)} style={{
          padding: '12px 10px', borderRadius: 10,
          border: '2px solid', borderColor: value === o.v ? 'var(--color-primary-500)' : 'var(--theme-border)',
          background: value === o.v ? 'color-mix(in oklch, var(--color-primary-500) 8%, var(--theme-card-bg))' : 'var(--theme-card-bg)',
          display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4,
          position: 'relative',
        }}>
          {o.popular && (
            <span style={{
              position: 'absolute', top: -8, right: 8,
              padding: '2px 6px', fontSize: 9, fontWeight: 700,
              background: 'var(--color-accent-500)', color: '#fff',
              borderRadius: 4, textTransform: 'uppercase', letterSpacing: '0.05em',
            }}>Popular</span>
          )}
          <div style={{ fontSize: 15, fontWeight: 700 }}>{o.label}</div>
          <div style={{ fontSize: 11, color: 'var(--theme-text-muted)', fontWeight: 500 }}>{o.sub}</div>
        </button>
      ))}
    </div>
  );
}

function Book() {
  const [step, setStep] = useStateB(1);
  const [lotId, setLotId] = useStateB('hq');
  const [slotId, setSlotId] = useStateB(null);
  const [duration, setDuration] = useStateB('8h');

  const lots = [
    { id: 'hq', name: 'HQ Garage', address: 'Maximilianstraße 12 · Munich', total: 48, occupied: 32, features: [
      {icon:'lightning',label:'12 EV'},{icon:'wheelchair',label:'Accessible'},{icon:'shield',label:'Covered'},
    ]},
    { id: 'annex-n', name: 'Annex North', address: 'Leopoldstraße 45 · Munich', total: 24, occupied: 18, features: [
      {icon:'lightning',label:'4 EV'},{icon:'sun',label:'Open-air'},
    ]},
    { id: 'annex-s', name: 'Annex South', address: 'Sendlinger Str. 8 · Munich', total: 16, occupied: 16, features: [
      {icon:'shield',label:'Covered'},{icon:'motorcycle',label:'Motorcycle'},
    ]},
    { id: 'campus', name: 'Campus East', address: 'Arnulfstraße 200 · Munich', total: 60, occupied: 22, features: [
      {icon:'lightning',label:'20 EV'},{icon:'wheelchair',label:'Accessible'},{icon:'star',label:'Premium'},
    ]},
  ];

  const selectedLot = lots.find((l) => l.id === lotId);

  // Generate 48 slots for the selected lot
  const slots = React.useMemo(() => {
    const status = (i) => {
      if ([2,5,9,13,15,21,29,33,41].includes(i)) return 'booked';
      if ([7,19,35].includes(i)) return 'ev';
      if ([11,27].includes(i)) return 'accessible';
      if ([24].includes(i)) return 'blocked';
      return 'free';
    };
    return Array.from({length:40}, (_,i) => ({
      id: `L2-${String(i+1).padStart(2,'0')}`,
      status: status(i),
    }));
  }, [lotId]);

  return (
    <div style={{ padding: 28, maxWidth: 1200, margin: '0 auto' }}>
      <Stepper step={step} />

      {step === 1 && (
        <div>
          <div style={{ display: 'flex', alignItems: 'baseline', justifyContent: 'space-between', marginBottom: 14 }}>
            <h2 style={{ fontSize: 18, fontWeight: 700, letterSpacing: '-0.02em' }}>Where are you parking?</h2>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <span style={{ fontSize: 12, color: 'var(--theme-text-muted)' }}>Sort:</span>
              <button className="btn btn-ghost btn-sm">Nearest <Icon name="chevron" size={12}/></button>
            </div>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2,1fr)', gap: 12 }}>
            {lots.map((l) => (
              <LotCard key={l.id} lot={l} selected={lotId === l.id} onSelect={() => setLotId(l.id)} />
            ))}
          </div>
        </div>
      )}

      {step === 2 && (
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 320px', gap: 20 }}>
          <div>
            <h2 style={{ fontSize: 18, fontWeight: 700, letterSpacing: '-0.02em', marginBottom: 4 }}>
              Pick your slot in {selectedLot.name}
            </h2>
            <p style={{ fontSize: 13, color: 'var(--theme-text-muted)', marginBottom: 16 }}>
              Level 2 · {slots.filter(s=>s.status==='free').length} free of {slots.length}
            </p>

            <div className="card" style={{ padding: 20 }}>
              {/* Entry arrow */}
              <div style={{
                display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 10,
                padding: '8px', marginBottom: 16, borderRadius: 8,
                background: 'var(--theme-bg-muted)', fontSize: 11,
                fontWeight: 600, color: 'var(--theme-text-muted)', textTransform: 'uppercase', letterSpacing: '0.05em',
              }}>
                <Icon name="arrow" size={14} weight={2} /> Entry from Maximilianstraße
              </div>

              <SlotGrid slots={slots} selected={slotId} onSelect={setSlotId} />

              <div style={{
                marginTop: 18, padding: '12px 16px', borderRadius: 8, background: 'var(--theme-bg-muted)',
                display: 'flex', flexWrap: 'wrap', gap: 14, fontSize: 11,
              }}>
                {[
                  {c:'var(--theme-bg-muted)', l:'Available', br:'1px solid var(--theme-border)'},
                  {c:'var(--color-primary-600)', l:'Selected'},
                  {c:'color-mix(in oklch, var(--color-danger) 40%, transparent)', l:'Booked'},
                  {c:'color-mix(in oklch, var(--color-success) 40%, transparent)', l:'EV charging'},
                  {c:'color-mix(in oklch, var(--color-info) 40%, transparent)', l:'Accessible'},
                ].map((leg) => (
                  <span key={leg.l} style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
                    <span style={{ width: 12, height: 12, borderRadius: 3, background: leg.c, border: leg.br || 'none' }}/>
                    {leg.l}
                  </span>
                ))}
              </div>
            </div>
          </div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
            <div className="card" style={{ padding: 18 }}>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 10 }}>Duration</h3>
              <TimePresets value={duration} onChange={setDuration} />
            </div>

            <div className="card" style={{ padding: 18 }}>
              <h3 style={{ fontSize: 14, fontWeight: 600, marginBottom: 10 }}>Vehicle</h3>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                {['BMW i4 · M-PH 2341','Tesla Model Y · M-EV 107','VW ID.3 · M-VW 4421'].map((v, i) => (
                  <label key={v} style={{
                    display: 'flex', alignItems: 'center', gap: 10, padding: '8px 10px',
                    borderRadius: 8, background: i===0 ? 'color-mix(in oklch, var(--color-primary-500) 8%, transparent)' : 'var(--theme-bg-muted)',
                    fontSize: 13, fontWeight: 500, cursor: 'pointer',
                  }}>
                    <input type="radio" name="vehicle" defaultChecked={i===0} style={{ accentColor: 'var(--color-primary-600)' }} />
                    <Icon name="car" size={16} />
                    <span style={{ flex: 1 }}>{v}</span>
                  </label>
                ))}
              </div>
            </div>

            <div className="card" style={{ padding: 18, background: 'color-mix(in oklch, var(--color-primary-500) 6%, var(--theme-card-bg))', border: '1px dashed var(--color-primary-400)' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 10 }}>
                <Icon name="sparkle" size={16} style={{ color: 'var(--color-primary-600)' }}/>
                <h3 style={{ fontSize: 13, fontWeight: 700, color: 'var(--color-primary-700)' }}>Smart suggestion</h3>
              </div>
              <p style={{ fontSize: 12, color: 'var(--theme-text-muted)', lineHeight: 1.45 }}>
                Slot <strong style={{ color: 'var(--theme-text)' }}>L2-14</strong> is closest to the main entrance and covered. Based on your last 12 bookings.
              </p>
            </div>
          </div>
        </div>
      )}

      {step === 3 && (
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 400px', gap: 20 }}>
          <div className="card" style={{ padding: 24 }}>
            <h2 style={{ fontSize: 18, fontWeight: 700, letterSpacing: '-0.02em', marginBottom: 4 }}>Confirm your booking</h2>
            <p style={{ fontSize: 13, color: 'var(--theme-text-muted)', marginBottom: 20 }}>Review the details — you'll get a QR pass instantly.</p>

            {[
              { label: 'Location', value: selectedLot.name, sub: selectedLot.address, icon: 'map-pin' },
              { label: 'Slot', value: slotId || 'L2-14', sub: 'Level 2 · Covered · Near entrance', icon: 'grid' },
              { label: 'Date & time', value: 'Today · 08:00 – 16:00', sub: 'Full day (8 hours)', icon: 'clock' },
              { label: 'Vehicle', value: 'BMW i4', sub: 'M-PH 2341', icon: 'car' },
            ].map((r) => (
              <div key={r.label} style={{
                display: 'flex', alignItems: 'center', gap: 14,
                padding: '14px 0', borderBottom: '1px solid var(--theme-border-subtle)',
              }}>
                <div style={{
                  width: 36, height: 36, borderRadius: 10,
                  background: 'color-mix(in oklch, var(--color-primary-500) 10%, transparent)',
                  color: 'var(--color-primary-700)',
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                }}>
                  <Icon name={r.icon} size={16} />
                </div>
                <div style={{ flex: 1 }}>
                  <div style={{ fontSize: 11, color: 'var(--theme-text-muted)', fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
                    {r.label}
                  </div>
                  <div style={{ fontSize: 14, fontWeight: 600, marginTop: 2 }}>{r.value}</div>
                  <div style={{ fontSize: 12, color: 'var(--theme-text-muted)' }}>{r.sub}</div>
                </div>
                <button className="btn btn-ghost btn-sm">Edit</button>
              </div>
            ))}

            <label style={{
              display: 'flex', alignItems: 'center', gap: 10, marginTop: 20,
              padding: 12, borderRadius: 10, background: 'var(--theme-bg-muted)',
              fontSize: 13, cursor: 'pointer',
            }}>
              <input type="checkbox" style={{ accentColor: 'var(--color-primary-600)' }} />
              Share pass with team · makes swap requests possible
            </label>
          </div>

          <div className="card" style={{
            padding: 24,
            background: 'linear-gradient(160deg, color-mix(in oklch, var(--color-primary-500) 8%, var(--theme-card-bg)), var(--theme-card-bg))',
            height: 'fit-content',
          }}>
            <h3 style={{ fontSize: 14, fontWeight: 700, marginBottom: 14, textTransform: 'uppercase', letterSpacing: '0.05em', color: 'var(--theme-text-muted)' }}>
              Cost summary
            </h3>
            {[
              ['Full-day slot', '8 credits'],
              ['Covered surcharge', '0 credits'],
              ['Loyalty discount', '−1 credit'],
            ].map(([l, v]) => (
              <div key={l} style={{ display: 'flex', justifyContent: 'space-between', padding: '6px 0', fontSize: 13, color: 'var(--theme-text-muted)' }}>
                <span>{l}</span><span style={{ fontVariantNumeric: 'tabular-nums' }}>{v}</span>
              </div>
            ))}
            <div style={{
              marginTop: 12, paddingTop: 12, borderTop: '1px solid var(--theme-border)',
              display: 'flex', justifyContent: 'space-between', alignItems: 'baseline',
            }}>
              <span style={{ fontSize: 14, fontWeight: 600 }}>Total</span>
              <span style={{ fontSize: 24, fontWeight: 800, fontVariantNumeric: 'tabular-nums', letterSpacing: '-0.025em' }}>7 credits</span>
            </div>
            <div style={{ marginTop: 4, fontSize: 11, color: 'var(--theme-text-muted)', textAlign: 'right' }}>
              You have 45 credits left · ≈ €14 retail
            </div>
            <button className="btn btn-primary" style={{ width: '100%', marginTop: 16, padding: '12px 16px', fontSize: 14 }}>
              <Icon name="check" size={16} weight={2.5} /> Confirm booking
            </button>
            <p style={{ fontSize: 11, color: 'var(--theme-text-muted)', textAlign: 'center', marginTop: 10 }}>
              Free cancellation up to 15 min before start
            </p>
          </div>
        </div>
      )}

      {/* Nav */}
      <div style={{ marginTop: 24, display: 'flex', justifyContent: 'space-between', gap: 10 }}>
        <button className="btn btn-secondary" disabled={step === 1} onClick={() => setStep(s => Math.max(1, s-1))}>
          <Icon name="back" size={14}/> Back
        </button>
        {step < 3 ? (
          <button className="btn btn-primary" onClick={() => setStep(s => Math.min(3, s+1))}>
            Continue <Icon name="arrow" size={14} weight={2.5}/>
          </button>
        ) : (
          <button className="btn btn-ghost" onClick={() => setStep(1)}>
            <Icon name="arrow" size={14} weight={2.5} style={{ transform: 'rotate(180deg)' }}/> Start over
          </button>
        )}
      </div>
    </div>
  );
}

window.Book = Book;
