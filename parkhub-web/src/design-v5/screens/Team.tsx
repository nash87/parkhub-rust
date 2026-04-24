import { useMemo } from 'react';
import NumberFlow from '@number-flow/react';
import { useQuery } from '@tanstack/react-query';
import { Badge, Card, V5NamedIcon } from '../primitives';
import { api, type TeamAbsenceEntry } from '../../api/client';
import type { ScreenId } from '../nav';

type TeamMemberStatus = 'booked' | 'absent' | 'available' | 'home';

interface TeamMemberRow {
  name: string;
  department: string;
  status: TeamMemberStatus;
  statusLabel: string;
  slot: string;
  since: string;
  entries: TeamAbsenceEntry[];
}

function inferStatus(absenceType?: string): TeamMemberStatus {
  if (!absenceType) return 'booked';
  if (absenceType === 'homeoffice') return 'home';
  if (absenceType === 'vacation' || absenceType === 'sick' || absenceType === 'other') return 'absent';
  return 'available';
}

function statusLabelFor(status: TeamMemberStatus): string {
  switch (status) {
    case 'booked': return 'Gebucht';
    case 'absent': return 'Abwesend';
    case 'home': return 'Home Office';
    default: return 'Verfügbar';
  }
}

function statusVariant(s: TeamMemberStatus) {
  switch (s) {
    case 'booked': return 'success' as const;
    case 'absent': return 'error' as const;
    case 'home': return 'info' as const;
    default: return 'primary' as const;
  }
}

function groupByUser(list: TeamAbsenceEntry[]): Record<string, TeamAbsenceEntry[]> {
  const groups: Record<string, TeamAbsenceEntry[]> = {};
  for (const entry of list) {
    if (!groups[entry.user_name]) groups[entry.user_name] = [];
    groups[entry.user_name].push(entry);
  }
  return groups;
}

function formatMonthYear(value: string): string {
  return new Date(`${value}T00:00:00`).toLocaleDateString('de-DE', { month: 'short', year: '2-digit' });
}

function formatDate(value: string): string {
  return new Date(`${value}T00:00:00`).toLocaleDateString('de-DE', { day: 'numeric', month: 'short' });
}

function absenceLabel(type: string): string {
  if (type === 'homeoffice') return 'Homeoffice';
  if (type === 'vacation') return 'Urlaub';
  if (type === 'sick') return 'Krank';
  if (type === 'other') return 'Sonstiges';
  return type;
}

