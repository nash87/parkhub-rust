import NumberFlow from '@number-flow/react';
import { useQuery } from '@tanstack/react-query';
import { Card, SectionLabel, StatCard, V5NamedIcon } from '../primitives';
import { api } from '../../api/client';
import type { ScreenId } from '../nav';

function BarChart({ data, label, color = 'var(--v5-acc)' }: {
  data: { label: string; value: number }[];
  label: string;
  color?: string;
}) {
  if (data.length === 0) {
    return (
      <div style={{ fontSize: 11, color: 'var(--v5-mut)', padding: '14px 0' }}>Keine Daten</div>
    );
  }
  const max = Math.max(...data.map((d) => d.value), 1);
  const width = 520;
  const height = 120;
  const pad = 6;
  const barW = (width - pad * 2) / data.length;
  return (
    <div>
      <SectionLabel>{label}</SectionLabel>
      <svg
        viewBox={`0 0 ${width} ${height + 24}`}
        style={{ width: '100%', height: 'auto', display: 'block' }}
        role="img"
        aria-label={label}
        data-testid="analytics-chart"
      >
        {data.map((d, i) => {
          const h = (d.value / max) * height;
          const x = pad + i * barW;
          const y = height - h;
          return (
            <g key={`${d.label}-${i}`}>
              <rect
                x={x + 2}
                y={y}
                width={barW - 4}
                height={h}
                fill={color}
                rx={3}
                opacity={0.85}
              />
              <text
                x={x + barW / 2}
                y={height + 14}
                fontSize="9"
                fill="var(--v5-mut)"
                textAnchor="middle"
                fontFamily="ui-monospace, SFMono-Regular, Menlo, monospace"
              >
                {d.label}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}

const DAY_LABELS = ['Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa', 'So'];

export function AnalyticsV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const { data: stats, isLoading, isError } = useQuery({
    queryKey: ['analytics-stats'],
    queryFn: async () => {
      const res = await api.getAdminStatsExtended();
      if (!res.success) throw new Error(res.error?.message ?? 'Statistiken konnten nicht geladen werden');
      return res.data;
    },
    staleTime: 60_000,
  });

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
        {[120, 200, 200].map((h, i) => (
          <div key={i} style={{ height: h, borderRadius: 14, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.1}s` }} />
        ))}
      </div>
    );
  }

  if (isError || !stats) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 28, textAlign: 'center', maxWidth: 360 }}>
          <V5NamedIcon name="x" size={26} color="var(--v5-err)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Fehler beim Laden</div>
        </Card>
      </div>
    );
  }

  const byHour = stats.occupancy_by_hour ?? {};
  const hourBars: { label: string; value: number }[] = [];
  for (let h = 0; h < 24; h++) {
    const key = String(h).padStart(2, '0');
    hourBars.push({ label: key, value: Number(byHour[key] ?? byHour[String(h)] ?? 0) });
  }

  const byDay = stats.occupancy_by_day ?? {};
  const dayBars = DAY_LABELS.map((lbl, i) => {
    const entry = byDay[lbl] ?? byDay[String(i)] ?? null;
    return { label: lbl, value: entry ? entry.avg_percentage : 0 };
  });

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Analytics</div>

      <div className="v5-ani" style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(160px, 1fr))', gap: 10, animationDelay: '0.06s' }}>
        <StatCard label="Nutzer" value={<NumberFlow value={stats.total_users} />} icon="users" />
        <StatCard label="Standorte" value={<NumberFlow value={stats.total_lots} />} icon="map" />
        <StatCard label="Buchungen" value={<NumberFlow value={stats.total_bookings} />} icon="list" />
        <StatCard label="Aktive Buchungen" value={<NumberFlow value={stats.active_bookings} />} icon="check" accent />
      </div>

      <Card className="v5-ani" style={{ padding: 16, animationDelay: '0.12s' }}>
        <BarChart data={hourBars} label="Auslastung nach Stunde (%)" />
      </Card>

      <Card className="v5-ani" style={{ padding: 16, animationDelay: '0.18s' }}>
        <BarChart data={dayBars} label="Auslastung nach Wochentag (%)" color="var(--v5-ok, oklch(0.65 0.17 160))" />
      </Card>

      <div className="v5-ani" style={{ fontSize: 10, color: 'var(--v5-mut)' }}>
        Interaktive Charts (uPlot) folgen in separater PR.
      </div>
    </div>
  );
}
