import { useState } from 'react';
import NumberFlow from '@number-flow/react';
import { useQuery } from '@tanstack/react-query';
import { Badge, Card, V5NamedIcon } from '../primitives';
import { api } from '../../api/client';
import type { ScreenId } from '../nav';

function formatWhen(iso: string): string {
  return new Date(iso).toLocaleString('de-DE', {
    day: '2-digit', month: '2-digit', year: 'numeric',
    hour: '2-digit', minute: '2-digit',
  });
}

export function AuditV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const [page, setPage] = useState(1);
  const [actionFilter, setActionFilter] = useState('');
  const [userFilter, setUserFilter] = useState('');
  const PER_PAGE = 25;

  const [appliedAction, setAppliedAction] = useState('');
  const [appliedUser, setAppliedUser] = useState('');

  const { data, isLoading, isError } = useQuery({
    queryKey: ['audit', page, appliedAction, appliedUser],
    queryFn: async () => {
      const res = await api.getAuditLog({
        page,
        per_page: PER_PAGE,
        action: appliedAction || undefined,
        user: appliedUser || undefined,
      });
      if (!res.success) throw new Error(res.error?.message ?? 'Audit-Log konnte nicht geladen werden');
      return res.data ?? { entries: [], total: 0, page: 1, per_page: PER_PAGE, total_pages: 1 };
    },
    staleTime: 15_000,
    placeholderData: (prev) => prev,
  });

  function applyFilters() {
    setAppliedAction(actionFilter);
    setAppliedUser(userFilter);
    setPage(1);
  }

  if (isLoading && !data) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto' }}>
        {[0, 1, 2].map((i) => (
          <div key={i} style={{ height: 50, borderRadius: 10, background: 'var(--v5-sur2)', marginBottom: 6, animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.05}s` }} />
        ))}
      </div>
    );
  }

  if (isError || !data) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 28, textAlign: 'center', maxWidth: 360 }}>
          <V5NamedIcon name="x" size={26} color="var(--v5-err)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Fehler beim Laden</div>
        </Card>
      </div>
    );
  }

  const entries = data.entries ?? [];

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Audit-Log</span>
          <Badge variant="gray"><NumberFlow value={data.total} /></Badge>
        </div>
        <span style={{ fontSize: 11, color: 'var(--v5-mut)' }}>
          Seite {data.page} / {Math.max(1, data.total_pages)}
        </span>
      </div>

      <Card className="v5-ani" style={{ padding: 12, display: 'flex', gap: 8, flexWrap: 'wrap', alignItems: 'flex-end', animationDelay: '0.06s' }}>
        <label style={{ display: 'flex', flexDirection: 'column', gap: 4, flex: '1 1 160px' }}>
          <span style={{ fontSize: 10, color: 'var(--v5-mut)' }}>Aktion</span>
          <input
            data-testid="audit-action"
            placeholder="z. B. login"
            value={actionFilter}
            onChange={(e) => setActionFilter(e.target.value)}
            style={{ padding: '6px 10px', borderRadius: 8, background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 11, outline: 'none', fontFamily: 'inherit' }}
          />
        </label>
        <label style={{ display: 'flex', flexDirection: 'column', gap: 4, flex: '1 1 160px' }}>
          <span style={{ fontSize: 10, color: 'var(--v5-mut)' }}>Nutzer</span>
          <input
            data-testid="audit-user"
            placeholder="username"
            value={userFilter}
            onChange={(e) => setUserFilter(e.target.value)}
            style={{ padding: '6px 10px', borderRadius: 8, background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 11, outline: 'none', fontFamily: 'inherit' }}
          />
        </label>
        <button
          type="button" onClick={applyFilters} data-testid="audit-apply"
          style={{ padding: '7px 14px', borderRadius: 8, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)', border: 'none', fontSize: 11, fontWeight: 600, cursor: 'pointer' }}
        >Filtern</button>
        <button
          type="button"
          onClick={() => { setActionFilter(''); setUserFilter(''); setAppliedAction(''); setAppliedUser(''); setPage(1); }}
          data-testid="audit-reset"
          style={{ padding: '7px 14px', borderRadius: 8, background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 11, fontWeight: 500, cursor: 'pointer' }}
        >Zurücksetzen</button>
      </Card>

      {entries.length === 0 ? (
        <Card className="v5-ani" style={{ padding: 36, textAlign: 'center', animationDelay: '0.12s' }}>
          <V5NamedIcon name="shield" size={20} color="var(--v5-mut)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Keine Einträge</div>
        </Card>
      ) : (
        <Card className="v5-ani" style={{ overflow: 'hidden', animationDelay: '0.12s' }}>
          <div className="v5-mono" style={{
            display: 'grid', gridTemplateColumns: '140px 110px 130px 1fr',
            padding: '6px 14px', fontSize: 9, letterSpacing: 1.2, textTransform: 'uppercase',
            color: 'var(--v5-mut)', borderBottom: '1px solid var(--v5-bor)',
          }}>
            <span>Zeit</span><span>Nutzer</span><span>Aktion</span><span>Details</span>
          </div>
          {entries.map((e, i) => (
            <div key={e.id} data-testid="audit-row" style={{
              display: 'grid', gridTemplateColumns: '140px 110px 130px 1fr',
              padding: '8px 14px', alignItems: 'center', gap: 6,
              borderBottom: i < entries.length - 1 ? '1px solid var(--v5-bor)' : 'none',
              fontSize: 11,
            }}>
              <span className="v5-mono" style={{ fontSize: 10, color: 'var(--v5-mut)' }}>{formatWhen(e.timestamp)}</span>
              <span style={{ color: 'var(--v5-txt)' }}>{e.username ?? '—'}</span>
              <Badge variant="info">{e.event_type}</Badge>
              <span className="v5-mono" style={{ fontSize: 10, color: 'var(--v5-mut)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }} title={e.details ?? ''}>
                {e.details ?? '—'}
              </span>
            </div>
          ))}
        </Card>
      )}

      <div className="v5-ani" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', animationDelay: '0.18s' }}>
        <button
          type="button" disabled={page <= 1}
          onClick={() => setPage((p) => Math.max(1, p - 1))}
          data-testid="audit-prev"
          style={{ padding: '7px 14px', borderRadius: 8, background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 11, fontWeight: 500, cursor: page > 1 ? 'pointer' : 'not-allowed', opacity: page > 1 ? 1 : 0.5 }}
        >← Zurück</button>
        <span style={{ fontSize: 11, color: 'var(--v5-mut)' }}>
          {data.entries.length > 0 ? `${(data.page - 1) * data.per_page + 1}–${(data.page - 1) * data.per_page + data.entries.length}` : '0'} von {data.total}
        </span>
        <button
          type="button" disabled={page >= data.total_pages}
          onClick={() => setPage((p) => Math.min(data.total_pages, p + 1))}
          data-testid="audit-next"
          style={{ padding: '7px 14px', borderRadius: 8, background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 11, fontWeight: 500, cursor: page < data.total_pages ? 'pointer' : 'not-allowed', opacity: page < data.total_pages ? 1 : 0.5 }}
        >Weiter →</button>
      </div>
    </div>
  );
}
