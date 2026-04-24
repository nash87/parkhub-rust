import { useState } from 'react';
import NumberFlow from '@number-flow/react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge, Card, V5NamedIcon, type BadgeVariant } from '../primitives';
import { useV5Toast } from '../Toast';
import { api, type User } from '../../api/client';
import type { ScreenId } from '../nav';

type FilterKey = 'alle' | 'active' | 'suspended' | 'admin';

const FILTERS: { key: FilterKey; label: string }[] = [
  { key: 'alle', label: 'Alle' },
  { key: 'active', label: 'Aktiv' },
  { key: 'suspended', label: 'Gesperrt' },
  { key: 'admin', label: 'Admins' },
];

function roleVariant(role: User['role']): BadgeVariant {
  switch (role) {
    case 'superadmin': return 'purple';
    case 'admin': return 'info';
    case 'premium': return 'warning';
    default: return 'gray';
  }
}

export function NutzerV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();
  const [filter, setFilter] = useState<FilterKey>('alle');
  const [query, setQuery] = useState('');

  const { data: users = [], isLoading, isError } = useQuery({
    queryKey: ['nutzer'],
    queryFn: async () => {
      const res = await api.adminUsers();
      if (!res.success) throw new Error(res.error?.message ?? 'Nutzer konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
  });

  const toggleActive = useMutation({
    mutationFn: async (payload: { id: string; is_active: boolean }) => {
      const res = await api.adminUpdateUser(payload.id, { is_active: payload.is_active });
      if (!res.success) throw new Error(res.error?.message ?? 'Aktualisierung fehlgeschlagen');
      return res.data;
    },
    onSuccess: (_data, vars) => {
      qc.invalidateQueries({ queryKey: ['nutzer'] });
      toast(vars.is_active ? 'Nutzer aktiviert' : 'Nutzer gesperrt', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Aktualisierung fehlgeschlagen', 'error'),
  });

  const filtered = users
    .filter((u) => {
      if (filter === 'active') return u.is_active;
      if (filter === 'suspended') return !u.is_active;
      if (filter === 'admin') return u.role === 'admin' || u.role === 'superadmin';
      return true;
    })
    .filter((u) =>
      !query.trim() ? true : [u.name, u.email, u.username].some((s) => s?.toLowerCase().includes(query.trim().toLowerCase()))
    );

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto' }}>
        <div style={{ height: 32, width: 220, borderRadius: 8, background: 'var(--v5-sur2)', marginBottom: 14, animation: 'ph-v5-pulse 1.6s ease infinite' }} />
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
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>Nutzer konnten nicht geladen werden.</div>
        </Card>
      </div>
    );
  }

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 8 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Nutzer</span>
          <Badge variant="gray"><NumberFlow value={filtered.length} /></Badge>
        </div>
        <input
          type="search"
          placeholder="Suchen …"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          data-testid="nutzer-search"
          style={{
            padding: '7px 12px', borderRadius: 9, background: 'var(--v5-sur2)',
            border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 12,
            outline: 'none', minWidth: 200, fontFamily: 'inherit',
          }}
        />
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
          <V5NamedIcon name="users" size={20} color="var(--v5-mut)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Keine Nutzer gefunden</div>
        </Card>
      ) : (
        <Card className="v5-ani" style={{ overflow: 'hidden', animationDelay: '0.12s' }}>
          <div className="v5-mono" style={{
            display: 'grid', gridTemplateColumns: '1fr 1.3fr 110px 110px 120px',
            padding: '8px 16px', fontSize: 9, letterSpacing: 1.2, textTransform: 'uppercase',
            color: 'var(--v5-mut)', borderBottom: '1px solid var(--v5-bor)',
          }}>
            <span>Name</span><span>E-Mail</span><span>Rolle</span><span>Status</span><span>Aktion</span>
          </div>
          {filtered.map((u, i) => {
            const isBusy = toggleActive.isPending && toggleActive.variables?.id === u.id;
            return (
              <div
                key={u.id} data-testid="nutzer-row"
                style={{
                  display: 'grid', gridTemplateColumns: '1fr 1.3fr 110px 110px 120px',
                  padding: '10px 16px', alignItems: 'center',
                  borderBottom: i < filtered.length - 1 ? '1px solid var(--v5-bor)' : 'none',
                  gap: 4,
                }}
              >
                <div>
                  <div style={{ fontSize: 12, fontWeight: 500, color: 'var(--v5-txt)' }}>{u.name}</div>
                  <div style={{ fontSize: 10, color: 'var(--v5-mut)' }}>@{u.username}</div>
                </div>
                <span style={{ fontSize: 11, color: 'var(--v5-txt)' }}>{u.email}</span>
                <Badge variant={roleVariant(u.role)}>{u.role}</Badge>
                <Badge variant={u.is_active ? 'success' : 'error'} dot>
                  {u.is_active ? 'Aktiv' : 'Gesperrt'}
                </Badge>
                <button
                  type="button"
                  disabled={isBusy}
                  onClick={() => toggleActive.mutate({ id: u.id, is_active: !u.is_active })}
                  aria-label={u.is_active ? `Nutzer ${u.name} sperren` : `Nutzer ${u.name} aktivieren`}
                  style={{
                    padding: '4px 10px', borderRadius: 7,
                    background: u.is_active
                      ? 'color-mix(in oklch, var(--v5-err) 8%, transparent)'
                      : 'var(--v5-acc-muted)',
                    border: 'none', fontSize: 10, fontWeight: 500,
                    color: u.is_active ? 'var(--v5-err)' : 'var(--v5-acc)',
                    cursor: isBusy ? 'default' : 'pointer', opacity: isBusy ? 0.5 : 1,
                  }}
                >
                  {isBusy ? '…' : u.is_active ? 'Sperren' : 'Aktivieren'}
                </button>
              </div>
            );
          })}
        </Card>
      )}
    </div>
  );
}
