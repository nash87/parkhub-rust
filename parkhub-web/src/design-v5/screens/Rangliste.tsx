import { useMemo } from 'react';
import NumberFlow from '@number-flow/react';
import { useQuery } from '@tanstack/react-query';
import { Badge, Card, V5NamedIcon } from '../primitives';
import { api, type TeamMember, type UserBookingStats } from '../../api/client';
import type { ScreenId } from '../nav';

interface BadgeDef {
  key: string;
  label: string;
  variant: 'primary' | 'success' | 'warning' | 'info' | 'purple';
}

interface LeaderboardEntry {
  id: string;
  name: string;
  username: string;
  ecoScore: number;
  bookingsThisMonth: number;
  evPercentage: number;
  badges: BadgeDef[];
  noShows: number;
}

function computeEcoScore(stats: UserBookingStats): number {
  const bookingScore = Math.min(stats.this_month * 5, 40);
  const evScore = stats.total > 0 ? (stats.ev_count / stats.total) * 30 : 0;
  const durationScore = Math.min(stats.avg_duration_hours * 3, 20);
  const reliabilityScore = stats.no_shows === 0 ? 10 : Math.max(0, 10 - stats.no_shows * 3);
  return Math.round(bookingScore + evScore + durationScore + reliabilityScore);
}

function computeBadges(stats: UserBookingStats): BadgeDef[] {
  const badges: BadgeDef[] = [];
  if (stats.ev_count > 0) badges.push({ key: 'ev', label: 'EV', variant: 'success' });
  if (stats.morning_count > 0) badges.push({ key: 'early', label: 'Früh', variant: 'warning' });
  if (stats.swaps_accepted > 0) badges.push({ key: 'team', label: 'Teamplayer', variant: 'info' });
  if (stats.this_month >= 10) badges.push({ key: 'frequent', label: 'Vielparker', variant: 'purple' });
  return badges;
}

const EMPTY_STATS: UserBookingStats = {
  total: 0, this_month: 0, ev_count: 0, morning_count: 0,
  swaps_accepted: 0, no_shows: 0, avg_duration_hours: 0,
};

const MEDAL_COLOR = ['oklch(0.82 0.16 85)', 'oklch(0.72 0 0)', 'oklch(0.55 0.1 55)'];

