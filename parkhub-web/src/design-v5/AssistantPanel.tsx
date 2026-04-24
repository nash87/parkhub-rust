import { Badge, Divider, V5NamedIcon } from './primitives';
import { useV5Toast } from './Toast';

type SuggestionType = 'booking' | 'ev' | 'swap' | 'info';

interface Suggestion {
  title: string;
  body: string;
  action: string;
  type: SuggestionType;
}

const TYPE_COLOR: Record<SuggestionType, string> = {
  booking: 'var(--v5-acc)',
  ev: 'var(--v5-ev)',
  swap: 'var(--v5-purple)',
  info: 'var(--v5-ok)',
};

const DEFAULT_SUGGESTIONS: readonly Suggestion[] = [
  {
    title: 'Beste Buchungszeit',
    body: 'Morgen, Di 09:00–17:00 in Zone A hat nur 23% Auslastung.',
    action: 'Jetzt buchen',
    type: 'booking',
  },
  {
    title: 'EV empfohlen',
    body: 'Ihr Tesla Model 3 sollte heute geladen werden. Station C-02 ist frei.',
    action: 'EV-Laden',
    type: 'ev',
  },
  {
    title: 'Tausch möglich',
    body: 'Anna K. bietet A-05 für Mo an — Ihr perfekter Slot.',
    action: 'Annehmen',
    type: 'swap',
  },
  {
    title: 'CO₂ Tipp',
    body: 'ÖPNV am Donnerstag spart 2.4 kg CO₂. Soll ich abmelden?',
    action: 'Abmelden',
    type: 'info',
  },
];

const DEFAULT_STATS: readonly { label: string; value: string; color: string }[] = [
  { label: 'Buchungsrate', value: '94%', color: 'var(--v5-ok)' },
  { label: 'Ø Auslastung', value: '73%', color: 'var(--v5-acc)' },
  { label: 'CO₂ gespart', value: '142 kg', color: 'var(--v5-ok)' },
];

export function V5AssistantPanel({
  open,
  suggestions = DEFAULT_SUGGESTIONS,
  stats = DEFAULT_STATS,
}: {
  open: boolean;
  suggestions?: readonly Suggestion[];
  stats?: readonly { label: string; value: string; color: string }[];
}) {
  const toast = useV5Toast();
  if (!open) return null;

  return (
    <aside
      aria-label="Assistent"
      className="v5-aniR"
      style={{
        width: 260,
        borderLeft: '1px solid var(--v5-bor)',
        background: 'var(--v5-sur)',
        display: 'flex',
        flexDirection: 'column',
        flexShrink: 0,
        overflow: 'hidden',
      }}
    >
      <div
        style={{
          height: 52,
          borderBottom: '1px solid var(--v5-bor)',
          display: 'flex',
          alignItems: 'center',
          padding: '0 16px',
          gap: 7,
        }}
      >
        <V5NamedIcon name="assistant" size={14} color="var(--v5-acc)" />
        <span
          style={{
            fontSize: 13,
            fontWeight: 700,
            color: 'var(--v5-txt)',
            letterSpacing: '-0.2px',
          }}
        >
          Assistent
        </span>
        <Badge variant="primary">Beta</Badge>
        <span style={{ marginLeft: 'auto' }}>
          <Badge variant="purple">Lokal</Badge>
        </span>
      </div>

      <div
        style={{
          flex: 1,
          overflowY: 'auto',
          padding: 12,
          display: 'flex',
          flexDirection: 'column',
          gap: 8,
        }}
      >
        <div
          className="v5-mono"
          style={{
            fontSize: 10,
            color: 'var(--v5-mut)',
            letterSpacing: 1.2,
            textTransform: 'uppercase',
            marginBottom: 2,
          }}
        >
          Vorschläge
        </div>
        {suggestions.map((s, i) => {
          const color = TYPE_COLOR[s.type];
          return (
            <article
              key={s.title}
              style={{
                background: 'var(--v5-sur2)',
                border: '1px solid var(--v5-bor)',
                borderRadius: 11,
                padding: 12,
                animation: `ph-v5-fadeUp 0.28s ease ${i * 0.07}s both`,
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 5, marginBottom: 6 }}>
                <span
                  aria-hidden="true"
                  style={{
                    width: 6,
                    height: 6,
                    borderRadius: '50%',
                    background: color,
                    display: 'inline-block',
                  }}
                />
                <span style={{ fontSize: 11, fontWeight: 600, color: 'var(--v5-txt)' }}>
                  {s.title}
                </span>
              </div>
              <p
                style={{
                  fontSize: 10,
                  color: 'var(--v5-mut)',
                  lineHeight: 1.55,
                  marginBottom: 8,
                  margin: '0 0 8px',
                }}
              >
                {s.body}
              </p>
              <button
                type="button"
                className="v5-btn"
                onClick={() => toast(`${s.action} ausgeführt`, 'success')}
                style={{
                  padding: '5px 10px',
                  borderRadius: 7,
                  background: `color-mix(in oklch, ${color} 15%, transparent)`,
                  color,
                  border: `1px solid color-mix(in oklch, ${color} 30%, transparent)`,
                  fontSize: 10,
                  fontWeight: 600,
                }}
              >
                {s.action} →
              </button>
            </article>
          );
        })}

        <Divider />

        <div
          className="v5-mono"
          style={{
            fontSize: 10,
            color: 'var(--v5-mut)',
            letterSpacing: 1.2,
            textTransform: 'uppercase',
            marginBottom: 2,
          }}
        >
          Statistiken
        </div>
        {stats.map((s, i) => (
          <div
            key={s.label}
            style={{
              display: 'flex',
              justifyContent: 'space-between',
              fontSize: 11,
              padding: '5px 0',
              borderBottom: i < stats.length - 1 ? '1px solid var(--v5-bor)' : 'none',
            }}
          >
            <span style={{ color: 'var(--v5-mut)' }}>{s.label}</span>
            <span
              className="v5-mono"
              style={{ color: s.color, fontWeight: 700 }}
            >
              {s.value}
            </span>
          </div>
        ))}
      </div>

      <footer
        style={{
          padding: '12px 18px',
          borderTop: '1px solid var(--v5-bor)',
          fontSize: 9,
          fontWeight: 500,
          color: 'var(--v5-mut)',
          letterSpacing: '0.06em',
          textTransform: 'uppercase',
          textAlign: 'center',
          flexShrink: 0,
        }}
      >
        Lokal · Keine Daten verlassen den Server
      </footer>
    </aside>
  );
}
