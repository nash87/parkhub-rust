import { Fragment, useState, useMemo, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import type { Booking } from '../api/client';

// ── Types ───────────────────────────────────────────────────────────────────

export interface HeatmapCell {
  /** 0-based day index (0=Mon, 6=Sun) */
  day: number;
  /** Hour of day (6-22) */
  hour: number;
  /** 0–100 occupancy percentage */
  percentage: number;
  /** Raw booking count in this slot */
  count: number;
}

export interface OccupancyHeatmapProps {
  bookings: Booking[];
  totalSlots: number;
}

// ── Data computation ────────────────────────────────────────────────────────

const HOUR_START = 6;
const HOUR_END = 22; // exclusive — rows: 6,7,...,21

/**
 * Compute hourly occupancy per day-of-week from the last 30 days of bookings.
 * Returns a flat array of HeatmapCell for every (day, hour) pair.
 */
export function computeHeatmapData(
  bookings: Booking[],
  totalSlots: number,
): HeatmapCell[] {
  const now = new Date();
  const thirtyDaysAgo = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);

  // Accumulate booking counts: [day][hour] = total count across weeks
  // Also track how many weeks each day appeared so we can average
  const counts: number[][] = Array.from({ length: 7 }, () =>
    new Array(HOUR_END - HOUR_START).fill(0),
  );

  // Count how many of each weekday exist in the 30-day window
  const weekdayCounts = new Array(7).fill(0);
  for (let d = new Date(thirtyDaysAgo); d <= now; d.setDate(d.getDate() + 1)) {
    // JS getDay: 0=Sun → remap to 0=Mon
    const jsDay = d.getDay();
    const day = jsDay === 0 ? 6 : jsDay - 1;
    weekdayCounts[day]++;
  }

  for (const b of bookings) {
    const start = new Date(b.start_time);
    const end = new Date(b.end_time);

    if (end < thirtyDaysAgo || start > now) continue;
    if (b.status === 'cancelled') continue;

    // Walk each hour the booking spans
    const clampedStart = start < thirtyDaysAgo ? thirtyDaysAgo : start;
    const clampedEnd = end > now ? now : end;

    const cursor = new Date(clampedStart);
    cursor.setMinutes(0, 0, 0);

    while (cursor < clampedEnd) {
      const h = cursor.getHours();
      if (h >= HOUR_START && h < HOUR_END) {
        const jsDay = cursor.getDay();
        const day = jsDay === 0 ? 6 : jsDay - 1;
        counts[day][h - HOUR_START]++;
      }
      cursor.setHours(cursor.getHours() + 1);
    }
  }

  const cells: HeatmapCell[] = [];
  const slotsOrOne = Math.max(totalSlots, 1);

  for (let day = 0; day < 7; day++) {
    for (let hi = 0; hi < HOUR_END - HOUR_START; hi++) {
      const rawCount = counts[day][hi];
      const weeks = Math.max(weekdayCounts[day], 1);
      const avgCount = rawCount / weeks;
      const pct = Math.min(Math.round((avgCount / slotsOrOne) * 100), 100);
      cells.push({
        day,
        hour: HOUR_START + hi,
        percentage: pct,
        count: Math.round(avgCount),
      });
    }
  }

  return cells;
}

// ── Color scale (OKLCH) ─────────────────────────────────────────────────────

/**
 * OKLCH-based smooth gradient: green (0%) → yellow (50%) → red (100%).
 * Returns an oklch() CSS string with good contrast in both light and dark modes.
 */
export function heatmapColor(pct: number, isDark: boolean): string {
  const t = Math.max(0, Math.min(1, pct / 100));

  // Hue: 145 (green) → 90 (yellow) → 25 (red)
  let hue: number;
  if (t <= 0.5) {
    hue = 145 - (145 - 90) * (t / 0.5);
  } else {
    hue = 90 - (90 - 25) * ((t - 0.5) / 0.5);
  }

  // Lightness: keep readable in both themes
  const baseLightness = isDark ? 0.55 : 0.72;
  const lightness = baseLightness - t * 0.08;

  // Chroma: moderate saturation, peak at midpoint
  const chroma = 0.12 + 0.08 * Math.sin(t * Math.PI);

  return `oklch(${lightness.toFixed(3)} ${chroma.toFixed(3)} ${hue.toFixed(1)})`;
}

// ── Component ───────────────────────────────────────────────────────────────

