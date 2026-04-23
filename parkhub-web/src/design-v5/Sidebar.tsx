import { useState } from 'react';
import { NAV, SECTION_HEADINGS, type NavSection, type ScreenId } from './nav';
import { V5NamedIcon, LiveDot } from './primitives';
import { useV5Theme } from './ThemeProvider';

interface SidebarProps {
  active: ScreenId;
  onNavigate: (id: ScreenId) => void;
  userLabel?: string;
  userEmail?: string;
}

function Brand({ inverted = false }: { inverted?: boolean }) {
  return (
    <>
      <div
        style={{
          width: 24,
          height: 24,
          borderRadius: 7,
          background: 'var(--v5-acc)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
        }}
      >
        <span
          style={{
            color: inverted ? 'var(--v5-accent-fg)' : '#fff',
            fontWeight: 800,
            fontSize: 11,
            fontStyle: 'italic',
          }}
        >
          P
        </span>
      </div>
      <span
        style={{
          fontWeight: 700,
          fontSize: 14,
          color: 'var(--v5-txt)',
          letterSpacing: '-0.4px',
        }}
      >
        ParkHub
      </span>
    </>
  );
}

function AccountFooter({ userLabel = 'Administrator', userEmail = 'admin@parkhub.test', inverted = false }: {
  userLabel?: string;
  userEmail?: string;
  inverted?: boolean;
}) {
  return (
    <div
      style={{
        borderTop: '1px solid var(--v5-bor)',
        padding: '9px 12px',
        display: 'flex',
        alignItems: 'center',
        gap: 8,
      }}
    >
      <div
        style={{
          width: 26,
          height: 26,
          borderRadius: '50%',
          background: 'linear-gradient(135deg, var(--v5-acc), oklch(0.4 0.1 175))',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          color: inverted ? 'var(--v5-accent-fg)' : '#fff',
          fontSize: 10,
          fontWeight: 700,
        }}
      >
        {userLabel.charAt(0)}
      </div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          style={{
            fontSize: 11,
            fontWeight: 600,
            color: 'var(--v5-txt)',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {userLabel}
        </div>
        <div
          className="v5-mono"
          style={{
            fontSize: 9,
            color: 'var(--v5-mut)',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
          }}
        >
          {userEmail}
        </div>
      </div>
      <button
        type="button"
        aria-label="Abmelden"
        style={{
          opacity: 0.5,
          cursor: 'pointer',
          background: 'transparent',
          border: 0,
          padding: 4,
        }}
      >
        <V5NamedIcon name="logout" size={12} color="var(--v5-txt)" />
      </button>
    </div>
  );
}

function MarbleSidebar({ active, onNavigate, userLabel, userEmail }: SidebarProps) {
  const sections: NavSection[] = ['main', 'fleet', 'admin'];

  return (
    <nav
      aria-label="Hauptnavigation"
      style={{
        width: 210,
        borderRight: '1px solid var(--v5-bor)',
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--v5-nav-bg)',
        flexShrink: 0,
      }}
    >
      <div
        style={{
          height: 54,
          display: 'flex',
          alignItems: 'center',
          padding: '0 14px',
          gap: 8,
          borderBottom: '1px solid var(--v5-bor)',
        }}
      >
        <Brand />
        <div style={{ marginLeft: 'auto' }}>
          <LiveDot color="var(--v5-acc)" />
        </div>
      </div>
      <div style={{ flex: 1, overflowY: 'auto', padding: '6px 0' }}>
        {sections.map((sec) => (
          <div key={sec} style={{ marginBottom: 4 }}>
            <div
              className="v5-mono"
              style={{
                fontSize: 9,
                letterSpacing: 1.4,
                color: 'var(--v5-bor)',
                padding: '7px 14px 3px',
                textTransform: 'uppercase',
              }}
            >
              {SECTION_HEADINGS[sec]}
            </div>
            {NAV.filter((n) => n.section === sec).map((item) => {
              const isActive = active === item.id;
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => onNavigate(item.id as ScreenId)}
                  aria-current={isActive ? 'page' : undefined}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    width: 'calc(100% - 10px)',
                    height: 34,
                    padding: '0 8px 0 12px',
                    cursor: 'pointer',
                    borderRadius: 8,
                    margin: '1px 5px',
                    background: isActive ? 'var(--v5-acc-muted)' : 'transparent',
                    borderLeft: `2px solid ${isActive ? 'var(--v5-acc)' : 'transparent'}`,
                    borderTop: 0,
                    borderRight: 0,
                    borderBottom: 0,
                    transition: 'all 0.11s',
                    textAlign: 'left',
                  }}
                >
                  <V5NamedIcon
                    name={item.icon}
                    size={13}
                    color={isActive ? 'var(--v5-acc)' : 'var(--v5-mut)'}
                  />
                  <span
                    style={{
                      fontSize: 12,
                      fontWeight: isActive ? 600 : 400,
                      color: isActive ? 'var(--v5-acc)' : 'var(--v5-txt)',
                      letterSpacing: '-0.1px',
                    }}
                  >
                    {item.label}
                  </span>
                </button>
              );
            })}
          </div>
        ))}
      </div>
      <AccountFooter userLabel={userLabel} userEmail={userEmail} />
    </nav>
  );
}

