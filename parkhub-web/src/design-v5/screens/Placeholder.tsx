import { Badge, Card, V5NamedIcon } from '../primitives';
import { byId, type ScreenId } from '../nav';

/**
 * Interim screen shown for every v5 route that hasn't been ported yet.
 * Points the user to the existing v4 surface so they aren't blocked —
 * bridges the migration rather than pretending the screen is done.
 */
const LEGACY_ROUTE: Partial<Record<ScreenId, string>> = {
  buchungen: '/bookings',
  buchen: '/book',
  fahrzeuge: '/vehicles',
  kalender: '/calendar',
  karte: '/map',
  credits: '/credits',
  team: '/team',
  rangliste: '/leaderboard',
  ev: '/ev-charging',
  tausch: '/swap',
  einchecken: '/checkin',
  vorhersagen: '/predictions',
  gaestepass: '/guest-pass',
  analytics: '/admin/analytics',
  nutzer: '/admin/users',
  billing: '/admin/billing',
  lobby: '/lobby',
  benachrichtigungen: '/notifications',
  einstellungen: '/settings',
  standorte: '/admin/lots',
  integrations: '/admin/integrations',
  apikeys: '/admin/api-keys',
  audit: '/admin/audit',
  policies: '/admin/policies',
  profil: '/profile',
};

export function PlaceholderV5({ id }: { id: ScreenId }) {
  const meta = byId.get(id);
  const legacy = LEGACY_ROUTE[id];
  const icon = meta?.icon ?? 'home';

  return (
    <div
      style={{
        padding: 16,
        flex: 1,
        overflow: 'auto',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}
    >
      <Card
        className="v5-ani"
        style={{
          padding: 32,
          textAlign: 'center',
          maxWidth: 460,
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: 14,
        }}
      >
        <div
          style={{
            width: 56,
            height: 56,
            borderRadius: 16,
            background: 'var(--v5-acc-muted)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
          }}
        >
          <V5NamedIcon name={icon} size={26} color="var(--v5-acc)" />
        </div>
        <div>
          <div style={{ fontSize: 18, fontWeight: 700, color: 'var(--v5-txt)', letterSpacing: '-0.4px' }}>
            {meta?.label ?? 'Screen'}
          </div>
          <div
            className="v5-mono"
            style={{ fontSize: 10, color: 'var(--v5-mut)', marginTop: 4, letterSpacing: 1.2 }}
          >
            SCREEN {meta?.n ?? '—'} · v5
          </div>
        </div>
        <p style={{ fontSize: 13, color: 'var(--v5-mut)', lineHeight: 1.6, margin: 0 }}>
          Dieser Screen wird im v5-Design noch gebaut. Das bestehende v4-Surface ist weiterhin
          vollständig funktional und über den Link unten direkt erreichbar.
        </p>
        <Badge variant="info">Migration in Arbeit</Badge>
        {legacy && (
          <a
            href={legacy}
            className="v5-btn"
            style={{
              padding: '8px 16px',
              borderRadius: 9,
              background: 'var(--v5-acc)',
              color: 'var(--v5-accent-fg)',
              textDecoration: 'none',
              fontSize: 12,
              fontWeight: 600,
              display: 'inline-flex',
              alignItems: 'center',
              gap: 6,
            }}
          >
            Zum bestehenden Surface →
          </a>
        )}
      </Card>
    </div>
  );
}
