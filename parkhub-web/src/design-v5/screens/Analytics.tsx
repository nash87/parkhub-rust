import NumberFlow from '@number-flow/react';
import { lazy, Suspense, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Card, SectionLabel, StatCard, V5NamedIcon } from '../primitives';
import { api } from '../../api/client';
import type { ScreenId } from '../nav';

/* ───────────────────────────────────────────────────────────────────
   uPlot (~40KB gz) is loaded lazily so non-admin routes never pay
   its cost — keeps LCP inside the Lighthouse budget. Admin-only
   Analytics screen is the sole consumer; the chunk materialises on
   mount and is replayed from the HTTP cache on subsequent visits.
   ─────────────────────────────────────────────────────────────────── */
const UPlotChart = lazy(() =>
  import('../primitives/UPlotChart').then((m) => ({ default: m.UPlotChart })),
);

const DAY_LABELS = ['Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa', 'So'];

/** Skeleton placeholder matching UPlotChart's visual height (140) while the
 *  lazy chunk streams in. Reuses the same ph-v5-pulse keyframe the screen's
 *  loading-state skeleton blocks use, so fallback feels native. */
function ChartSkeleton({ ariaLabel }: { ariaLabel: string }) {
  return (
    <div
      role="img"
      aria-label={ariaLabel}
      style={{
        height: 140,
        borderRadius: 10,
        background: 'var(--v5-sur2)',
        animation: 'ph-v5-pulse 1.6s ease infinite',
      }}
    />
  );
}

/** Canvas-rendered chart block backed by uPlot (MIT, ~40KB). */
function ChartBlock({
  label,
  xs,
  ys,
  tickLabels,
  stroke = 'var(--v5-acc)',
  fill = 'var(--v5-acc-muted)',
}: {
  label: string;
  xs: number[];
  ys: number[];
  tickLabels: string[];
  stroke?: string;
  fill?: string;
}) {
  // Memoize data + options so the UPlotChart effect doesn't tear down
  // and rebuild the canvas on every parent re-render (e.g. query status
  // flips, layout toggles). Stable references → stable uPlot instance.
  // Hooks MUST run unconditionally — the empty-data branch just skips render.
  const data = useMemo<[number[], number[]]>(() => [xs, ys], [xs, ys]);
  const options = useMemo<Partial<import('uplot').Options>>(
    () => ({
      series: [
        {},
        {
          stroke,
          fill,
          width: 2,
          paths: (u, sidx, i0, i1) => {
            // Bar-style paths via uPlot's built-in bars plugin-style drawing.
            const { ctx } = u;
            const xVals = u.data[0] as number[];
            const yVals = u.data[sidx] as number[];
            const path = new Path2D();
            const n = xVals.length;
            if (n === 0) return null;
            const span = n > 1 ? xVals[1]! - xVals[0]! : 1;
            const barW = Math.max(2, u.valToPos(span, 'x', true) - u.valToPos(0, 'x', true) - 4);
            const zeroY = u.valToPos(0, 'y', true);
            for (let i = i0; i <= i1; i++) {
              const cx = u.valToPos(xVals[i]!, 'x', true);
              const cy = u.valToPos(yVals[i]!, 'y', true);
              const h = zeroY - cy;
              path.rect(cx - barW / 2, cy, barW, h);
            }
            ctx.save();
            ctx.fillStyle = typeof fill === 'string' ? fill : 'currentColor';
            ctx.fill(path);
            ctx.restore();
            return null;
          },
          points: { show: false },
        },
      ],
      axes: [
        {
          stroke: 'var(--v5-mut)',
          grid: { show: false },
          ticks: { show: false },
          values: (_u, splits) => splits.map((v) => tickLabels[Math.round(v)] ?? ''),
          size: 22,
        },
        {
          stroke: 'var(--v5-mut)',
          grid: { stroke: 'var(--v5-bord)', width: 1 },
          ticks: { show: false },
          size: 32,
        },
      ],
      scales: {
        x: { time: false, range: [xs[0]! - 0.5, xs[xs.length - 1]! + 0.5] },
        y: { range: (_u, _dMin, dMax) => [0, Math.max(dMax, 1) * 1.1] },
      },
    }),
    [stroke, fill, tickLabels, xs],
  );

  if (xs.length === 0) {
    return (
      <div>
        <SectionLabel>{label}</SectionLabel>
        <div
          role="img"
          aria-label={label}
          style={{ fontSize: 11, color: 'var(--v5-mut)', padding: '14px 0' }}
        >
          Keine Daten
        </div>
      </div>
    );
  }

  return (
    <div>
      <SectionLabel>{label}</SectionLabel>
      <Suspense fallback={<ChartSkeleton ariaLabel={label} />}>
        <UPlotChart data={data} options={options} ariaLabel={label} height={140} />
      </Suspense>
    </div>
  );
}

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

  const hourData = useMemo(() => {
    const byHour = stats?.occupancy_by_hour ?? {};
    const xs: number[] = [];
    const ys: number[] = [];
    const ticks: string[] = [];
    for (let h = 0; h < 24; h++) {
      const key = String(h).padStart(2, '0');
      xs.push(h);
      ys.push(Number(byHour[key] ?? byHour[String(h)] ?? 0));
      ticks.push(key);
    }
    return { xs, ys, ticks };
  }, [stats]);

  const dayData = useMemo(() => {
    const byDay = stats?.occupancy_by_day ?? {};
    const xs: number[] = [];
    const ys: number[] = [];
    DAY_LABELS.forEach((lbl, i) => {
      const entry = byDay[lbl] ?? byDay[String(i)] ?? null;
      xs.push(i);
      ys.push(entry ? entry.avg_percentage : 0);
    });
    return { xs, ys, ticks: DAY_LABELS };
  }, [stats]);

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
        <ChartBlock
          label="Auslastung nach Stunde (%)"
          xs={hourData.xs}
          ys={hourData.ys}
          tickLabels={hourData.ticks}
        />
      </Card>

      <Card className="v5-ani" style={{ padding: 16, animationDelay: '0.18s' }}>
        <ChartBlock
          label="Auslastung nach Wochentag (%)"
          xs={dayData.xs}
          ys={dayData.ys}
          tickLabels={dayData.ticks}
          stroke="var(--v5-ok, oklch(0.65 0.17 160))"
          fill="var(--v5-ok-muted, oklch(0.65 0.17 160 / 0.25))"
        />
      </Card>
    </div>
  );
}
