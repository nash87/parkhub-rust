// App shell: sidebar navigation + topbar, matches Layout.tsx structure
const { useState } = React;

function ParkHubLogo({ size = 28 }) {
  return (
    <div style={{
      width: size, height: size, borderRadius: 8,
      background: 'linear-gradient(135deg, var(--color-primary-500), var(--color-primary-700))',
      display: 'flex', alignItems: 'center', justifyContent: 'center',
      color: '#fff', flexShrink: 0,
      boxShadow: '0 4px 12px -4px var(--color-primary-600)',
    }}>
      <Icon name="car-simple" size={size * 0.6} weight={2.2} />
    </div>
  );
}

const NAV = [
  { section: 'Core', items: [
    { icon: 'home', label: 'Dashboard', to: 'dashboard' },
    { icon: 'calendar-check', label: 'Bookings', to: 'bookings', badge: 3 },
    { icon: 'calendar-plus', label: 'Book a spot', to: 'book' },
    { icon: 'car', label: 'Vehicles', to: 'vehicles' },
    { icon: 'calendar', label: 'Calendar', to: 'calendar' },
    { icon: 'coins', label: 'Credits', to: 'credits' },
  ]},
  { section: 'Fleet', items: [
    { icon: 'star', label: 'Favorites', to: 'favorites' },
    { icon: 'users', label: 'Team', to: 'team' },
    { icon: 'trophy', label: 'Leaderboard', to: 'leaderboard' },
    { icon: 'map-pin', label: 'Map', to: 'map' },
    { icon: 'qr', label: 'Check-in', to: 'checkin' },
    { icon: 'swap', label: 'Swap requests', to: 'swap', badge: 2 },
    { icon: 'sparkle', label: 'Predictions', to: 'predict' },
  ]},
  { section: 'Admin', items: [
    { icon: 'grid', label: 'Lot editor', to: 'admin-lots' },
    { icon: 'users', label: 'Users', to: 'admin-users' },
    { icon: 'shield', label: 'Roles', to: 'admin-roles' },
    { icon: 'chart-line', label: 'Analytics', to: 'admin-analytics' },
    { icon: 'settings', label: 'Settings', to: 'admin-settings' },
  ]},
];

function Sidebar({ active, onNav, collapsed, onToggle }) {
  return (
    <aside style={{
      width: collapsed ? 72 : 260,
      flexShrink: 0,
      borderRight: '1px solid var(--theme-border)',
      background: 'var(--theme-bg-subtle)',
      backdropFilter: 'blur(12px)',
      height: '100%',
      display: 'flex', flexDirection: 'column',
      transition: 'width 200ms cubic-bezier(.4,0,.2,1)',
      overflow: 'hidden',
    }}>
      {/* Logo + brand */}
      <div style={{
        padding: '16px', display: 'flex', alignItems: 'center', gap: 10,
        borderBottom: '1px solid var(--theme-border-subtle)',
      }}>
        <ParkHubLogo size={32} />
        {!collapsed && (
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontWeight: 700, fontSize: 15, letterSpacing: '-0.02em' }}>ParkHub</div>
            <div style={{ fontSize: 11, color: 'var(--theme-text-muted)', fontVariantNumeric: 'tabular-nums' }}>v4.13.0 · Rust</div>
          </div>
        )}
        <button className="btn btn-ghost btn-icon" onClick={onToggle} title="Toggle sidebar">
          <Icon name={collapsed ? 'arrow' : 'menu'} size={16} />
        </button>
      </div>

      {/* Nav */}
      <nav style={{ flex: 1, overflowY: 'auto', padding: '12px 8px' }}>
        {NAV.map((sec) => (
          <div key={sec.section} style={{ marginBottom: 14 }}>
            {!collapsed && (
              <div style={{
                padding: '8px 12px 6px', fontSize: 11, fontWeight: 600,
                color: 'var(--theme-text-faint)', textTransform: 'uppercase',
                letterSpacing: '0.06em',
              }}>{sec.section}</div>
            )}
            <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
              {sec.items.map((item) => {
                const isActive = active === item.to;
                return (
                  <button key={item.to} onClick={() => onNav(item.to)}
                    title={collapsed ? item.label : undefined}
                    style={{
                      display: 'flex', alignItems: 'center', gap: 10,
                      padding: collapsed ? '10px' : '8px 12px',
                      borderRadius: 8, textAlign: 'left', width: '100%',
                      justifyContent: collapsed ? 'center' : 'flex-start',
                      background: isActive ? 'color-mix(in oklch, var(--color-primary-500) 12%, transparent)' : 'transparent',
                      color: isActive ? 'var(--color-primary-700)' : 'var(--theme-text)',
                      fontWeight: isActive ? 600 : 500, fontSize: 14,
                      position: 'relative',
                    }}
                    onMouseEnter={(e) => {
                      if (!isActive) e.currentTarget.style.background = 'var(--theme-bg-muted)';
                    }}
                    onMouseLeave={(e) => {
                      if (!isActive) e.currentTarget.style.background = 'transparent';
                    }}>
                    {isActive && (
                      <span style={{
                        position: 'absolute', left: 0, top: 8, bottom: 8, width: 3,
                        borderRadius: 3, background: 'var(--color-primary-500)',
                      }} />
                    )}
                    <Icon name={item.icon} size={18} />
                    {!collapsed && <span style={{ flex: 1 }}>{item.label}</span>}
                    {!collapsed && item.badge && (
                      <span className="badge badge-primary" style={{ fontSize: 10, padding: '2px 6px' }}>{item.badge}</span>
                    )}
                  </button>
                );
              })}
            </div>
          </div>
        ))}
      </nav>

      {/* User */}
      {!collapsed && (
        <div style={{
          padding: 12, borderTop: '1px solid var(--theme-border-subtle)',
          display: 'flex', alignItems: 'center', gap: 10,
        }}>
          <div style={{
            width: 36, height: 36, borderRadius: '50%',
            background: 'linear-gradient(135deg, var(--color-primary-400), var(--color-primary-600))',
            display: 'flex', alignItems: 'center', justifyContent: 'center',
            color: '#fff', fontWeight: 700, fontSize: 14,
          }}>FB</div>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div style={{ fontSize: 13, fontWeight: 600, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>Florian Bauer</div>
            <div style={{ fontSize: 11, color: 'var(--theme-text-muted)' }}>Admin · 45 credits</div>
          </div>
          <button className="btn btn-ghost btn-icon" title="Sign out"><Icon name="x" size={16} /></button>
        </div>
      )}
    </aside>
  );
}

