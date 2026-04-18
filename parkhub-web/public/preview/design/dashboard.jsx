// Dashboard — mirrors parkhub-web/src/views/Dashboard.tsx structure
const { useMemo: useMemoD } = React;

function KpiCard({ label, value, delta, icon, live, tone = 'primary' }) {
  return (
    <div className="card" style={{ padding: 16, position: 'relative', overflow: 'hidden' }}>
      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', marginBottom: 10 }}>
        <span style={{ fontSize: 12, fontWeight: 500, color: 'var(--theme-text-muted)' }}>{label}</span>
        <div style={{
          width: 28, height: 28, borderRadius: 8,
          background: `color-mix(in oklch, var(--color-${tone}-500) 12%, transparent)`,
          color: `var(--color-${tone}-600)`,
          display: 'flex', alignItems: 'center', justifyContent: 'center',
        }}>
          <Icon name={icon} size={14} weight={2} />
        </div>
      </div>
      <div style={{
        fontSize: 28, fontWeight: 800, letterSpacing: '-0.02em',
        lineHeight: 1, fontVariantNumeric: 'tabular-nums',
        color: 'var(--theme-text)',
      }}>{value}</div>
      {delta !== undefined && (
        <div style={{
          marginTop: 8, display: 'inline-flex', alignItems: 'center', gap: 4,
          fontSize: 11, fontWeight: 600,
          color: delta > 0 ? 'var(--color-success)' : 'var(--color-danger)',
        }}>
          <Icon name="trend" size={12} weight={2} />
          {delta > 0 ? '+' : ''}{delta}% vs last month
        </div>
      )}
      {live && (
        <span style={{
          position: 'absolute', top: 10, right: 10,
          display: 'inline-flex', alignItems: 'center', gap: 4,
          fontSize: 10, fontWeight: 600, color: 'var(--color-success)',
        }}>
          <span className="pulse-dot" style={{
            width: 6, height: 6, borderRadius: '50%',
            background: 'var(--color-success)',
          }}/>
        </span>
      )}
    </div>
  );
}

function Sparkline({ data, height = 52 }) {
  const max = Math.max(...data, 1);
  const min = Math.min(...data, 0);
  const range = max - min || 1;
  const w = 300, h = height;
  const pts = data.map((v, i) => [
    (i / (data.length - 1)) * w,
    h - ((v - min) / range) * h * 0.85 - 4,
  ]);
  const d = 'M ' + pts.map((p) => p.join(',')).join(' L ');
  const fill = d + ` L ${w},${h} L 0,${h} Z`;
  return (
    <svg viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none"
         style={{ width: '100%', height }}>
      <defs>
        <linearGradient id="sparkFill" x1="0" x2="0" y1="0" y2="1">
          <stop offset="0%" stopColor="var(--color-primary-500)" stopOpacity="0.25"/>
          <stop offset="100%" stopColor="var(--color-primary-500)" stopOpacity="0"/>
        </linearGradient>
      </defs>
      <path d={fill} fill="url(#sparkFill)" />
      <path d={d} fill="none" stroke="var(--color-primary-500)" strokeWidth="2"
            strokeLinecap="round" strokeLinejoin="round" vectorEffect="non-scaling-stroke"/>
      {pts.map(([x, y], i) => (
        <circle key={i} cx={x} cy={y} r={i === pts.length - 1 ? 3 : 0}
                fill="var(--color-primary-600)" />
      ))}
    </svg>
  );
}

function TrendCard({ title, subtitle, data, labels, period, onPeriodChange }) {
  return (
    <div className="card" style={{ padding: 20, height: '100%' }}>
      <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', marginBottom: 14 }}>
        <div>
          <h3 style={{ fontSize: 15, fontWeight: 600, letterSpacing: '-0.01em' }}>{title}</h3>
          <p style={{ fontSize: 12, color: 'var(--theme-text-muted)', marginTop: 2 }}>{subtitle}</p>
        </div>
        <div style={{ display: 'flex', gap: 4, background: 'var(--theme-bg-muted)', padding: 3, borderRadius: 8 }}>
          {['7d', '30d'].map((p) => (
            <button key={p} onClick={() => onPeriodChange(p)} style={{
              padding: '4px 10px', fontSize: 11, fontWeight: 600, borderRadius: 5,
              background: period === p ? 'var(--theme-card-bg)' : 'transparent',
              color: period === p ? 'var(--color-primary-700)' : 'var(--theme-text-muted)',
              boxShadow: period === p ? 'var(--shadow-xs)' : 'none',
            }}>{p.toUpperCase()}</button>
          ))}
        </div>
      </div>

      <Sparkline data={data} height={110} />

      <div style={{
        display: 'flex', justifyContent: 'space-between',
        fontSize: 10, color: 'var(--theme-text-faint)', fontWeight: 600,
        marginTop: 8, textTransform: 'uppercase', letterSpacing: '0.05em',
      }}>
        {labels.map((l, i) => <span key={i}>{l}</span>)}
      </div>
    </div>
  );
}

