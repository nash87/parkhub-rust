import { useState } from 'react';
import { NAV, type ScreenId } from '../nav';
import { V5NamedIcon } from '../primitives';
import type { SidebarProps } from './MarbleSidebar';

/**
 * MinimalSidebar — icon-only collapsed rail (52px wide).
 *
 * For power users who want maximum content area. Tooltips on hover surface
 * the label so the navigation remains discoverable. The brand square stays
 * pinned at top, account avatar at bottom — both unlabeled.
 */
export function MinimalSidebar({ active, onNavigate, userLabel = 'Administrator' }: SidebarProps) {
  const [hover, setHover] = useState<string | null>(null);

  return (
    <nav
      aria-label="Hauptnavigation"
      data-variant="minimal"
      style={{
        width: 52,
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
          justifyContent: 'center',
          borderBottom: '1px solid var(--v5-bor)',
        }}
      >
        <div
          aria-label="ParkHub"
          title="ParkHub"
          style={{
            width: 28,
            height: 28,
            borderRadius: 8,
            background: 'var(--v5-acc)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#fff',
            fontWeight: 800,
            fontSize: 13,
            fontStyle: 'italic',
          }}
        >
          P
        </div>
      </div>

      <div
        style={{
          flex: 1,
          overflowY: 'auto',
          overflowX: 'visible',
          padding: '6px 0',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: 2,
        }}
      >
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
              onFocus={() => setHover(item.id)}
              onBlur={() => setHover(null)}
              aria-current={isActive ? 'page' : undefined}
              aria-label={item.label}
              title={item.label}
              style={{
                position: 'relative',
                width: 36,
                height: 36,
                borderRadius: 9,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                background: isActive ? 'var(--v5-acc-muted)' : 'transparent',
                border: 0,
                cursor: 'pointer',
                transition: 'background 0.12s',
              }}
            >
              <V5NamedIcon
                name={item.icon}
                size={15}
                color={isActive ? 'var(--v5-acc)' : 'var(--v5-mut)'}
              />
              {isActive && (
                <span
                  aria-hidden="true"
                  style={{
                    position: 'absolute',
                    left: 0,
                    top: 8,
                    bottom: 8,
                    width: 2,
                    borderRadius: 2,
                    background: 'var(--v5-acc)',
                  }}
                />
              )}
              {isHover && (
                <span
                  role="tooltip"
                  style={{
                    position: 'absolute',
                    left: '100%',
                    marginLeft: 8,
                    top: '50%',
                    transform: 'translateY(-50%)',
                    background: 'var(--v5-sur)',
                    color: 'var(--v5-txt)',
                    border: '1px solid var(--v5-bor)',
                    borderRadius: 6,
                    padding: '4px 8px',
                    fontSize: 11,
                    fontWeight: 500,
                    whiteSpace: 'nowrap',
                    boxShadow: 'var(--v5-shadow-card)',
                    zIndex: 50,
                    pointerEvents: 'none',
                  }}
                >
                  {item.label}
                </span>
              )}
            </button>
          );
        })}
      </div>

      <div
        style={{
          height: 46,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          borderTop: '1px solid var(--v5-bor)',
        }}
      >
        <div
          aria-label={userLabel}
          title={userLabel}
          style={{
            width: 28,
            height: 28,
            borderRadius: '50%',
            background: 'linear-gradient(135deg, var(--v5-acc), oklch(0.4 0.1 175))',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#fff',
            fontSize: 11,
            fontWeight: 700,
          }}
        >
          {userLabel.charAt(0)}
        </div>
      </div>
    </nav>
  );
}
