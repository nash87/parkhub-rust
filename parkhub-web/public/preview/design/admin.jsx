// Admin Lots — lot editor + slot grid
const { useState: useStateA } = React;

function AdminLots() {
  const [selectedLot, setSelectedLot] = useStateA(0);
  const [tool, setTool] = useStateA('select');

  const lots = [
    { name: 'HQ Garage', slots: 48, zones: 3, occ: 32, status: 'published' },
    { name: 'Annex North', slots: 24, zones: 2, occ: 18, status: 'published' },
    { name: 'Annex South', slots: 16, zones: 1, occ: 16, status: 'published' },
    { name: 'Campus East', slots: 60, zones: 4, occ: 22, status: 'draft' },
  ];

  // Floor plan: 2 rows of 10 slots + vertical aisle
  const renderSlot = (id, type, occupied, selected) => {
    const bg = occupied ? 'color-mix(in oklch, var(--color-danger) 20%, transparent)'
      : type === 'ev' ? 'color-mix(in oklch, var(--color-success) 15%, transparent)'
      : type === 'accessible' ? 'color-mix(in oklch, var(--color-info) 15%, transparent)'
      : type === 'motorcycle' ? 'color-mix(in oklch, var(--color-warning) 15%, transparent)'
      : 'var(--theme-bg-muted)';
    const col = occupied ? 'var(--color-danger)'
      : type === 'ev' ? 'var(--color-success)'
      : type === 'accessible' ? 'var(--color-info)'
      : type === 'motorcycle' ? 'var(--color-warning)'
      : 'var(--theme-text-muted)';
    return (
      <div key={id} style={{
        position: 'relative', borderRadius: 6, background: bg, color: col,
        display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center',
        fontSize: 10, fontWeight: 700, cursor: 'pointer',
        border: selected ? '2px solid var(--color-primary-500)' : '1px solid transparent',
        padding: 4, minHeight: 42, fontVariantNumeric: 'tabular-nums',
      }}>
        <div>{id}</div>
        {type !== 'standard' && (
          <div style={{ fontSize: 10, marginTop: 1 }}>
            {type === 'ev' && '⚡'}
            {type === 'accessible' && '♿'}
            {type === 'motorcycle' && '🏍'}
          </div>
        )}
        {occupied && (
          <span style={{ position: 'absolute', top: 3, right: 3, width: 5, height: 5, borderRadius: '50%', background: 'var(--color-danger)' }}/>
        )}
      </div>
    );
  };

  const slotType = (i) => {
    if ([6, 15].includes(i)) return 'ev';
    if (i === 10) return 'accessible';
    if (i === 19) return 'motorcycle';
    return 'standard';
  };
  const isOccupied = (i) => [1, 4, 8, 12, 13, 17, 21, 25, 28].includes(i);

  return (
    <div style={{ display: 'grid', gridTemplateColumns: '260px 1fr 320px', height: '100%', minHeight: 600 }}>
      {/* Lot list */}
      <div style={{
        borderRight: '1px solid var(--theme-border)',
        padding: 16, display: 'flex', flexDirection: 'column', gap: 4,
        background: 'var(--theme-bg-subtle)',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 10 }}>
          <h3 style={{ fontSize: 13, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.05em', color: 'var(--theme-text-muted)' }}>
            Parking lots
          </h3>
          <button className="btn btn-ghost btn-icon"><Icon name="plus" size={14} /></button>
        </div>
        {lots.map((l, i) => {
          const active = i === selectedLot;
          return (
            <button key={l.name} onClick={() => setSelectedLot(i)} style={{
              padding: 12, borderRadius: 8, textAlign: 'left',
              background: active ? 'var(--theme-card-bg)' : 'transparent',
              border: '1px solid', borderColor: active ? 'var(--color-primary-300)' : 'transparent',
              boxShadow: active ? 'var(--shadow-xs)' : 'none',
              display: 'flex', flexDirection: 'column', gap: 4,
            }} onMouseEnter={e => { if (!active) e.currentTarget.style.background = 'var(--theme-bg-muted)'; }}
               onMouseLeave={e => { if (!active) e.currentTarget.style.background = 'transparent'; }}>
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <span style={{ fontSize: 13, fontWeight: 600 }}>{l.name}</span>
                <span className={`badge badge-${l.status === 'published' ? 'success' : 'warning'}`} style={{ fontSize: 9 }}>
                  {l.status}
                </span>
              </div>
              <div style={{ display: 'flex', gap: 10, fontSize: 11, color: 'var(--theme-text-muted)', fontVariantNumeric: 'tabular-nums' }}>
                <span>{l.slots} slots</span>
                <span>·</span>
                <span>{l.zones} zones</span>
                <span>·</span>
                <span style={{ color: l.occ === l.slots ? 'var(--color-danger)' : 'inherit' }}>{l.occ}/{l.slots} occ</span>
              </div>
            </button>
          );
        })}
      </div>

      {/* Floor plan editor */}
      <div style={{ padding: 24, overflow: 'auto' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 16 }}>
          <div>
            <h2 style={{ fontSize: 18, fontWeight: 700, letterSpacing: '-0.02em' }}>{lots[selectedLot].name}</h2>
            <p style={{ fontSize: 12, color: 'var(--theme-text-muted)', marginTop: 2 }}>Level 2 · Drag to edit · {lots[selectedLot].slots} slots · {lots[selectedLot].occ} currently occupied</p>
          </div>
          <div style={{ display: 'flex', gap: 8 }}>
            <button className="btn btn-secondary btn-sm"><Icon name="eye" size={14}/> Preview</button>
            <button className="btn btn-primary btn-sm"><Icon name="check" size={14}/> Publish changes</button>
          </div>
        </div>

        {/* Toolbar */}
        <div style={{
          display: 'flex', gap: 4, padding: 4, borderRadius: 10,
          background: 'var(--theme-bg-muted)', marginBottom: 16, width: 'fit-content',
        }}>
          {[
            { k: 'select', i: 'filter', l: 'Select' },
            { k: 'add', i: 'plus', l: 'Add slot' },
            { k: 'ev', i: 'lightning', l: 'EV' },
            { k: 'accessible', i: 'wheelchair', l: 'Accessible' },
            { k: 'motorcycle', i: 'motorcycle', l: 'Motorcycle' },
            { k: 'delete', i: 'minus', l: 'Remove' },
          ].map((t) => (
            <button key={t.k} onClick={() => setTool(t.k)} style={{
              padding: '6px 12px', borderRadius: 6, fontSize: 12, fontWeight: 600,
              background: tool === t.k ? 'var(--theme-card-bg)' : 'transparent',
              color: tool === t.k ? 'var(--color-primary-700)' : 'var(--theme-text-muted)',
              boxShadow: tool === t.k ? 'var(--shadow-xs)' : 'none',
              display: 'inline-flex', alignItems: 'center', gap: 6,
            }}>
              <Icon name={t.i} size={13}/>{t.l}
            </button>
          ))}
        </div>

        {/* Floor plan */}
        <div style={{
          padding: 20, borderRadius: 14, background: 'var(--theme-bg-muted)',
          border: '2px dashed var(--theme-border)', position: 'relative',
        }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', fontSize: 11, fontWeight: 600, color: 'var(--theme-text-muted)', marginBottom: 12, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
            <span>North wall</span>
            <span>Entry ↓</span>
          </div>

          {/* Top row */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(10, 1fr)', gap: 4 }}>
            {Array.from({length: 10}, (_, i) => renderSlot(
              `L2-${String(i+1).padStart(2,'0')}`,
              slotType(i),
              isOccupied(i),
              i === 13,
            ))}
          </div>

          {/* Aisle */}
          <div style={{
            margin: '8px 0', padding: '10px', borderRadius: 6,
            background: 'color-mix(in oklch, var(--color-info) 6%, transparent)',
            textAlign: 'center', fontSize: 11, fontWeight: 600,
            color: 'var(--theme-text-muted)', textTransform: 'uppercase', letterSpacing: '0.1em',
          }}>
            ← — — — — — Aisle — — — — — →
          </div>

          {/* Middle row */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(10, 1fr)', gap: 4 }}>
            {Array.from({length: 10}, (_, i) => renderSlot(
              `L2-${String(i+11).padStart(2,'0')}`,
              slotType(i+10),
              isOccupied(i+10),
              false,
            ))}
          </div>

          {/* Aisle */}
          <div style={{
            margin: '8px 0', padding: '10px', borderRadius: 6,
            background: 'color-mix(in oklch, var(--color-info) 6%, transparent)',
            textAlign: 'center', fontSize: 11, fontWeight: 600,
            color: 'var(--theme-text-muted)', textTransform: 'uppercase', letterSpacing: '0.1em',
          }}>
            ← — — — — — Aisle — — — — — →
          </div>

          {/* Bottom row */}
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(10, 1fr)', gap: 4 }}>
            {Array.from({length: 10}, (_, i) => renderSlot(
              `L2-${String(i+21).padStart(2,'0')}`,
              slotType(i+20),
              isOccupied(i+20),
              false,
            ))}
          </div>

          <div style={{ marginTop: 12, display: 'flex', justifyContent: 'space-between', fontSize: 11, fontWeight: 600, color: 'var(--theme-text-muted)', textTransform: 'uppercase', letterSpacing: '0.05em' }}>
            <span>South wall · Emergency exit</span>
            <span style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
              Scale: 1 cell = 2.5m × 5m
            </span>
          </div>
        </div>

        {/* Stats */}
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(4,1fr)', gap: 12, marginTop: 16 }}>
          {[
            { l: 'Standard', v: 24, c: 'var(--theme-text)' },
            { l: 'EV charging', v: 4, c: 'var(--color-success)' },
            { l: 'Accessible', v: 2, c: 'var(--color-info)' },
            { l: 'Motorcycle', v: 1, c: 'var(--color-warning)' },
          ].map((s) => (
            <div key={s.l} className="card" style={{ padding: 12 }}>
              <div style={{ fontSize: 11, color: 'var(--theme-text-muted)', fontWeight: 600 }}>{s.l}</div>
              <div style={{ fontSize: 20, fontWeight: 800, color: s.c, fontVariantNumeric: 'tabular-nums' }}>{s.v}</div>
            </div>
          ))}
        </div>
      </div>

      {/* Properties panel */}
      <div style={{
        borderLeft: '1px solid var(--theme-border)',
        padding: 20, display: 'flex', flexDirection: 'column', gap: 16,
        background: 'var(--theme-bg-subtle)',
        overflow: 'auto',
      }}>
        <div>
          <div style={{ fontSize: 11, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.05em', color: 'var(--theme-text-muted)', marginBottom: 4 }}>
            Selected slot
          </div>
          <div style={{ fontSize: 22, fontWeight: 800, letterSpacing: '-0.02em', fontVariantNumeric: 'tabular-nums' }}>L2-14</div>
        </div>

        {[
          { l: 'Slot type', kind: 'select', value: 'Standard', options: ['Standard','EV charging','Accessible','Motorcycle','Visitor'] },
          { l: 'Zone', kind: 'select', value: 'Zone B', options: ['Zone A','Zone B','Zone C'] },
          { l: 'Dimensions (m)', kind: 'dual', v1: '2.5', v2: '5.0' },
          { l: 'Cost multiplier', kind: 'input', value: '1.0x' },
          { l: 'Reserved for', kind: 'input', value: 'All users' },
        ].map((f) => (
          <div key={f.l}>
            <label style={{ fontSize: 11, fontWeight: 600, color: 'var(--theme-text-muted)', display: 'block', marginBottom: 4 }}>{f.l}</label>
            {f.kind === 'select' && (
              <div className="input" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                {f.value}
                <Icon name="chevron" size={14} style={{ transform: 'rotate(90deg)', color: 'var(--theme-text-muted)' }}/>
              </div>
            )}
            {f.kind === 'input' && <div className="input">{f.value}</div>}
            {f.kind === 'dual' && (
              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
                <div className="input">{f.v1}</div>
                <div className="input">{f.v2}</div>
              </div>
            )}
          </div>
        ))}

        <div style={{
          padding: 14, borderRadius: 10,
          background: 'color-mix(in oklch, var(--color-info) 6%, transparent)',
          border: '1px solid color-mix(in oklch, var(--color-info) 20%, transparent)',
          display: 'flex', gap: 10, alignItems: 'flex-start',
        }}>
          <Icon name="info" size={16} style={{ color: 'var(--color-info)', marginTop: 2 }}/>
          <div style={{ fontSize: 12, lineHeight: 1.45 }}>
            Bookings referencing this slot (23 active) will be preserved. Changes apply at next publish.
          </div>
        </div>

        <button className="btn btn-secondary" style={{ color: 'var(--color-danger)', justifyContent: 'center' }}>
          <Icon name="minus" size={14}/> Delete slot
        </button>
      </div>
    </div>
  );
}

window.AdminLots = AdminLots;