function SensorFeedCard() {
  const sensors = [
    { name: 'Entrance Gate A', status: 'active', lastPing: '2s ago' },
    { name: 'Entrance Gate B', status: 'active', lastPing: '4s ago' },
    { name: 'Exit Gate North', status: 'active', lastPing: '1s ago' },
    { name: 'EV Charger 3', status: 'maintenance', lastPing: '12m ago' },
    { name: 'Barrier Zone C', status: 'active', lastPing: '6s ago' },
  ];
  return (
    <div className="card" style={{ padding: 20, height: '100%' }}>
      <div style={{ marginBottom: 12 }}>
        <h3 style={{ fontSize: 15, fontWeight: 600, letterSpacing: '-0.01em' }}>Live Sensor Feed</h3>
        <p style={{ fontSize: 12, color: 'var(--theme-text-muted)', marginTop: 2 }}>Real-time gate and entry status</p>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
        {sensors.map((s) => (
          <div key={s.name} style={{
            display: 'flex', alignItems: 'center', gap: 10,
            padding: '8px 10px', borderRadius: 8,
            background: 'var(--theme-bg-muted)',
          }}>
            <span className={s.status === 'active' ? 'pulse-dot' : ''} style={{
              width: 8, height: 8, borderRadius: '50%', flexShrink: 0,
              background: s.status === 'active' ? 'var(--color-success)' : 'var(--color-warning)',
            }}/>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontSize: 13, fontWeight: 500 }}>{s.name}</div>
            </div>
            <div style={{ fontSize: 11, color: 'var(--theme-text-muted)', fontVariantNumeric: 'tabular-nums' }}>{s.lastPing}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

function ActiveBookingRow({ slot, lot, vehicle, endIn, status }) {
  return (
    <div style={{
      display: 'flex', alignItems: 'center', gap: 14,
      padding: 12, borderRadius: 10,
      background: 'var(--theme-bg-muted)',
      transition: 'background 150ms',
    }} onMouseEnter={e => e.currentTarget.style.background='var(--theme-bg-subtle)'}
       onMouseLeave={e => e.currentTarget.style.background='var(--theme-bg-muted)'}>
      <div style={{
        width: 44, height: 44, borderRadius: 10,
        background: 'color-mix(in oklch, var(--color-primary-500) 12%, transparent)',
        color: 'var(--color-primary-700)',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
        fontSize: 14, fontWeight: 700, fontVariantNumeric: 'tabular-nums',
        flexShrink: 0,
      }}>{slot}</div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ fontSize: 14, fontWeight: 600, marginBottom: 2 }}>{lot}</div>
        <div style={{ fontSize: 12, color: 'var(--theme-text-muted)', display: 'flex', alignItems: 'center', gap: 6 }}>
          <Icon name="car" size={12}/>{vehicle}
          <span>·</span>
          <Icon name="clock" size={12}/>ends in {endIn}
        </div>
      </div>
      <span className={`badge badge-${status === 'active' ? 'success' : 'info'}`}>{status}</span>
    </div>
  );
}

function QuickAction({ icon, label, accent, onClick }) {
  return (
    <button onClick={onClick} style={{
      display: 'flex', alignItems: 'center', gap: 12,
      padding: '12px 14px', borderRadius: 10, width: '100%', textAlign: 'left',
      background: accent ? 'color-mix(in oklch, var(--color-primary-500) 10%, transparent)' : 'transparent',
      color: accent ? 'var(--color-primary-700)' : 'var(--theme-text)',
      fontSize: 14, fontWeight: 500,
    }} onMouseEnter={e => {
      if (!accent) e.currentTarget.style.background = 'var(--theme-bg-muted)';
    }} onMouseLeave={e => {
      if (!accent) e.currentTarget.style.background = 'transparent';
    }}>
      <Icon name={icon} size={18} />
      <span style={{ flex: 1 }}>{label}</span>
      <Icon name="chevron" size={14} weight={2} />
    </button>
  );
}

function RecentActivityTable() {
  const rows = [
    { vehicle: 'BMW i4 · M-PH 2341', lot: 'HQ Garage, Level 2', slot: 'L2-14', time: '08:42', duration: '4h 30m', status: 'in_progress' },
    { vehicle: 'Tesla Model Y · M-EV 107', lot: 'HQ Garage, Level 1', slot: 'L1-03', time: '08:15', duration: '8h 00m', status: 'confirmed' },
    { vehicle: 'VW ID.3 · M-VW 4421', lot: 'Annex North', slot: 'N-09', time: '07:58', duration: '6h 15m', status: 'confirmed' },
    { vehicle: 'Fiat 500e · M-FT 998', lot: 'HQ Garage, Level 3', slot: 'L3-27', time: '07:30', duration: '2h 00m', status: 'completed' },
    { vehicle: 'Audi e-tron · M-AU 512', lot: 'Annex South', slot: 'S-11', time: 'Yesterday', duration: '9h 10m', status: 'completed' },
  ];
  const statusMap = {
    in_progress: { cls: 'badge-success', label: 'In progress' },
    confirmed: { cls: 'badge-info', label: 'Confirmed' },
    completed: { cls: 'badge-gray', label: 'Completed' },
    cancelled: { cls: 'badge-error', label: 'Cancelled' },
  };
  return (
    <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '16px 20px' }}>
        <div>
          <h3 style={{ fontSize: 15, fontWeight: 600 }}>Recent Activity</h3>
          <p style={{ fontSize: 12, color: 'var(--theme-text-muted)', marginTop: 2 }}>Last 5 check-ins across your fleet</p>
        </div>
        <button className="btn btn-ghost btn-sm" style={{ color: 'var(--color-primary-600)' }}>
          View all <Icon name="arrow" size={12} />
        </button>
      </div>
      <div style={{ overflowX: 'auto' }}>
        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 13 }}>
          <thead>
            <tr style={{
              borderTop: '1px solid var(--theme-border-subtle)',
              borderBottom: '1px solid var(--theme-border-subtle)',
              background: 'var(--theme-bg-muted)',
            }}>
              {['Vehicle / Location', 'Slot', 'Check-in', 'Duration', 'Status'].map((h) => (
                <th key={h} style={{
                  textAlign: 'left', padding: '10px 20px', fontSize: 11,
                  fontWeight: 600, color: 'var(--theme-text-muted)',
                  textTransform: 'uppercase', letterSpacing: '0.05em',
                }}>{h}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((r, i) => (
              <tr key={i} style={{ borderBottom: i < rows.length - 1 ? '1px solid var(--theme-border-subtle)' : 'none' }}>
                <td style={{ padding: '12px 20px' }}>
                  <div style={{ fontWeight: 500 }}>{r.vehicle}</div>
                  <div style={{ fontSize: 11, color: 'var(--theme-text-muted)', marginTop: 1 }}>{r.lot}</div>
                </td>
                <td style={{ padding: '12px 20px', fontVariantNumeric: 'tabular-nums', fontWeight: 600 }}>{r.slot}</td>
                <td style={{ padding: '12px 20px', fontVariantNumeric: 'tabular-nums', color: 'var(--theme-text-muted)' }}>{r.time}</td>
                <td style={{ padding: '12px 20px', fontVariantNumeric: 'tabular-nums' }}>{r.duration}</td>
                <td style={{ padding: '12px 20px' }}>
                  <span className={`badge ${statusMap[r.status].cls}`}>{statusMap[r.status].label}</span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function Dashboard({ edition }) {
  const [period, setPeriod] = React.useState('7d');
  const data7 = [3, 7, 5, 9, 6, 11, 8];
  const data30 = Array.from({length: 30}, (_,i) => 5 + Math.sin(i/3)*3 + (i%5));
  const data = period === '7d' ? data7 : data30;
  const labels = period === '7d'
    ? ['Mon','Tue','Wed','Thu','Fri','Sat','Sun']
    : ['M1','W1','W2','W3','W4','M2'];

  const hour = new Date().getHours();
  const timeOfDay = hour < 12 ? 'morning' : hour < 18 ? 'afternoon' : 'evening';
  const gradient = hour < 12
    ? 'linear-gradient(90deg, rgba(251,191,36,0.1), rgba(251,146,60,0.05), transparent)'
    : hour < 18
    ? 'linear-gradient(90deg, rgba(56,189,248,0.1), rgba(59,130,246,0.05), transparent)'
    : 'linear-gradient(90deg, rgba(99,102,241,0.1), rgba(168,85,247,0.05), transparent)';

  return (
    <div style={{ padding: 28, display: 'flex', flexDirection: 'column', gap: 20 }}>
      {/* Greeting */}
      <div style={{
        padding: '18px 24px', borderRadius: 14, background: gradient,
        display: 'flex', alignItems: 'center', gap: 12,
      }}>
        <h1 style={{ fontSize: 24, fontWeight: 700, letterSpacing: '-0.025em' }}>
          Good {timeOfDay}, Florian
        </h1>
        <span style={{
          display: 'inline-flex', alignItems: 'center', gap: 6,
          fontSize: 11, fontWeight: 600, color: 'var(--color-success)',
        }}>
          <span className="pulse-dot" style={{ width: 6, height: 6, borderRadius: '50%', background: 'var(--color-success)' }} />
          LIVE · {edition.toUpperCase()} edition
        </span>
      </div>

      {/* KPIs */}
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(5, 1fr)', gap: 12 }}>
        <KpiCard label="Active bookings" value="3" icon="chart-line" tone="primary" live />
        <KpiCard label="Credits left" value="45" icon="coins" delta={-12} tone="primary" />
        <KpiCard label="This month" value="28" icon="calendar-check" delta={14} tone="primary" />
        <KpiCard label="Total bookings" value="247" icon="gauge" tone="primary" />
        <KpiCard label="CO₂ saved (30d)" value="12.4 kg" icon="leaf" tone="primary" delta={8} />
      </div>

      {/* Trend + Sensors */}
      <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: 14 }}>
        <TrendCard
          title="Weekly Activity"
          subtitle="Booking volume over the selected period"
          data={data}
          labels={labels}
          period={period}
          onPeriodChange={setPeriod}
        />
        <SensorFeedCard />
      </div>

      {/* Active bookings + Quick actions */}
      <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: 14 }}>
        <div className="card" style={{ padding: 20 }}>
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 14 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
              <h2 style={{ fontSize: 15, fontWeight: 600 }}>Active bookings</h2>
              <span style={{ display: 'inline-flex', alignItems: 'center', gap: 4, fontSize: 11, color: 'var(--color-success)', fontWeight: 600 }}>
                <span className="pulse-dot" style={{ width: 6, height: 6, borderRadius: '50%', background: 'var(--color-success)' }} />3
              </span>
            </div>
            <button className="btn btn-ghost btn-sm" style={{ color: 'var(--color-primary-600)' }}>
              View all <Icon name="arrow" size={12} />
            </button>
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <ActiveBookingRow slot="L2-14" lot="HQ Garage, Level 2" vehicle="BMW i4 · M-PH 2341" endIn="3h 12m" status="active" />
            <ActiveBookingRow slot="L1-03" lot="HQ Garage, Level 1" vehicle="Tesla Model Y · M-EV 107" endIn="7h 45m" status="confirmed" />
            <ActiveBookingRow slot="N-09" lot="Annex North" vehicle="VW ID.3 · M-VW 4421" endIn="5h 50m" status="confirmed" />
          </div>
        </div>

        <div className="card" style={{ padding: 20 }}>
          <h2 style={{ fontSize: 15, fontWeight: 600, marginBottom: 10 }}>Quick actions</h2>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            <QuickAction icon="calendar-plus" label="Book a spot" accent />
            <QuickAction icon="qr" label="Show parking pass" />
            <QuickAction icon="car" label="My vehicles" />
            <QuickAction icon="coins" label="Buy credits" />
            <QuickAction icon="users" label="Invite team" />
          </div>
        </div>
      </div>

      {/* Recent activity table */}
      <RecentActivityTable />
    </div>
  );
}

window.Dashboard = Dashboard;
