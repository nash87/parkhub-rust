// Parking Pass + QR check-in — the "moment of truth"
function ParkingPass() {
  // Simple QR using a pattern of squares
  const qrGrid = React.useMemo(() => {
    const size = 25;
    const pat = [];
    // Pseudo-random but deterministic
    for (let y = 0; y < size; y++) {
      for (let x = 0; x < size; x++) {
        const isFinder = (x < 7 && y < 7) || (x >= size - 7 && y < 7) || (x < 7 && y >= size - 7);
        const isFinderInner = (x >= 1 && x < 6 && y >= 1 && y < 6 && !(x >= 2 && x < 5 && y >= 2 && y < 5)) ||
                              (x >= size - 6 && x < size - 1 && y >= 1 && y < 6 && !(x >= size - 5 && x < size - 2 && y >= 2 && y < 5)) ||
                              (x >= 1 && x < 6 && y >= size - 6 && y < size - 1 && !(x >= 2 && x < 5 && y >= size - 5 && y < size - 2));
        const isFinderCenter = (x >= 2 && x < 5 && y >= 2 && y < 5) ||
                               (x >= size - 5 && x < size - 2 && y >= 2 && y < 5) ||
                               (x >= 2 && x < 5 && y >= size - 5 && y < size - 2);
        let on = false;
        if (isFinder && !isFinderInner) on = true;
        if (isFinderCenter) on = true;
        if (!isFinder) on = ((x * 37 + y * 53 + x*y*7) % 3) !== 0;
        pat.push({ x, y, on });
      }
    }
    return { size, pat };
  }, []);

  return (
    <div style={{ padding: 28, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 20 }}>
      {/* Pass */}
      <div style={{
        width: '100%', maxWidth: 420, borderRadius: 20,
        background: 'linear-gradient(170deg, var(--color-primary-600), var(--color-primary-800))',
        color: '#fff', overflow: 'hidden',
        boxShadow: '0 20px 60px -20px color-mix(in oklch, var(--color-primary-600) 60%, black)',
        position: 'relative',
      }}>
        <div aria-hidden style={{
          position: 'absolute', inset: 0,
          backgroundImage: 'radial-gradient(circle at 20% 0%, rgba(255,255,255,0.15), transparent 50%), radial-gradient(circle at 80% 100%, rgba(255,255,255,0.08), transparent 40%)',
          pointerEvents: 'none',
        }}/>
        {/* Header */}
        <div style={{ padding: '18px 20px', display: 'flex', alignItems: 'center', justifyContent: 'space-between', position: 'relative' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <div style={{
              width: 28, height: 28, borderRadius: 7,
              background: 'rgba(255,255,255,0.18)',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
            }}>
              <Icon name="car-simple" size={18} weight={2.2} />
            </div>
            <div>
              <div style={{ fontSize: 10, opacity: 0.7, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.08em' }}>ParkHub</div>
              <div style={{ fontSize: 13, fontWeight: 700 }}>Digital pass</div>
            </div>
          </div>
          <span className="pulse-dot" style={{
            display: 'inline-flex', alignItems: 'center', gap: 6,
            padding: '4px 8px', borderRadius: 999,
            background: 'rgba(34,197,94,0.2)', color: '#86efac',
            fontSize: 10, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.05em',
          }}>
            <span style={{ width: 6, height: 6, borderRadius: '50%', background: '#22c55e' }}/>
            Active
          </span>
        </div>

        {/* QR */}
        <div style={{ padding: '6px 24px 20px', display: 'flex', justifyContent: 'center' }}>
          <div style={{ width: '100%', aspectRatio: '1/1', maxWidth: 300, padding: 14, background: '#fff', borderRadius: 16, boxShadow: '0 10px 30px rgba(0,0,0,0.25)' }}>
            <svg viewBox={`0 0 ${qrGrid.size} ${qrGrid.size}`} style={{ width: '100%', height: '100%', display: 'block' }} shapeRendering="crispEdges">
              {qrGrid.pat.filter(c => c.on).map((c, i) => (
                <rect key={i} x={c.x} y={c.y} width="1" height="1" fill="#0c1422"/>
              ))}
              {/* Center logo */}
              <rect x={qrGrid.size/2-2.5} y={qrGrid.size/2-2.5} width="5" height="5" fill="#fff"/>
              <rect x={qrGrid.size/2-1.8} y={qrGrid.size/2-1.8} width="3.6" height="3.6" rx="0.5" fill="var(--color-primary-600)"/>
            </svg>
          </div>
        </div>

        {/* Details */}
        <div style={{ padding: '0 24px 24px', position: 'relative' }}>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 14 }}>
            {[
              { l: 'Lot', v: 'HQ Garage', s: 'Level 2' },
              { l: 'Slot', v: 'L2-14', s: 'Covered · Entry A' },
              { l: 'Vehicle', v: 'BMW i4', s: 'M-PH 2341' },
              { l: 'Valid until', v: '16:00', s: 'Today · 5h 12m left' },
            ].map((d) => (
              <div key={d.l}>
                <div style={{ fontSize: 10, opacity: 0.7, fontWeight: 600, textTransform: 'uppercase', letterSpacing: '0.08em', marginBottom: 3 }}>{d.l}</div>
                <div style={{ fontSize: 15, fontWeight: 700, letterSpacing: '-0.01em', fontVariantNumeric: 'tabular-nums' }}>{d.v}</div>
                <div style={{ fontSize: 11, opacity: 0.75 }}>{d.s}</div>
              </div>
            ))}
          </div>
        </div>

        {/* Perforation */}
        <div style={{ position: 'relative', height: 18, background: 'rgba(0,0,0,0.15)' }}>
          <div style={{ position: 'absolute', left: -10, top: '50%', transform: 'translateY(-50%)', width: 20, height: 20, borderRadius: '50%', background: 'var(--theme-bg)' }}/>
          <div style={{ position: 'absolute', right: -10, top: '50%', transform: 'translateY(-50%)', width: 20, height: 20, borderRadius: '50%', background: 'var(--theme-bg)' }}/>
          <div style={{
            position: 'absolute', left: 20, right: 20, top: '50%',
            borderTop: '1.5px dashed rgba(255,255,255,0.4)',
          }}/>
        </div>

        {/* Footer */}
        <div style={{ padding: '14px 24px 20px', display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: 11 }}>
          <div style={{ opacity: 0.7, fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace' }}>
            PH-8F4A-2341-L2-14
          </div>
          <div style={{ opacity: 0.7 }}>
            Issued 08:42
          </div>
        </div>
      </div>

      {/* Actions */}
      <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', justifyContent: 'center' }}>
        <button className="btn btn-secondary"><Icon name="download" size={14}/> Add to Wallet</button>
        <button className="btn btn-secondary"><Icon name="printer" size={14}/> Print</button>
        <button className="btn btn-secondary"><Icon name="copy" size={14}/> Share link</button>
        <button className="btn btn-secondary" style={{ color: 'var(--color-danger)' }}><Icon name="x" size={14}/> End session</button>
      </div>

      {/* Check-in status card */}
      <div className="card" style={{ padding: 20, width: '100%', maxWidth: 420 }}>
        <h3 style={{ fontSize: 14, fontWeight: 700, marginBottom: 12, display: 'flex', alignItems: 'center', gap: 8 }}>
          <Icon name="clock" size={16} /> Check-in timeline
        </h3>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 0, position: 'relative' }}>
          {[
            { t: '08:42', label: 'Booking confirmed', status: 'done' },
            { t: '08:43', label: 'QR pass issued', status: 'done' },
            { t: '09:01', label: 'Entered HQ Garage', status: 'done', detail: 'Gate A · scanned in 0.3s' },
            { t: '—',   label: 'Expected check-out', status: 'pending', detail: '16:00' },
          ].map((s, i, arr) => (
            <div key={i} style={{ display: 'flex', alignItems: 'flex-start', gap: 12, paddingBottom: i < arr.length-1 ? 14 : 0 }}>
              <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', paddingTop: 2 }}>
                <div style={{
                  width: 16, height: 16, borderRadius: '50%',
                  background: s.status === 'done' ? 'var(--color-success)' : 'var(--theme-bg-muted)',
                  border: s.status === 'pending' ? '2px dashed var(--theme-border)' : 'none',
                  display: 'flex', alignItems: 'center', justifyContent: 'center',
                }}>
                  {s.status === 'done' && <Icon name="check" size={10} weight={3} style={{ color: '#fff' }} />}
                </div>
                {i < arr.length-1 && <div style={{ width: 2, flex: 1, minHeight: 20, background: s.status === 'done' ? 'var(--color-success)' : 'var(--theme-border)', marginTop: 2 }}/>}
              </div>
              <div style={{ flex: 1, paddingBottom: 6 }}>
                <div style={{ fontSize: 13, fontWeight: 600 }}>{s.label}</div>
                {s.detail && <div style={{ fontSize: 11, color: 'var(--theme-text-muted)', marginTop: 2 }}>{s.detail}</div>}
              </div>
              <div style={{ fontSize: 11, color: 'var(--theme-text-muted)', fontVariantNumeric: 'tabular-nums', fontWeight: 600 }}>{s.t}</div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

window.ParkingPass = ParkingPass;