function TopBar({ title, subtitle, breadcrumbs, edition, onEditionChange, onCmdK }) {
  return (
    <header style={{
      padding: '14px 28px',
      borderBottom: '1px solid var(--theme-border-subtle)',
      background: 'var(--glass-bg)',
      backdropFilter: 'blur(12px)',
      display: 'flex', alignItems: 'center', gap: 16,
      position: 'sticky', top: 0, zIndex: 10,
    }}>
      <div style={{ flex: 1, minWidth: 0 }}>
        {breadcrumbs && (
          <div style={{ fontSize: 12, color: 'var(--theme-text-muted)', marginBottom: 2 }}>
            {breadcrumbs.join(' / ')}
          </div>
        )}
        <h1 style={{ fontSize: 20, fontWeight: 700, letterSpacing: '-0.025em' }}>{title}</h1>
        {subtitle && <div style={{ fontSize: 12, color: 'var(--theme-text-muted)', marginTop: 2 }}>{subtitle}</div>}
      </div>

      {/* Command palette */}
      <button onClick={onCmdK} style={{
        display: 'flex', alignItems: 'center', gap: 8,
        padding: '8px 12px', border: '1px solid var(--theme-border)',
        borderRadius: 8, background: 'var(--theme-bg-muted)',
        color: 'var(--theme-text-muted)', fontSize: 13, minWidth: 220,
      }}>
        <Icon name="search" size={14} />
        <span style={{ flex: 1, textAlign: 'left' }}>Search or jump to…</span>
        <kbd style={{
          padding: '2px 6px', fontSize: 10, fontFamily: 'inherit',
          background: 'var(--theme-bg-subtle)', border: '1px solid var(--theme-border)',
          borderRadius: 4, fontWeight: 600,
        }}>⌘K</kbd>
      </button>

      {/* Edition switcher */}
      <div style={{
        display: 'flex', alignItems: 'center',
        background: 'var(--theme-bg-muted)', borderRadius: 8,
        padding: 3, border: '1px solid var(--theme-border)',
      }}>
        {['rust', 'php'].map((ed) => (
          <button key={ed} onClick={() => onEditionChange(ed)}
            style={{
              padding: '5px 11px', borderRadius: 6, fontSize: 12, fontWeight: 600,
              background: edition === ed ? 'var(--theme-card-bg)' : 'transparent',
              color: edition === ed ? 'var(--color-primary-700)' : 'var(--theme-text-muted)',
              boxShadow: edition === ed ? 'var(--shadow-xs)' : 'none',
              textTransform: 'uppercase', letterSpacing: '0.03em',
            }}>
            {ed}
          </button>
        ))}
      </div>

      <button className="btn btn-ghost btn-icon" title="Notifications" style={{ position: 'relative' }}>
        <Icon name="bell" size={18} />
        <span style={{
          position: 'absolute', top: 6, right: 6, width: 8, height: 8,
          borderRadius: '50%', background: 'var(--color-danger)',
          border: '2px solid var(--theme-bg-subtle)',
        }} />
      </button>
    </header>
  );
}

window.Sidebar = Sidebar;
window.TopBar = TopBar;
window.ParkHubLogo = ParkHubLogo;