export function TeamV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const { data: entries = [], isLoading, isError } = useQuery({
    queryKey: ['team-absences'],
    queryFn: async () => {
      const res = await api.teamAbsences();
      if (!res.success) throw new Error(res.error?.message ?? 'Team konnte nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  });

  const todayStr = useMemo(() => new Date().toISOString().slice(0, 10), []);

  const todayEntries = useMemo(
    () => entries.filter((e) => e.start_date <= todayStr && e.end_date >= todayStr),
    [entries, todayStr],
  );

  const upcomingEntries = useMemo(
    () => entries.filter((e) => e.start_date > todayStr).sort((a, b) => a.start_date.localeCompare(b.start_date)),
    [entries, todayStr],
  );

  const members = useMemo<TeamMemberRow[]>(() => {
    const byUser = groupByUser(entries);
    return Object.entries(byUser)
      .map(([name, userEntries], index) => {
        const current = userEntries.find((e) => e.start_date <= todayStr && e.end_date >= todayStr);
        const next = [...userEntries].sort((a, b) => a.start_date.localeCompare(b.start_date))[0];
        const status = inferStatus(current?.absence_type);
        return {
          name,
          department: `Team ${index + 1}`,
          status,
          statusLabel: statusLabelFor(status),
          slot: status === 'booked' ? `A-0${(index % 8) + 1}` : '—',
          since: next ? formatMonthYear(next.start_date) : '—',
          entries: userEntries,
        };
      })
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [entries, todayStr]);

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto' }}>
        <div style={{ height: 32, width: 200, borderRadius: 8, background: 'var(--v5-sur2)', marginBottom: 14, animation: 'ph-v5-pulse 1.6s ease infinite' }} />
        {[0, 1, 2].map((i) => (
          <div key={i} style={{ height: 70, borderRadius: 12, background: 'var(--v5-sur2)', marginBottom: 8, animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.1}s` }} />
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
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>Team konnte nicht geladen werden.</div>
        </Card>
      </div>
    );
  }

  if (members.length === 0) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 36, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10, maxWidth: 380 }}>
          <div style={{ width: 48, height: 48, borderRadius: 14, background: 'var(--v5-acc-muted)', border: '1.5px dashed color-mix(in oklch, var(--v5-acc) 50%, transparent)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <V5NamedIcon name="users" size={20} color="var(--v5-acc)" />
          </div>
          <div style={{ textAlign: 'center' }}>
            <div style={{ fontWeight: 600, color: 'var(--v5-txt)', fontSize: 13 }}>Noch keine Teamdaten</div>
            <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 3 }}>Sobald Kollegen Abwesenheiten eintragen, erscheinen sie hier.</div>
          </div>
        </Card>
      </div>
    );
  }

  const presentCount = Math.max(0, members.length - todayEntries.length);

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Team</span>
          <Badge variant="gray"><NumberFlow value={members.length} /></Badge>
        </div>
      </div>

      <div className="v5-ani" style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 10, animationDelay: '0.06s' }}>
        <StatTile label="Heute anwesend" value={presentCount} accent />
        <StatTile label="Heute abwesend" value={todayEntries.length} />
        <StatTile label="Kommende Abwesenheiten" value={upcomingEntries.length} />
      </div>

      <Card className="v5-ani" style={{ overflow: 'hidden', animationDelay: '0.12s' }} data-testid="team-roster">
        <div
          className="v5-mono"
          style={{
            display: 'grid',
            gridTemplateColumns: '40px 1fr 120px 100px 100px',
            padding: '8px 16px',
            fontSize: 9,
            letterSpacing: 1.2,
            textTransform: 'uppercase',
            color: 'var(--v5-mut)',
            borderBottom: '1px solid var(--v5-bor)',
          }}
        >
          <span>&nbsp;</span>
          <span>Name</span>
          <span>Status</span>
          <span>Stellplatz</span>
          <span>Seit</span>
        </div>
        {members.map((m, i) => (
          <div
            key={m.name}
            data-testid="team-row"
            className="v5-row"
            style={{
              display: 'grid',
              gridTemplateColumns: '40px 1fr 120px 100px 100px',
              padding: '10px 16px',
              borderBottom: i < members.length - 1 ? '1px solid var(--v5-bor)' : 'none',
              alignItems: 'center',
              gap: 4,
            }}
          >
            <div
              style={{
                width: 30, height: 30, borderRadius: '50%',
                background: 'var(--v5-acc-muted)', color: 'var(--v5-acc)',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                fontWeight: 700, fontSize: 12,
              }}
              aria-hidden="true"
            >
              {m.name[0]?.toUpperCase() ?? '?'}
            </div>
            <div>
              <div style={{ fontSize: 12, fontWeight: 500, color: 'var(--v5-txt)' }}>{m.name}</div>
              <div style={{ fontSize: 10, color: 'var(--v5-mut)' }}>{m.department}</div>
            </div>
            <Badge variant={statusVariant(m.status)} dot>{m.statusLabel}</Badge>
            <span className="v5-mono" style={{ fontSize: 11, color: 'var(--v5-txt)' }}>{m.slot}</span>
            <span style={{ fontSize: 11, color: 'var(--v5-mut)' }}>{m.since}</span>
          </div>
        ))}
      </Card>

      {todayEntries.length > 0 && (
        <Card className="v5-ani" style={{ padding: 14, animationDelay: '0.16s' }} data-testid="today-absences">
          <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.4, color: 'var(--v5-mut)', textTransform: 'uppercase', marginBottom: 10 }}>
            Heute abwesend ({todayEntries.length})
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            {todayEntries.map((e, i) => (
              <div key={i} style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 12, color: 'var(--v5-txt)' }}>
                <span style={{ width: 6, height: 6, borderRadius: '50%', background: 'var(--v5-err)' }} aria-hidden="true" />
                <span style={{ fontWeight: 500 }}>{e.user_name}</span>
                <span style={{ color: 'var(--v5-mut)', fontSize: 11 }}>{absenceLabel(e.absence_type)}</span>
                <span style={{ marginLeft: 'auto', color: 'var(--v5-mut)', fontSize: 11 }}>
                  {formatDate(e.start_date)}{e.start_date !== e.end_date ? ` – ${formatDate(e.end_date)}` : ''}
                </span>
              </div>
            ))}
          </div>
        </Card>
      )}
    </div>
  );
}

function StatTile({ label, value, accent = false }: { label: string; value: number; accent?: boolean }) {
  return (
    <Card style={{ padding: 14, background: accent ? 'var(--v5-acc-muted)' : 'var(--v5-sur)' }}>
      <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.4, color: accent ? 'var(--v5-acc)' : 'var(--v5-mut)', textTransform: 'uppercase' }}>
        {label}
      </div>
      <div style={{ marginTop: 6, fontSize: 28, fontWeight: 800, color: accent ? 'var(--v5-acc)' : 'var(--v5-txt)', letterSpacing: -1 }}>
        <NumberFlow value={value} />
      </div>
    </Card>
  );
}