export function RanglisteV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const teamQuery = useQuery({
    queryKey: ['team-members'],
    queryFn: async () => {
      const res = await api.getTeam();
      if (!res.success) throw new Error(res.error?.message ?? 'Rangliste konnte nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  });

  const statsQuery = useQuery({
    queryKey: ['admin-stats-extended'],
    queryFn: async () => {
      const res = await api.getAdminStatsExtended();
      if (!res.success) throw new Error(res.error?.message ?? 'Statistiken konnten nicht geladen werden');
      return res.data;
    },
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  });

  const isLoading = teamQuery.isLoading || statsQuery.isLoading;
  const isError = teamQuery.isError || statsQuery.isError;
  const members: TeamMember[] = teamQuery.data ?? [];
  const stats = statsQuery.data;

  const leaderboard = useMemo<LeaderboardEntry[]>(() => {
    if (!members.length) return [];
    const byUser = stats?.bookings_by_user ?? {};
    return members
      .map((m) => {
        const s: UserBookingStats = byUser[m.id] ?? EMPTY_STATS;
        return {
          id: m.id,
          name: m.name || m.username,
          username: m.username,
          ecoScore: computeEcoScore(s),
          bookingsThisMonth: s.this_month,
          evPercentage: s.total > 0 ? Math.round((s.ev_count / s.total) * 100) : 0,
          badges: computeBadges(s),
          noShows: s.no_shows,
        };
      })
      .sort((a, b) => b.ecoScore - a.ecoScore);
  }, [members, stats]);

  const maxBookings = useMemo(
    () => Math.max(1, ...leaderboard.map((e) => e.bookingsThisMonth)),
    [leaderboard],
  );

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto' }}>
        <div style={{ height: 32, width: 220, borderRadius: 8, background: 'var(--v5-sur2)', marginBottom: 14, animation: 'ph-v5-pulse 1.6s ease infinite' }} />
        {[0, 1, 2, 3].map((i) => (
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
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>Rangliste konnte nicht geladen werden.</div>
        </Card>
      </div>
    );
  }

  if (leaderboard.length === 0) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 36, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10, maxWidth: 380 }}>
          <div style={{ width: 48, height: 48, borderRadius: 14, background: 'var(--v5-acc-muted)', border: '1.5px dashed color-mix(in oklch, var(--v5-acc) 50%, transparent)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <V5NamedIcon name="rank" size={20} color="var(--v5-acc)" />
          </div>
          <div style={{ textAlign: 'center' }}>
            <div style={{ fontWeight: 600, color: 'var(--v5-txt)', fontSize: 13 }}>Keine Rangliste verfügbar</div>
            <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 3 }}>Sobald Teammitglieder Buchungen haben, erscheint die Rangliste hier.</div>
          </div>
        </Card>
      </div>
    );
  }

  const highlights = [
    {
      label: 'Aktivster',
      value: [...leaderboard].sort((a, b) => b.bookingsThisMonth - a.bookingsThisMonth)[0],
      suffix: (e: LeaderboardEntry) => `${e.bookingsThisMonth} Buchungen`,
    },
    {
      label: 'Grünster',
      value: [...leaderboard].sort((a, b) => b.evPercentage - a.evPercentage)[0],
      suffix: (e: LeaderboardEntry) => `${e.evPercentage}% EV`,
    },
    {
      label: 'Zuverlässigster',
      value: [...leaderboard].sort((a, b) => a.noShows - b.noShows || b.ecoScore - a.ecoScore)[0],
      suffix: (e: LeaderboardEntry) => `${e.noShows} Ausfälle`,
    },
  ];

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Rangliste</span>
          <Badge variant="gray"><NumberFlow value={leaderboard.length} /></Badge>
        </div>
      </div>

      <div className="v5-ani" style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 10, animationDelay: '0.06s' }} data-testid="highlights">
        {highlights.map((h) => (
          <Card key={h.label} style={{ padding: 14 }}>
            <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.4, color: 'var(--v5-mut)', textTransform: 'uppercase' }}>
              {h.label}
            </div>
            <div style={{ marginTop: 6, fontSize: 14, fontWeight: 700, color: 'var(--v5-txt)' }}>{h.value.name}</div>
            <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 2 }}>{h.suffix(h.value)}</div>
          </Card>
        ))}
      </div>

      <Card className="v5-ani" style={{ overflow: 'hidden', animationDelay: '0.12s' }} data-testid="leaderboard">
        <div
          className="v5-mono"
          style={{
            display: 'grid',
            gridTemplateColumns: '36px 1fr 110px 60px 120px',
            padding: '8px 16px',
            fontSize: 9,
            letterSpacing: 1.2,
            textTransform: 'uppercase',
            color: 'var(--v5-mut)',
            borderBottom: '1px solid var(--v5-bor)',
          }}
        >
          <span>Rang</span>
          <span>Name</span>
          <span>Abzeichen</span>
          <span>Score</span>
          <span>Buchungen</span>
        </div>
        {leaderboard.map((entry, idx) => (
          <div
            key={entry.id}
            data-testid="rank-row"
            className="v5-row"
            style={{
              display: 'grid',
              gridTemplateColumns: '36px 1fr 110px 60px 120px',
              padding: '10px 16px',
              borderBottom: idx < leaderboard.length - 1 ? '1px solid var(--v5-bor)' : 'none',
              alignItems: 'center',
              gap: 6,
            }}
          >
            {idx < 3 ? (
              <V5NamedIcon name="rank" size={18} color={MEDAL_COLOR[idx]} />
            ) : (
              <span className="v5-mono" style={{ fontSize: 12, color: 'var(--v5-mut)', fontWeight: 600 }}>{idx + 1}</span>
            )}
            <div style={{ minWidth: 0 }}>
              <div style={{ fontSize: 12, fontWeight: 500, color: 'var(--v5-txt)', whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis' }}>{entry.name}</div>
              <div style={{ fontSize: 10, color: 'var(--v5-mut)' }}>@{entry.username}</div>
            </div>
            <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4 }}>
              {entry.badges.map((b) => (
                <Badge key={b.key} variant={b.variant}>{b.label}</Badge>
              ))}
            </div>
            <span className="v5-mono" style={{ fontSize: 14, fontWeight: 700, color: 'var(--v5-acc)', fontVariantNumeric: 'tabular-nums' }}>
              {entry.ecoScore}
            </span>
            <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
              <div style={{ flex: 1, height: 6, background: 'var(--v5-sur2)', borderRadius: 999, overflow: 'hidden' }}>
                <div
                  style={{
                    height: '100%',
                    width: `${Math.min(100, (entry.bookingsThisMonth / maxBookings) * 100)}%`,
                    background: 'var(--v5-acc)',
                    borderRadius: 999,
                  }}
                />
              </div>
              <span className="v5-mono" style={{ fontSize: 10, color: 'var(--v5-mut)', minWidth: 18, textAlign: 'right', fontVariantNumeric: 'tabular-nums' }}>
                {entry.bookingsThisMonth}
              </span>
            </div>
          </div>
        ))}
      </Card>
    </div>
  );
}
