import NumberFlow from '@number-flow/react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, Row, SectionLabel, StatCard, V5NamedIcon } from '../primitives';
import { useV5Toast } from '../Toast';
import { api, type CreditTransaction } from '../../api/client';
import type { ScreenId } from '../nav';

const TX_TYPE_LABEL: Record<CreditTransaction['type'], string> = {
  grant: 'Gutschrift',
  deduction: 'Abbuchung',
  refund: 'Rückerstattung',
  monthly_refill: 'Monatliche Aufladung',
};

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString('de-DE', { day: '2-digit', month: '2-digit', year: 'numeric' });
}

export function CreditsV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();

  const { data: credits, isLoading, isError } = useQuery({
    queryKey: ['credits'],
    queryFn: async () => {
      const res = await api.getUserCredits();
      return res.data;
    },
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  });

  const buyMutation = useMutation({
    mutationFn: () => api.createCheckout(10),
    onSuccess: (res) => {
      if (res.data?.checkout_url) {
        toast('Weiterleitung zur Kasse…', 'info');
        window.location.href = res.data.checkout_url;
      } else {
        toast('Keine Checkout-URL erhalten', 'error');
      }
      qc.invalidateQueries({ queryKey: ['credits'] });
    },
    onError: () => toast('Credits kaufen fehlgeschlagen', 'error'),
  });

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
        {[240, 90, 200].map((h, i) => (
          <div key={i} style={{ height: h, borderRadius: 14, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.1}s` }} />
        ))}
      </div>
    );
  }

  if (isError || !credits) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 28, textAlign: 'center', maxWidth: 360 }}>
          <V5NamedIcon name="x" size={26} color="var(--v5-err)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Fehler beim Laden</div>
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>Credits konnten nicht geladen werden.</div>
        </Card>
      </div>
    );
  }

  const balance = credits.balance;
  const quota = credits.monthly_quota;
  const used = Math.max(0, quota - balance);
  const pct = quota > 0 ? Math.round((balance / quota) * 100) : 0;
  const txs = credits.transactions ?? [];

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <Card
        className="v5-ani"
        style={{
          padding: 22,
          background: 'linear-gradient(145deg, var(--v5-acc-muted), transparent)',
          border: '1px solid color-mix(in oklch, var(--v5-acc) 30%, transparent)',
          display: 'flex',
          flexDirection: 'column',
          gap: 12,
        }}
      >
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: 12 }}>
          <div>
            <SectionLabel>Credits</SectionLabel>
            <div
              className="v5-mono"
              style={{ fontSize: 44, fontWeight: 800, color: 'var(--v5-acc)', letterSpacing: -2, lineHeight: 1 }}
            >
              <NumberFlow value={balance} />
            </div>
            <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 4 }}>
              von {quota} · {credits.last_refilled ? `Zuletzt aufgeladen ${formatDate(credits.last_refilled)}` : 'kein Ablauf'}
            </div>
          </div>
          <button
            type="button"
            disabled={buyMutation.isPending}
            onClick={() => buyMutation.mutate()}
            className="v5-btn"
            style={{
              padding: '9px 18px',
              borderRadius: 10,
              background: 'var(--v5-acc)',
              color: 'var(--v5-accent-fg)',
              border: 'none',
              fontSize: 12,
              fontWeight: 600,
              cursor: buyMutation.isPending ? 'default' : 'pointer',
              opacity: buyMutation.isPending ? 0.7 : 1,
              whiteSpace: 'nowrap',
            }}
          >
            {buyMutation.isPending ? 'Weiterleitung…' : '+ Credits kaufen'}
          </button>
        </div>
        <div>
          <div
            style={{ height: 4, background: 'color-mix(in oklch, var(--v5-acc) 18%, transparent)', borderRadius: 4 }}
            aria-label={`${balance} von ${quota} Credits`}
            role="meter"
            aria-valuenow={balance}
            aria-valuemin={0}
            aria-valuemax={Math.max(quota, balance)}
          >
            <div style={{ height: '100%', width: `${Math.min(100, pct)}%`, background: 'var(--v5-acc)', borderRadius: 4, transition: 'width 0.8s ease' }} />
          </div>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 4, fontSize: 10, color: 'var(--v5-mut)' }}>
            <span>{pct}% verbleibend</span>
            <span>{used} verwendet</span>
          </div>
        </div>
      </Card>

      <div className="v5-ani" style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 10, animationDelay: '0.06s' }}>
        <StatCard label="Monatl. Kontingent" value={<NumberFlow value={quota} />} icon="credit" delay={0} />
        <StatCard label="Verbraucht" value={<NumberFlow value={used} />} icon="trend" delay={0.06} />
        <StatCard
          label="Letzte Aufladung"
          value={credits.last_refilled ? formatDate(credits.last_refilled) : '—'}
          icon="cal"
          delay={0.12}
        />
      </div>

      <Card className="v5-ani" style={{ overflow: 'hidden', animationDelay: '0.12s' }}>
        <div style={{ padding: '12px 18px 8px' }}>
          <SectionLabel>Transaktionen</SectionLabel>
        </div>
        {txs.length === 0 ? (
          <div style={{ padding: '28px 18px', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 8 }}>
            <V5NamedIcon name="list" size={24} color="var(--v5-mut)" />
            <div style={{ fontSize: 12, color: 'var(--v5-mut)' }}>Keine Transaktionen</div>
          </div>
        ) : (
          txs.map((tx, i) => {
            const positive = tx.amount > 0;
            const label = TX_TYPE_LABEL[tx.type] ?? tx.type;
            return (
              <Row
                key={tx.id}
                label={label}
                sub={tx.description ?? formatDate(tx.created_at)}
                last={i === txs.length - 1}
              >
                <div style={{ textAlign: 'right' }}>
                  <div
                    className="v5-mono"
                    style={{ fontSize: 13, fontWeight: 700, color: positive ? 'var(--v5-ok)' : 'var(--v5-err)' }}
                  >
                    {positive ? '+' : ''}{tx.amount}
                  </div>
                </div>
              </Row>
            );
          })
        )}
      </Card>
    </div>
  );
}