function VoidSidebar({ active, onNavigate, userLabel, userEmail }: SidebarProps) {
  const [hover, setHover] = useState<string | null>(null);
  const ticker = 'PARKHUB AKTIV · ADMIN · 40 CREDITS · 1/12 BELEGT · EV: 3 FREI · LIVE · ';

  return (
    <nav
      aria-label="Hauptnavigation"
      style={{
        width: 230,
        borderRight: '1px solid var(--v5-bor)',
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--v5-nav-bg)',
        flexShrink: 0,
      }}
    >
      {/* Ticker — editorial constant motion */}
      <div
        aria-hidden="true"
        style={{
          height: 26,
          borderBottom: '1px solid var(--v5-bor)',
          overflow: 'hidden',
          display: 'flex',
          alignItems: 'center',
        }}
      >
        <div
          style={{
            display: 'flex',
            whiteSpace: 'nowrap',
            animation: 'ph-v5-ticker 20s linear infinite',
          }}
        >
          {[0, 1, 2].map((i) => (
            <span
              key={i}
              className="v5-mono"
              style={{
                fontSize: 8,
                letterSpacing: 1.8,
                color: '#2A2A28',
                paddingRight: 40,
              }}
            >
              {ticker}
              <span style={{ color: 'var(--v5-acc)' }}>●</span>{' '}
            </span>
          ))}
        </div>
      </div>

      <div
        style={{
          height: 48,
          display: 'flex',
          alignItems: 'center',
          padding: '0 16px',
          gap: 8,
          borderBottom: '1px solid var(--v5-bor)',
        }}
      >
        <Brand inverted />
        <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 4 }}>
          <LiveDot color="var(--v5-acc)" />
          <span
            className="v5-mono"
            style={{ fontSize: 8, color: 'var(--v5-acc)' }}
          >
            LIVE
          </span>
        </div>
      </div>

      <div style={{ flex: 1, overflowY: 'auto', padding: '6px 0' }}>
        {NAV.map((item) => {
          const isActive = active === item.id;
          const isHover = hover === item.id;
          return (
            <button
              key={item.id}
              type="button"
              onClick={() => onNavigate(item.id as ScreenId)}
              onMouseEnter={() => setHover(item.id)}
              onMouseLeave={() => setHover(null)}
              aria-current={isActive ? 'page' : undefined}
              style={{
                display: 'flex',
                alignItems: 'center',
                width: '100%',
                height: 36,
                cursor: 'pointer',
                borderLeft: `2px solid ${isActive ? 'var(--v5-acc)' : 'transparent'}`,
                borderTop: 0,
                borderRight: 0,
                borderBottom: 0,
                background: isActive
                  ? 'oklch(0.72 0.14 175 / 0.07)'
                  : isHover
                  ? 'rgba(255, 255, 255, 0.02)'
                  : 'transparent',
                transition: 'all 0.1s',
                textAlign: 'left',
                padding: 0,
              }}
            >
              <div style={{ width: 32, display: 'flex', justifyContent: 'center', flexShrink: 0 }}>
                <V5NamedIcon
                  name={item.icon}
                  size={13}
                  color={isActive ? 'var(--v5-acc)' : isHover ? '#555' : '#2E2E2A'}
                />
              </div>
              <span
                className="v5-mono"
                style={{
                  fontSize: 8,
                  color: isActive ? 'var(--v5-acc)' : '#252523',
                  letterSpacing: 0.5,
                  marginRight: 6,
                  flexShrink: 0,
                  width: 20,
                }}
              >
                {item.n}
              </span>
              <span
                style={{
                  fontSize: 11,
                  fontWeight: isActive ? 600 : 400,
                  color: isActive ? '#fff' : isHover ? '#777' : '#3E3E3A',
                  letterSpacing: '-0.1px',
                }}
              >
                {item.label}
              </span>
              {isActive && (
                <div
                  aria-hidden="true"
                  style={{
                    marginLeft: 'auto',
                    marginRight: 10,
                    width: 4,
                    height: 4,
                    borderRadius: '50%',
                    background: 'var(--v5-acc)',
                  }}
                />
              )}
            </button>
          );
        })}
      </div>

      <AccountFooter userLabel={userLabel} userEmail={userEmail} inverted />
    </nav>
  );
}

export function V5Sidebar(props: SidebarProps) {
  const { isVoid } = useV5Theme();
  return isVoid ? <VoidSidebar {...props} /> : <MarbleSidebar {...props} />;
}