export function OccupancyHeatmap({ bookings, totalSlots }: OccupancyHeatmapProps) {
  const { t } = useTranslation();
  const [tooltip, setTooltip] = useState<{ x: number; y: number; cell: HeatmapCell } | null>(null);

  const cells = useMemo(
    () => computeHeatmapData(bookings, totalSlots),
    [bookings, totalSlots],
  );

  const isDark = useMemo(() => {
    if (typeof document === 'undefined') return false;
    return document.documentElement.classList.contains('dark');
  }, []);

  const dayLabels = useMemo(() => [
    t('heatmap.mon'),
    t('heatmap.tue'),
    t('heatmap.wed'),
    t('heatmap.thu'),
    t('heatmap.fri'),
    t('heatmap.sat'),
    t('heatmap.sun'),
  ], [t]);

  const hours = useMemo(
    () => Array.from({ length: HOUR_END - HOUR_START }, (_, i) => HOUR_START + i),
    [],
  );

  const handleMouseEnter = useCallback(
    (e: React.MouseEvent<HTMLDivElement>, cell: HeatmapCell) => {
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
      const parent = (e.currentTarget as HTMLElement).closest('[data-heatmap-root]');
      const parentRect = parent?.getBoundingClientRect() ?? rect;
      setTooltip({
        x: rect.left - parentRect.left + rect.width / 2,
        y: rect.top - parentRect.top - 4,
        cell,
      });
    },
    [],
  );

  const handleMouseLeave = useCallback(() => setTooltip(null), []);

  const maxPct = useMemo(
    () => Math.max(...cells.map(c => c.percentage), 1),
    [cells],
  );

  return (
    <div data-heatmap-root="" className="relative" style={{ position: 'relative' }}>
      {/* Horizontal scroll wrapper for mobile */}
      <div className="overflow-x-auto -mx-2 px-2 pb-2">
        <div
          className="inline-grid gap-1"
          style={{
            gridTemplateColumns: `auto repeat(7, minmax(2.5rem, 1fr))`,
            gridTemplateRows: `auto repeat(${hours.length}, minmax(1.75rem, 1fr))`,
            minWidth: '22rem',
          }}
          role="grid"
          aria-label={t('heatmap.title')}
        >
          {/* Top-left empty corner */}
          <div />

          {/* Day column headers */}
          {dayLabels.map((label, i) => (
            <div
              key={`day-${i}`}
              className="text-xs font-medium text-surface-500 dark:text-surface-400 text-center py-1 select-none"
              role="columnheader"
            >
              {label}
            </div>
          ))}

          {/* Rows: one per hour */}
          {hours.map(hour => (
            <Fragment key={`row-${hour}`}>
              {/* Hour label */}
              <div
                key={`hour-label-${hour}`}
                className="text-xs font-medium text-surface-500 dark:text-surface-400 text-right pr-2 flex items-center justify-end select-none tabular-nums"
                role="rowheader"
              >
                {`${hour}:00`}
              </div>

              {/* 7 cells for this hour */}
              {Array.from({ length: 7 }, (_, day) => {
                const cell = cells.find(c => c.day === day && c.hour === hour);
                const pct = cell?.percentage ?? 0;
                return (
                  <div
                    key={`cell-${day}-${hour}`}
                    role="gridcell"
                    aria-label={`${dayLabels[day]} ${hour}:00 - ${pct}%`}
                    className="rounded-md cursor-pointer transition-transform hover:scale-110 hover:z-10"
                    style={{
                      backgroundColor: heatmapColor(pct, isDark),
                      opacity: pct === 0 ? 0.25 : 0.4 + 0.6 * (pct / maxPct),
                    }}
                    onMouseEnter={cell ? (e) => handleMouseEnter(e, cell) : undefined}
                    onMouseLeave={handleMouseLeave}
                  />
                );
              })}
            </Fragment>
          ))}
        </div>
      </div>

      {/* Tooltip */}
      {tooltip && (
        <div
          data-testid="heatmap-tooltip"
          className="absolute z-50 pointer-events-none px-3 py-2 rounded-lg text-xs font-medium shadow-lg
            bg-white/90 dark:bg-surface-800/90 backdrop-blur-sm
            text-surface-900 dark:text-white border border-surface-200 dark:border-surface-700"
          style={{
            left: tooltip.x,
            top: tooltip.y,
            transform: 'translate(-50%, -100%)',
            whiteSpace: 'nowrap',
          }}
        >
          <span className="font-semibold">{tooltip.cell.percentage}%</span>
          {' '}
          {t('heatmap.occupancy')}
          <br />
          <span className="text-surface-500 dark:text-surface-400">
            {tooltip.cell.count} {t('heatmap.bookings')}
          </span>
        </div>
      )}

      {/* Color legend */}
      <div className="flex items-center gap-2 mt-3 text-xs text-surface-500 dark:text-surface-400">
        <span>{t('heatmap.empty')}</span>
        <div
          className="flex-1 h-2 rounded-full max-w-[12rem]"
          style={{
            background: `linear-gradient(to right, ${heatmapColor(0, isDark)}, ${heatmapColor(50, isDark)}, ${heatmapColor(100, isDark)})`,
          }}
        />
        <span>{t('heatmap.full')}</span>
      </div>
    </div>
  );
}
