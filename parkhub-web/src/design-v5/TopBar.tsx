import { V5NamedIcon } from './primitives';
import { useV5Theme, V5_MODE_LABELS, V5_MODES } from './ThemeProvider';

export function V5TopBar({
  title,
  breadcrumb,
  onOpenCommand,
  onToggleAI,
  aiOpen,
}: {
  title: string;
  breadcrumb: string;
  onOpenCommand: () => void;
  onToggleAI: () => void;
  aiOpen: boolean;
}) {
  const { mode, setMode } = useV5Theme();

  return (
    <header
      style={{
        height: 52,
        borderBottom: '1px solid var(--v5-bor)',
        display: 'flex',
        alignItems: 'center',
        padding: '0 18px',
        gap: 10,
        background: 'var(--v5-sur)',
        flexShrink: 0,
      }}
    >
      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          className="v5-mono"
          style={{
            fontSize: 9,
            color: 'var(--v5-mut)',
            letterSpacing: 1.2,
            marginBottom: 1,
          }}
        >
          {breadcrumb}
        </div>
        <h1
          style={{
            fontSize: 14,
            fontWeight: 700,
            color: 'var(--v5-txt)',
            letterSpacing: '-0.3px',
            margin: 0,
          }}
        >
          {title}
        </h1>
      </div>

      {/* Theme switcher — 3 modes */}
      <div
        role="radiogroup"
        aria-label="Farbthema"
        style={{
          display: 'flex',
          background: 'var(--v5-sur2)',
          border: '1px solid var(--v5-bor)',
          borderRadius: 9,
          overflow: 'hidden',
        }}
      >
        {V5_MODES.map((m) => {
          const active = mode === m;
          return (
            <button
              key={m}
              type="button"
              role="radio"
              aria-checked={active}
              onClick={() => setMode(m)}
              style={{
                padding: '5px 10px',
                background: active ? 'var(--v5-acc)' : 'transparent',
                color: active ? 'var(--v5-accent-fg)' : 'var(--v5-mut)',
                border: 'none',
                fontSize: 10,
                fontWeight: active ? 600 : 400,
                cursor: 'pointer',
                transition: 'all 0.15s',
              }}
            >
              {V5_MODE_LABELS[m]}
            </button>
          );
        })}
      </div>

      <button
        type="button"
        onClick={onOpenCommand}
        aria-label="Befehlspalette öffnen"
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 7,
          background: 'var(--v5-sur2)',
          border: '1px solid var(--v5-bor)',
          borderRadius: 40,
          padding: '0 11px',
          height: 30,
          cursor: 'pointer',
          color: 'var(--v5-mut)',
        }}
      >
        <V5NamedIcon name="search" size={12} color="var(--v5-mut)" />
        <span style={{ fontSize: 11 }}>Suchen…</span>
        <kbd
          className="v5-mono"
          style={{
            fontSize: 9,
            color: 'var(--v5-mut)',
            background: 'var(--v5-bg)',
            border: '1px solid var(--v5-bor)',
            borderRadius: 4,
            padding: '1px 5px',
            marginLeft: 3,
          }}
        >
          ⌘K
        </kbd>
      </button>

      <button
        type="button"
        onClick={onToggleAI}
        aria-pressed={aiOpen}
        aria-label="KI-Assistent umschalten"
        style={{
          height: 30,
          padding: '0 10px',
          borderRadius: 20,
          background: aiOpen ? 'var(--v5-acc-muted)' : 'transparent',
          border: `1px solid ${aiOpen ? 'var(--v5-acc)' : 'var(--v5-bor)'}`,
          display: 'flex',
          alignItems: 'center',
          gap: 5,
          cursor: 'pointer',
          fontSize: 10,
          fontWeight: 600,
          color: aiOpen ? 'var(--v5-acc)' : 'var(--v5-mut)',
          transition: 'all 0.15s',
        }}
      >
        <V5NamedIcon name="ai" size={12} color={aiOpen ? 'var(--v5-acc)' : 'var(--v5-mut)'} />
        AI
      </button>
    </header>
  );
}
