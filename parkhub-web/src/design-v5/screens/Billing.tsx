import NumberFlow from '@number-flow/react';
import { useQuery } from '@tanstack/react-query';
import { Badge, Card, V5NamedIcon, type BadgeVariant } from '../primitives';
import { api, type PaymentHistoryEntry } from '../../api/client';
import type { ScreenId } from '../nav';

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString('de-DE', { day: '2-digit', month: '2-digit', year: 'numeric' });
}

function formatMoney(amount: number, currency: string): string {
  return new Intl.NumberFormat('de-DE', { style: 'currency', currency: currency || 'EUR' }).format(amount / 100);
}

function statusVariant(s: PaymentHistoryEntry['status']): BadgeVariant {
  switch (s) {
    case 'completed': return 'success';
    case 'pending': return 'warning';
    case 'failed': return 'error';
    default: return 'gray';
  }
}

const STATUS_LABEL: Record<PaymentHistoryEntry['status'], string> = {
  completed: 'Bezahlt', pending: 'Ausstehend', failed: 'Fehlgeschlagen', expired: 'Abgelaufen',
};

export function BillingV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const { data: history = [], isLoading, isError } = useQuery({
    queryKey: ['billing-history'],
    queryFn: async () => {
      const res = await api.getPaymentHistory();
      if (!res.success) throw new Error(res.error?.message ?? 'Zahlungsverlauf konnte nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
  });

  const { data: config } = useQuery({
    queryKey: ['billing-config'],
    queryFn: async () => {
      const res = await api.getStripeConfig();
      if (!res.success) throw new Error(res.error?.message ?? 'Konfiguration fehlt');
      return res.data;
    },
    staleTime: 60_000,
  });

  const totalPaid = history
    .filter((h) => h.status === 'completed')
    .reduce((sum, h) => sum + h.amount, 0);
  const totalCredits = history
    .filter((h) => h.status === 'completed')
    .reduce((sum, h) => sum + h.credits, 0);

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto' }}>
        {[0, 1, 2].map((i) => (
          <div key={i} style={{ height: 60, borderRadius: 12, background: 'var(--v5-sur2)', marginBottom: 8, animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.1}s` }} />
        ))}
      </div>
    );
  }

  if (isError) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 28, textAlign: 'center', maxWidth: 360 }}>
          <V5NamedIcon name="x" size={26} color="var(--v5-err)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Fehler beim Laden</div>
        </Card>
      </div>
    );
  }

  const currency = history[0]?.currency ?? 'EUR';

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Abrechnung</div>

      <div className="v5-ani" style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: 10, animationDelay: '0.06s' }}>
        <Card style={{ padding: 14 }}>
          <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.3, color: 'var(--v5-mut)', textTransform: 'uppercase' }}>Gesamt bezahlt</div>
          <div className="v5-mono" style={{ fontSize: 22, fontWeight: 700, color: 'var(--v5-txt)', marginTop: 4 }}>{formatMoney(totalPaid, currency)}</div>
        </Card>
        <Card style={{ padding: 14 }}>
          <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.3, color: 'var(--v5-mut)', textTransform: 'uppercase' }}>Credits erworben</div>
          <div className="v5-mono" style={{ fontSize: 22, fontWeight: 700, color: 'var(--v5-txt)', marginTop: 4 }}><NumberFlow value={totalCredits} /></div>
        </Card>
        <Card style={{ padding: 14 }}>
          <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.3, color: 'var(--v5-mut)', textTransform: 'uppercase' }}>Stripe</div>
          <div style={{ marginTop: 8 }}>
            <Badge variant={config?.configured ? 'success' : 'gray'} dot>
              {config?.configured ? 'Konfiguriert' : 'Nicht konfiguriert'}
            </Badge>
          </div>
        </Card>
      </div>

      <Card className="v5-ani" style={{ padding: 16, animationDelay: '0.12s' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 10 }}>
          <span style={{ fontWeight: 600, fontSize: 13, color: 'var(--v5-txt)' }}>Zahlungsverlauf</span>
          <Badge variant="gray">{history.length}</Badge>
        </div>
        {history.length === 0 ? (
          <div style={{ padding: 24, textAlign: 'center', fontSize: 12, color: 'var(--v5-mut)' }}>
            Noch keine Zahlungen
          </div>
        ) : (
          <div>
            <div className="v5-mono" style={{
              display: 'grid', gridTemplateColumns: '110px 1fr 100px 120px 110px',
              padding: '6px 4px', fontSize: 9, letterSpacing: 1.2, textTransform: 'uppercase',
              color: 'var(--v5-mut)', borderBottom: '1px solid var(--v5-bor)',
            }}>
              <span>Datum</span><span>ID</span><span>Credits</span><span>Betrag</span><span>Status</span>
            </div>
            {history.map((h, i) => (
              <div
                key={h.id} data-testid="billing-row"
                style={{
                  display: 'grid', gridTemplateColumns: '110px 1fr 100px 120px 110px',
                  padding: '10px 4px', alignItems: 'center',
                  borderBottom: i < history.length - 1 ? '1px solid var(--v5-bor)' : 'none',
                  fontSize: 11, color: 'var(--v5-txt)',
                }}
              >
                <span>{formatDate(h.created_at)}</span>
                <span className="v5-mono" style={{ fontSize: 10, color: 'var(--v5-mut)' }} title={h.id}>{h.id.slice(0, 14)}…</span>
                <span className="v5-mono"><NumberFlow value={h.credits} /></span>
                <span className="v5-mono" style={{ fontWeight: 500 }}>{formatMoney(h.amount, h.currency)}</span>
                <Badge variant={statusVariant(h.status)} dot>{STATUS_LABEL[h.status]}</Badge>
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
