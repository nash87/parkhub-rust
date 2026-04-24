import { useState } from 'react';
import NumberFlow from '@number-flow/react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge, Card, V5NamedIcon, type BadgeVariant } from '../primitives';
import { useV5Toast } from '../Toast';
import { api, type Announcement } from '../../api/client';
import type { ScreenId } from '../nav';

/**
 * Benachrichtigungen — admin view of tenant-wide announcements.
 *
 * This screen is the admin authoring surface for the broadcast banner
 * that surfaces to every user via the bell dropdown + top banner. It
 * talks to `/api/v1/admin/announcements` (list/create/delete), NOT the
 * per-user notification inbox which is surfaced elsewhere.
 *
 * Codex #376: the v5 draft shipped this admin-nav entry wired to
 * `getNotifications` / `markNotificationRead`, so admins could only
 * read-state their own alerts and had no way to compose or retire a
 * system-wide announcement from the new shell.
 */

function severityVariant(sev: string): BadgeVariant {
  switch (sev) {
    case 'critical': return 'error';
    case 'warning': return 'warning';
    case 'success': return 'success';
    case 'info':
    default:
      return 'info';
  }
}

function formatWhen(iso: string): string {
  return new Date(iso).toLocaleString('de-DE', {
    day: '2-digit', month: '2-digit', year: 'numeric',
    hour: '2-digit', minute: '2-digit',
  });
}

const SEVERITIES: { value: string; label: string }[] = [
  { value: 'info', label: 'Info' },
  { value: 'success', label: 'Erfolg' },
  { value: 'warning', label: 'Warnung' },
  { value: 'critical', label: 'Kritisch' },
];

