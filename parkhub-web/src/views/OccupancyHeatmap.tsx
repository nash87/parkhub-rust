import { Fragment, useState, useEffect, useMemo } from 'react';
import { motion } from 'framer-motion';
import { ChartBar, Clock, CalendarBlank, TrendUp } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import { getInMemoryToken } from '../api/client';

interface HeatmapCell {
  day: number;       // 0=Monday .. 6=Sunday
  hour: number;      // 0..23
  percentage: number; // 0..100
  avg_bookings: number;
}

interface HeatmapData {
  cells: HeatmapCell[];
  lots: { id: string; name: string }[];
}

interface Stats {
  peakHour: string;
  avgOccupancy: string;
  busiestDay: string;
}

const DAYS = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
const DAY_SHORT = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];

function cellColor(pct: number): string {
  if (pct >= 90) return 'bg-red-500';
  if (pct >= 75) return 'bg-amber-400 dark:bg-amber-500';
  if (pct >= 50) return 'bg-primary-300 dark:bg-primary-400';
  if (pct >= 20) return 'bg-primary-100 dark:bg-primary-900';
  return 'bg-surface-100 dark:bg-surface-800';
}

function cellTextColor(pct: number): string {
  if (pct >= 90) return 'text-white';
  if (pct >= 75) return 'text-amber-900';
  if (pct >= 50) return 'text-primary-900 dark:text-primary-100';
  return 'text-surface-500';
}

function authHeaders(): Record<string, string> {
  const token = getInMemoryToken();
  return {
    Accept: 'application/json',
    'X-Requested-With': 'XMLHttpRequest',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
  };
}

