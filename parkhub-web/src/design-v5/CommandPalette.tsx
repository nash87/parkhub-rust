import { useEffect } from 'react';
import { Command } from 'cmdk';
import { NAV, SECTION_HEADINGS, type NavSection, type ScreenId } from './nav';
import { V5NamedIcon } from './primitives';

/**
 * ⌘K command palette — cmdk-powered, v5-themed via the [cmdk-*] selectors in
 * tokens.css. Keyboard navigation, filter, ARIA wiring all delegated to cmdk.
 * Sections ordered: main → fleet → admin (mirrors the sidebar).
 */
export function V5CommandPalette({
  open,
  onClose,
  onNavigate,
}: {
  open: boolean;
  onClose: () => void;
  onNavigate: (id: ScreenId) => void;
}) {
  useEffect(() => {
    if (!open) return;
    const h = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      }
    };
    window.addEventListener('keydown', h);
    return () => window.removeEventListener('keydown', h);
  }, [open, onClose]);

  if (!open) return null;

  const sections: NavSection[] = ['main', 'fleet', 'admin'];

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      onClick={onClose}
      style={{
        position: 'fixed',
        inset: 0,
        background: 'color-mix(in oklch, #000 55%, transparent)',
        backdropFilter: 'blur(4px)',
        zIndex: 1000,
        display: 'flex',
        alignItems: 'flex-start',
        justifyContent: 'center',
        paddingTop: 110,
        animation: 'ph-v5-fadeIn 0.12s ease',
      }}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          width: 540,
          background: 'var(--v5-sur)',
          border: '1px solid var(--v5-bor)',
          borderRadius: 16,
          overflow: 'hidden',
          boxShadow: '0 24px 80px rgba(0, 0, 0, 0.35)',
          animation: 'ph-v5-fadeUp 0.14s ease',
        }}
      >
        <Command label="Screen navigation" loop>
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 9,
              padding: '0 14px',
              borderBottom: '1px solid var(--v5-bor)',
              height: 48,
            }}
          >
            <V5NamedIcon name="search" size={15} color="var(--v5-mut)" />
            <Command.Input
              autoFocus
              placeholder="Screen suchen oder navigieren…"
              style={{ fontSize: 14, padding: '12px 0' }}
            />
            <kbd
              className="v5-mono"
              style={{
                fontSize: 9,
                color: 'var(--v5-mut)',
                background: 'var(--v5-bg)',
                border: '1px solid var(--v5-bor)',
                borderRadius: 5,
                padding: '2px 6px',
              }}
            >
              ESC
            </kbd>
          </div>

          <Command.List>
            <Command.Empty>Keine Treffer.</Command.Empty>
            {sections.map((sec) => {
              const items = NAV.filter((n) => n.section === sec);
              if (!items.length) return null;
              return (
                <Command.Group key={sec} heading={SECTION_HEADINGS[sec]}>
                  {items.map((item) => (
                    <Command.Item
                      key={item.id}
                      value={`${item.label} ${item.n} ${item.id}`}
                      onSelect={() => onNavigate(item.id as ScreenId)}
                    >
                      <div
                        style={{
                          width: 26,
                          height: 26,
                          borderRadius: 8,
                          background: 'var(--v5-sur2)',
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'center',
                          flexShrink: 0,
                        }}
                      >
                        <V5NamedIcon name={item.icon} size={12} color="var(--v5-mut)" />
                      </div>
                      <span style={{ flex: 1, fontWeight: 500 }}>{item.label}</span>
                      <span
                        className="v5-mono"
                        style={{ fontSize: 9, color: 'var(--v5-mut)' }}
                      >
                        {item.n}
                      </span>
                      <V5NamedIcon name="chev" size={11} color="var(--v5-mut)" />
                    </Command.Item>
                  ))}
                </Command.Group>
              );
            })}
          </Command.List>

          <div
            className="v5-mono"
            style={{
              padding: '7px 14px',
              borderTop: '1px solid var(--v5-bor)',
              display: 'flex',
              gap: 12,
              fontSize: 9,
              color: 'var(--v5-mut)',
            }}
          >
            <span>↑↓ navigieren</span>
            <span>↵ öffnen</span>
            <span>ESC schließen</span>
          </div>
        </Command>
      </div>
    </div>
  );
}
