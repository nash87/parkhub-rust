import NumberFlow from '@number-flow/react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge, Card, V5NamedIcon } from '../primitives';
import { useV5Toast } from '../Toast';
import { api, type Integration } from '../../api/client';
import type { ScreenId } from '../nav';

function formatWhen(iso: string | null): string {
  if (!iso) return '—';
  return new Date(iso).toLocaleDateString('de-DE', { day: '2-digit', month: '2-digit', year: 'numeric' });
}

export function IntegrationsV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();

  const { data: items = [], isLoading, isError } = useQuery({
    queryKey: ['integrations'],
    queryFn: async () => {
      const res = await api.getIntegrations();
      if (!res.success) throw new Error(res.error?.message ?? 'Integrationen konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
  });

  const connect = useMutation({
    mutationFn: async (id: string) => {
      const res = await api.connectIntegration(id);
      if (!res.success) throw new Error(res.error?.message ?? 'Verbindung fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['integrations'] });
      toast('Integration verbunden', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Verbindung fehlgeschlagen', 'error'),
  });

  const disconnect = useMutation({
    mutationFn: async (id: string) => {
      const res = await api.disconnectIntegration(id);
      if (!res.success) throw new Error(res.error?.message ?? 'Trennen fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['integrations'] });
      toast('Integration getrennt', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Trennen fehlgeschlagen', 'error'),
  });

  const connectedCount = items.filter((i: Integration) => i.connected).length;

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto' }}>
        {[0, 1, 2].map((i) => (
          <div key={i} style={{ height: 80, borderRadius: 12, background: 'var(--v5-sur2)', marginBottom: 8, animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.1}s` }} />
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

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Integrationen</span>
        <Badge variant="gray"><NumberFlow value={items.length} /></Badge>
        {connectedCount > 0 && <Badge variant="success" dot>{connectedCount} aktiv</Badge>}
      </div>

      {items.length === 0 ? (
        <Card className="v5-ani" style={{ padding: 36, textAlign: 'center' }}>
          <V5NamedIcon name="key" size={20} color="var(--v5-mut)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Keine Integrationen verfügbar</div>
        </Card>
      ) : (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))', gap: 10 }}>
          {items.map((item, idx) => {
            const busy =
              (connect.isPending && connect.variables === item.id) ||
              (disconnect.isPending && disconnect.variables === item.id);
            return (
              <Card
                key={item.id} data-testid="integrations-card"
                className="v5-ani"
                style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 10, animationDelay: `${idx * 0.04}s` }}
              >
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                    <div style={{
                      width: 36, height: 36, borderRadius: 10,
                      background: item.connected ? 'var(--v5-acc-muted)' : 'var(--v5-sur2)',
                      display: 'flex', alignItems: 'center', justifyContent: 'center',
                    }}>
                      <V5NamedIcon name="key" size={16} color={item.connected ? 'var(--v5-acc)' : 'var(--v5-mut)'} />
                    </div>
                    <div>
                      <div style={{ fontSize: 13, fontWeight: 600, color: 'var(--v5-txt)' }}>{item.name}</div>
                      <div style={{ fontSize: 10, color: 'var(--v5-mut)' }}>{item.provider}</div>
                    </div>
                  </div>
                  <Badge variant={item.connected ? 'success' : 'gray'} dot>
                    {item.connected ? 'Verbunden' : 'Nicht verbunden'}
                  </Badge>
                </div>
                <div style={{ fontSize: 11, color: 'var(--v5-mut)', minHeight: 28 }}>{item.description}</div>
                {item.connected && item.account_label && (
                  <div style={{ fontSize: 10, color: 'var(--v5-mut)' }}>
                    Konto: <span style={{ color: 'var(--v5-txt)' }}>{item.account_label}</span>
                    {' · '}seit {formatWhen(item.connected_at)}
                  </div>
                )}
                <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
                  {item.connected ? (
                    <button
                      type="button" disabled={busy}
                      onClick={() => disconnect.mutate(item.id)}
                      data-testid="integrations-disconnect"
                      style={{
                        padding: '6px 14px', borderRadius: 8,
                        background: 'color-mix(in oklch, var(--v5-err) 8%, transparent)',
                        border: 'none', fontSize: 11, fontWeight: 500, color: 'var(--v5-err)',
                        cursor: busy ? 'default' : 'pointer', opacity: busy ? 0.5 : 1,
                      }}
                    >{busy ? '…' : 'Trennen'}</button>
                  ) : (
                    <button
                      type="button" disabled={busy}
                      onClick={() => connect.mutate(item.id)}
                      data-testid="integrations-connect"
                      style={{
                        padding: '6px 14px', borderRadius: 8,
                        background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)',
                        border: 'none', fontSize: 11, fontWeight: 600,
                        cursor: busy ? 'default' : 'pointer', opacity: busy ? 0.5 : 1,
                      }}
                    >{busy ? '…' : 'Verbinden'}</button>
                  )}
                </div>
              </Card>
            );
          })}
        </div>
      )}

      <div className="v5-ani" style={{ fontSize: 10, color: 'var(--v5-mut)', marginTop: 6 }}>
        Hinweis: OAuth-Flows für neue Provider werden in einer Folge-PR ergänzt.
      </div>
    </div>
  );
}
