import { useState } from 'react';
import NumberFlow from '@number-flow/react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge, Card, V5NamedIcon } from '../primitives';
import { useV5Toast } from '../Toast';
import { api, type Notification } from '../../api/client';
import type { ScreenId } from '../nav';

type FilterKey = 'alle' | 'ungelesen' | 'gelesen';

const FILTERS: { key: FilterKey; label: string }[] = [
  { key: 'alle', label: 'Alle' },
  { key: 'ungelesen', label: 'Ungelesen' },
  { key: 'gelesen', label: 'Gelesen' },
];

function formatWhen(iso: string): string {
  return new Date(iso).toLocaleString('de-DE', {
    day: '2-digit', month: '2-digit', year: 'numeric',
    hour: '2-digit', minute: '2-digit',
  });
}

export function BenachrichtigungenV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();
  const [filter, setFilter] = useState<FilterKey>('alle');

  const { data: items = [], isLoading, isError } = useQuery({
    queryKey: ['benachrichtigungen'],
    queryFn: async () => {
      const res = await api.getNotifications();
      if (!res.success) throw new Error(res.error?.message ?? 'Benachrichtigungen konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
  });

  const markRead = useMutation({
    mutationFn: async (id: string) => {
      const res = await api.markNotificationRead(id);
      if (!res.success) throw new Error(res.error?.message ?? 'Markieren fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['benachrichtigungen'] });
      toast('Als gelesen markiert', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Markieren fehlgeschlagen', 'error'),
  });

  const markAll = useMutation({
    mutationFn: async () => {
      const res = await api.markAllNotificationsRead();
      if (!res.success) throw new Error(res.error?.message ?? 'Aktion fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['benachrichtigungen'] });
      toast('Alle als gelesen markiert', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Aktion fehlgeschlagen', 'error'),
  });

  const filtered = items.filter((n: Notification) => {
    if (filter === 'ungelesen') return !n.read;
    if (filter === 'gelesen') return n.read;
    return true;
  });
  const unreadCount = items.filter((n) => !n.read).length;

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

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Benachrichtigungen</span>
          <Badge variant="gray"><NumberFlow value={items.length} /></Badge>
          {unreadCount > 0 && <Badge variant="primary" dot>{unreadCount} ungelesen</Badge>}
        </div>
        <button
          type="button"
          disabled={unreadCount === 0 || markAll.isPending}
          onClick={() => markAll.mutate()}
          data-testid="benach-mark-all"
          style={{
            padding: '7px 14px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)',
            border: 'none', fontSize: 11, fontWeight: 600,
            cursor: unreadCount > 0 && !markAll.isPending ? 'pointer' : 'not-allowed',
            opacity: unreadCount > 0 && !markAll.isPending ? 1 : 0.5,
          }}
        >
          Alle als gelesen
        </button>
      </div>

      <div className="v5-ani" role="group" aria-label="Filter" style={{ display: 'flex', gap: 6, flexWrap: 'wrap', animationDelay: '0.06s' }}>
        {FILTERS.map((f) => {
          const active = filter === f.key;
          return (
            <button
              key={f.key} type="button" aria-pressed={active} onClick={() => setFilter(f.key)}
              style={{
                padding: '5px 12px', borderRadius: 999, fontSize: 11, fontWeight: 500, cursor: 'pointer',
                border: `1.5px solid ${active ? 'var(--v5-acc)' : 'var(--v5-bor)'}`,
                background: active ? 'var(--v5-acc-muted)' : 'transparent',
                color: active ? 'var(--v5-acc)' : 'var(--v5-mut)',
              }}
            >{f.label}</button>
          );
        })}
      </div>

      {filtered.length === 0 ? (
        <Card className="v5-ani" style={{ padding: 36, textAlign: 'center', animationDelay: '0.12s' }}>
          <V5NamedIcon name="bell" size={20} color="var(--v5-mut)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Keine Benachrichtigungen</div>
        </Card>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {filtered.map((n, i) => {
            const busy = markRead.isPending && markRead.variables === n.id;
            return (
              <Card
                key={n.id} data-testid="benach-row"
                className="v5-ani"
                style={{
                  padding: 14, display: 'flex', alignItems: 'flex-start', gap: 12,
                  animationDelay: `${0.1 + i * 0.03}s`,
                  borderLeft: n.read ? undefined : '3px solid var(--v5-acc)',
                }}
              >
                <div style={{
                  width: 32, height: 32, borderRadius: 8,
                  background: n.read ? 'var(--v5-sur2)' : 'var(--v5-acc-muted)',
                  display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0,
                }}>
                  <V5NamedIcon name="bell" size={14} color={n.read ? 'var(--v5-mut)' : 'var(--v5-acc)'} />
                </div>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ display: 'flex', alignItems: 'baseline', gap: 8 }}>
                    <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--v5-txt)' }}>{n.title}</span>
                    <span style={{ fontSize: 10, color: 'var(--v5-mut)' }}>{formatWhen(n.created_at)}</span>
                  </div>
                  <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 3 }}>{n.message}</div>
                </div>
                {!n.read && (
                  <button
                    type="button" disabled={busy}
                    aria-label={`Benachrichtigung ${n.id} als gelesen markieren`}
                    onClick={() => markRead.mutate(n.id)}
                    style={{
                      padding: '4px 10px', borderRadius: 7, background: 'var(--v5-acc-muted)',
                      border: 'none', fontSize: 10, fontWeight: 500, color: 'var(--v5-acc)',
                      cursor: busy ? 'default' : 'pointer', opacity: busy ? 0.5 : 1, flexShrink: 0,
                    }}
                  >{busy ? '…' : 'Gelesen'}</button>
                )}
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}