export function OccupancyHeatmapPage() {
  const { t } = useTranslation();
  const [data, setData] = useState<HeatmapData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedLot, setSelectedLot] = useState<string>('');
  const [tooltip, setTooltip] = useState<{ day: number; hour: number; pct: number; avg: number; x: number; y: number } | null>(null);

  useEffect(() => {
    setLoading(true);
    setError(null);
    const params = selectedLot ? `?lot_id=${selectedLot}` : '';
    fetch(`/api/v1/admin/analytics/occupancy-heatmap${params}`, {
      headers: authHeaders(),
      credentials: 'include',
    })
      .then(r => r.json())
      .then(json => {
        if (json?.data) {
          setData(json.data);
        } else {
          setError(t('heatmap.loadError'));
        }
      })
      .catch(() => setError(t('heatmap.loadError')))
      .finally(() => setLoading(false));
  }, [selectedLot, t]);

  // Compute stats from cells
  const stats = useMemo<Stats>(() => {
    if (!data?.cells?.length) {
      return { peakHour: '-', avgOccupancy: '0%', busiestDay: '-' };
    }
    const cells = data.cells;

    // Peak hour: hour with highest average across all days
    const hourAvg = new Map<number, number[]>();
    for (const c of cells) {
      if (!hourAvg.has(c.hour)) hourAvg.set(c.hour, []);
      hourAvg.get(c.hour)!.push(c.percentage);
    }
    let peakHour = 0;
    let peakVal = 0;
    for (const [hour, vals] of hourAvg) {
      const avg = vals.reduce((a, b) => a + b, 0) / vals.length;
      if (avg > peakVal) { peakVal = avg; peakHour = hour; }
    }

    // Average occupancy
    const avgOcc = cells.reduce((sum, c) => sum + c.percentage, 0) / cells.length;

    // Busiest day
    const dayAvg = new Map<number, number[]>();
    for (const c of cells) {
      if (!dayAvg.has(c.day)) dayAvg.set(c.day, []);
      dayAvg.get(c.day)!.push(c.percentage);
    }
    let busiestDay = 0;
    let busiestVal = 0;
    for (const [day, vals] of dayAvg) {
      const avg = vals.reduce((a, b) => a + b, 0) / vals.length;
      if (avg > busiestVal) { busiestVal = avg; busiestDay = day; }
    }

    return {
      peakHour: `${String(peakHour).padStart(2, '0')}:00`,
      avgOccupancy: `${Math.round(avgOcc)}%`,
      busiestDay: DAYS[busiestDay] || '-',
    };
  }, [data]);

  // Build cell lookup
  const cellMap = useMemo(() => {
    const map = new Map<string, HeatmapCell>();
    if (data?.cells) {
      for (const c of data.cells) {
        map.set(`${c.day}-${c.hour}`, c);
      }
    }
    return map;
  }, [data]);

  function handleCellHover(day: number, hour: number, e: React.MouseEvent) {
    const cell = cellMap.get(`${day}-${hour}`);
    const pct = cell?.percentage ?? 0;
    const avg = cell?.avg_bookings ?? 0;
    const rect = (e.target as HTMLElement).getBoundingClientRect();
    setTooltip({ day, hour, pct, avg, x: rect.left + rect.width / 2, y: rect.top - 8 });
  }

  return (
    <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} className="space-y-6" data-testid="heatmap-page">
      {/* Header */}
      <div className="flex items-center justify-between flex-wrap gap-4">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white flex items-center gap-2">
            <ChartBar weight="duotone" className="w-7 h-7 text-primary-500" />
            {t('heatmap.title')}
          </h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">{t('heatmap.subtitle')}</p>
        </div>
        {data?.lots && data.lots.length > 1 && (
          <select
            value={selectedLot}
            onChange={e => setSelectedLot(e.target.value)}
            className="rounded-xl bg-surface-50 dark:bg-surface-800 border border-surface-200 dark:border-surface-700 px-3 py-2 text-sm"
            data-testid="lot-selector"
          >
            <option value="">{t('heatmap.allLots')}</option>
            {data.lots.map(lot => (
              <option key={lot.id} value={lot.id}>{lot.name}</option>
            ))}
          </select>
        )}
      </div>

      {/* Stat cards */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4" data-testid="stat-cards">
        <div className="bg-white dark:bg-surface-900 rounded-xl p-5 border border-surface-200 dark:border-surface-800 shadow-sm">
          <div className="flex items-center gap-3 mb-2">
            <div className="w-10 h-10 rounded-lg bg-primary-50 dark:bg-primary-950/30 flex items-center justify-center">
              <Clock weight="fill" className="w-5 h-5 text-primary-600 dark:text-primary-400" />
            </div>
            <span className="text-sm text-surface-500 dark:text-surface-400">{t('heatmap.peakHour')}</span>
          </div>
          <div className="text-2xl font-bold text-surface-900 dark:text-white" data-testid="stat-peak-hour">{stats.peakHour}</div>
        </div>
        <div className="bg-white dark:bg-surface-900 rounded-xl p-5 border border-surface-200 dark:border-surface-800 shadow-sm">
          <div className="flex items-center gap-3 mb-2">
            <div className="w-10 h-10 rounded-lg bg-primary-50 dark:bg-primary-950/30 flex items-center justify-center">
              <TrendUp weight="fill" className="w-5 h-5 text-primary-600 dark:text-primary-400" />
            </div>
            <span className="text-sm text-surface-500 dark:text-surface-400">{t('heatmap.avgOccupancy')}</span>
          </div>
          <div className="text-2xl font-bold text-surface-900 dark:text-white" data-testid="stat-avg-occupancy">{stats.avgOccupancy}</div>
        </div>
        <div className="bg-white dark:bg-surface-900 rounded-xl p-5 border border-surface-200 dark:border-surface-800 shadow-sm">
          <div className="flex items-center gap-3 mb-2">
            <div className="w-10 h-10 rounded-lg bg-primary-50 dark:bg-primary-950/30 flex items-center justify-center">
              <CalendarBlank weight="fill" className="w-5 h-5 text-primary-600 dark:text-primary-400" />
            </div>
            <span className="text-sm text-surface-500 dark:text-surface-400">{t('heatmap.busiestDay')}</span>
          </div>
          <div className="text-2xl font-bold text-surface-900 dark:text-white" data-testid="stat-busiest-day">{stats.busiestDay}</div>
        </div>
      </div>

      {/* Heatmap grid */}
      {loading ? (
        <div className="flex justify-center py-12" data-testid="loading">
          <div className="w-8 h-8 border-2 border-primary-500 border-t-transparent rounded-full animate-spin" />
        </div>
      ) : error ? (
        <div className="text-center py-12 text-red-500" data-testid="error-state">
          <p>{error}</p>
        </div>
      ) : (
        <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-4 sm:p-6 overflow-x-auto">
          <div
            className="grid gap-0.5"
            style={{ gridTemplateColumns: 'auto repeat(24, minmax(24px, 1fr))' }}
            role="grid"
            data-testid="heatmap-grid"
          >
            {/* Hour header row */}
            <div /> {/* empty corner */}
            {Array.from({ length: 24 }, (_, h) => (
              <div key={h} className="text-center text-[10px] text-surface-400 pb-1" role="columnheader">
                {h}
              </div>
            ))}

            {/* Day rows */}
            {DAY_SHORT.map((day, dayIdx) => (
              <Fragment key={`day-row-${dayIdx}`}>
                <div key={`label-${dayIdx}`} className="pr-2 text-xs text-surface-500 text-right flex items-center justify-end" role="rowheader">
                  {day}
                </div>
                {Array.from({ length: 24 }, (_, hour) => {
                  const cell = cellMap.get(`${dayIdx}-${hour}`);
                  const pct = cell?.percentage ?? 0;
                  return (
                    <div
                      key={`${dayIdx}-${hour}`}
                      className={`aspect-square rounded-sm cursor-pointer transition-transform hover:scale-110 ${cellColor(pct)}`}
                      role="gridcell"
                      aria-label={`${DAYS[dayIdx]} ${String(hour).padStart(2, '0')}:00 - ${Math.round(pct)}%`}
                      onMouseEnter={e => handleCellHover(dayIdx, hour, e)}
                      onMouseLeave={() => setTooltip(null)}
                    />
                  );
                })}
              </Fragment>
            ))}
          </div>

          {/* Tooltip */}
          {tooltip && (
            <div
              className="fixed z-50 bg-surface-900 dark:bg-surface-100 text-white dark:text-surface-900 text-xs rounded-lg px-3 py-2 pointer-events-none shadow-lg"
              style={{ left: tooltip.x, top: tooltip.y, transform: 'translate(-50%, -100%)' }}
              data-testid="heatmap-tooltip"
            >
              <span className="font-medium">{DAYS[tooltip.day]} {String(tooltip.hour).padStart(2, '0')}:00</span>
              <span className="mx-1">&mdash;</span>
              <span>{Math.round(tooltip.pct)}%</span>
              <span className="text-surface-400 dark:text-surface-500 ml-1">({t('heatmap.avgBookings', { count: tooltip.avg.toFixed(1) })})</span>
            </div>
          )}

          {/* Legend */}
          <div className="flex items-center justify-center gap-4 mt-4 text-xs text-surface-500" data-testid="heatmap-legend">
            <div className="flex items-center gap-1">
              <div className="w-4 h-4 rounded-sm bg-surface-100 dark:bg-surface-800 border border-surface-200" />
              <span>{t('heatmap.empty')}</span>
            </div>
            <div className="flex items-center gap-1">
              <div className="w-4 h-4 rounded-sm bg-primary-100 dark:bg-primary-900" />
              <span>{t('heatmap.low')}</span>
            </div>
            <div className="flex items-center gap-1">
              <div className="w-4 h-4 rounded-sm bg-primary-300 dark:bg-primary-400" />
              <span>{t('heatmap.medium')}</span>
            </div>
            <div className="flex items-center gap-1">
              <div className="w-4 h-4 rounded-sm bg-amber-400 dark:bg-amber-500" />
              <span>{t('heatmap.high')}</span>
            </div>
            <div className="flex items-center gap-1">
              <div className="w-4 h-4 rounded-sm bg-red-500" />
              <span>{t('heatmap.full')}</span>
            </div>
          </div>
        </div>
      )}
    </motion.div>
  );
}