export function BenachrichtigungenV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();

  const { data: items = [], isLoading, isError } = useQuery({
    queryKey: ['admin-announcements'],
    queryFn: async () => {
      const res = await api.adminListAnnouncements();
      if (!res.success) throw new Error(res.error?.message ?? 'Ankündigungen konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
  });

  const [title, setTitle] = useState('');
  const [message, setMessage] = useState('');
  const [severity, setSeverity] = useState('info');
  const [active, setActive] = useState(true);

  const createMutation = useMutation({
    mutationFn: async () => {
      const res = await api.adminCreateAnnouncement({
        title: title.trim(),
        message: message.trim(),
        severity,
        active,
      });
      if (!res.success) throw new Error(res.error?.message ?? 'Erstellen fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['admin-announcements'] });
      setTitle('');
      setMessage('');
      toast('Ankündigung erstellt', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Erstellen fehlgeschlagen', 'error'),
  });

  const deleteMutation = useMutation({
    mutationFn: async (id: string) => {
      const res = await api.adminDeleteAnnouncement(id);
      if (!res.success) throw new Error(res.error?.message ?? 'Löschen fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['admin-announcements'] });
      toast('Ankündigung gelöscht', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Löschen fehlgeschlagen', 'error'),
  });

  const activeCount = items.filter((a: Announcement) => a.active).length;

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
          <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Ankündigungen</span>
          <Badge variant="gray"><NumberFlow value={items.length} /></Badge>
          {activeCount > 0 && <Badge variant="primary" dot>{activeCount} aktiv</Badge>}
        </div>
      </div>

      <Card className="v5-ani" style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 10, animationDelay: '0.06s' }}>
        <div style={{ fontSize: 12, fontWeight: 600, color: 'var(--v5-txt)' }}>Neue Ankündigung</div>
        <input
          data-testid="benach-new-title"
          type="text"
          placeholder="Titel"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          style={{
            padding: '8px 11px', borderRadius: 9, background: 'var(--v5-sur2)',
            border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 12,
            width: '100%', outline: 'none', boxSizing: 'border-box', fontFamily: 'inherit',
          }}
        />
        <textarea
          data-testid="benach-new-message"
          placeholder="Nachricht"
          value={message}
          onChange={(e) => setMessage(e.target.value)}
          rows={3}
          style={{
            padding: '8px 11px', borderRadius: 9, background: 'var(--v5-sur2)',
            border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 12,
            width: '100%', outline: 'none', boxSizing: 'border-box', fontFamily: 'inherit',
            resize: 'vertical',
          }}
        />
        <div style={{ display: 'flex', alignItems: 'center', gap: 12, flexWrap: 'wrap' }}>
          <div role="group" aria-label="Priorität" style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
            {SEVERITIES.map((s) => {
              const selected = severity === s.value;
              return (
                <button
                  key={s.value} type="button" aria-pressed={selected}
                  onClick={() => setSeverity(s.value)}
                  style={{
                    padding: '5px 12px', borderRadius: 999, fontSize: 11, fontWeight: 500, cursor: 'pointer',
                    border: `1.5px solid ${selected ? 'var(--v5-acc)' : 'var(--v5-bor)'}`,
                    background: selected ? 'var(--v5-acc-muted)' : 'transparent',
                    color: selected ? 'var(--v5-acc)' : 'var(--v5-mut)',
                  }}
                >{s.label}</button>
              );
            })}
          </div>
          <label style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 11, color: 'var(--v5-mut)' }}>
            <input
              type="checkbox"
              checked={active}
              onChange={(e) => setActive(e.target.checked)}
              data-testid="benach-new-active"
            />
            Aktiv
          </label>
          <div style={{ flex: 1 }} />
          <button
            type="button"
            disabled={!title.trim() || !message.trim() || createMutation.isPending}
            onClick={() => createMutation.mutate()}
            data-testid="benach-new-submit"
            style={{
              padding: '7px 14px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)',
              border: 'none', fontSize: 11, fontWeight: 600,
              cursor: (!title.trim() || !message.trim() || createMutation.isPending) ? 'not-allowed' : 'pointer',
              opacity: (!title.trim() || !message.trim() || createMutation.isPending) ? 0.5 : 1,
            }}
          >
            {createMutation.isPending ? 'Erstellt …' : 'Senden'}
          </button>
        </div>
      </Card>

      {items.length === 0 ? (
        <Card className="v5-ani" style={{ padding: 36, textAlign: 'center', animationDelay: '0.12s' }}>
          <V5NamedIcon name="bell" size={20} color="var(--v5-mut)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Keine Ankündigungen</div>
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>
            Erstelle oben eine neue Ankündigung für das gesamte Team.
          </div>
        </Card>
      ) : (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {items.map((a: Announcement, i: number) => {
            const deleting = deleteMutation.isPending && deleteMutation.variables === a.id;
            return (
              <Card
                key={a.id} data-testid="benach-row"
                className="v5-ani"
                style={{
                  padding: 14, display: 'flex', alignItems: 'flex-start', gap: 12,
                  animationDelay: `${0.1 + i * 0.03}s`,
                  borderLeft: a.active ? '3px solid var(--v5-acc)' : undefined,
                  opacity: a.active ? 1 : 0.7,
                }}
              >
                <div style={{
                  width: 32, height: 32, borderRadius: 8,
                  background: a.active ? 'var(--v5-acc-muted)' : 'var(--v5-sur2)',
                  display: 'flex', alignItems: 'center', justifyContent: 'center', flexShrink: 0,
                }}>
                  <V5NamedIcon name="bell" size={14} color={a.active ? 'var(--v5-acc)' : 'var(--v5-mut)'} />
                </div>
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div style={{ display: 'flex', alignItems: 'baseline', gap: 8, flexWrap: 'wrap' }}>
                    <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--v5-txt)' }}>{a.title}</span>
                    <Badge variant={severityVariant(a.severity)} dot>{a.severity}</Badge>
                    {!a.active && <Badge variant="gray">Inaktiv</Badge>}
                    <span style={{ fontSize: 10, color: 'var(--v5-mut)' }}>{formatWhen(a.created_at)}</span>
                  </div>
                  <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 3 }}>{a.message}</div>
                </div>
                <button
                  type="button" disabled={deleting}
                  aria-label={`Ankündigung ${a.id} löschen`}
                  onClick={() => deleteMutation.mutate(a.id)}
                  style={{
                    padding: '4px 10px', borderRadius: 7, background: 'var(--v5-sur2)',
                    border: '1px solid var(--v5-bor)', fontSize: 10, fontWeight: 500, color: 'var(--v5-err)',
                    cursor: deleting ? 'default' : 'pointer', opacity: deleting ? 0.5 : 1, flexShrink: 0,
                  }}
                >{deleting ? '…' : 'Löschen'}</button>
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}
