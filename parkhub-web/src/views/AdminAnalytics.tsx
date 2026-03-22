import { useState, useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { ChartBar, TrendUp, Users, Clock, CurrencyDollar, Export, CalendarBlank } from '@phosphor-icons/react';
import { getInMemoryToken } from '../api/client';

interface DailyDataPoint { date: string; value: number; }
interface HourBin { hour: number; count: number; }
interface TopLot { lot_id: string; lot_name: string; total_slots: number; bookings: number; utilization_percent: number; }
interface MonthlyGrowth { month: string; count: number; }
interface AnalyticsData {
  daily_bookings: DailyDataPoint[];
  daily_revenue: DailyDataPoint[];
  peak_hours: HourBin[];
  top_lots: TopLot[];
  user_growth: MonthlyGrowth[];
  avg_booking_duration_minutes: number;
  total_bookings: number;
  total_revenue: number;
  active_users: number;
}

type DateRange = '7' | '30' | '90' | '365';

function StatCard({ icon: Icon, label, value, sub }: { icon: any; label: string; value: string; sub?: string }) {
  return (
    <div className="bg-white dark:bg-surface-900 rounded-xl p-5 border border-surface-200 dark:border-surface-800 shadow-sm">
      <div className="flex items-center gap-3 mb-2">
        <div className="w-10 h-10 rounded-lg bg-primary-50 dark:bg-primary-950/30 flex items-center justify-center">
          <Icon weight="fill" className="w-5 h-5 text-primary-600 dark:text-primary-400" />
        </div>
        <span className="text-sm text-surface-500 dark:text-surface-400">{label}</span>
      </div>
      <div className="text-2xl font-bold text-surface-900 dark:text-white">{value}</div>
      {sub && <div className="text-xs text-surface-400 mt-1">{sub}</div>}
    </div>
  );
}

function MiniBarChart({ data, height = 120, color = 'var(--color-primary-500, #6366f1)' }: { data: { label: string; value: number }[]; height?: number; color?: string }) {
  if (!data.length) return null;
  const max = Math.max(...data.map(d => d.value), 1);
  const barW = Math.max(2, Math.min(12, (300 / data.length) - 2));
  return (
    <svg viewBox={`0 0 ${data.length * (barW + 2)} ${height}`} className="w-full" style={{ height }} preserveAspectRatio="none">
      {data.map((d, i) => (
        <rect
          key={i}
          x={i * (barW + 2)}
          y={height - (d.value / max) * (height - 4)}
          width={barW}
          height={(d.value / max) * (height - 4)}
          rx={1}
          fill={color}
          opacity={0.85}
        >
          <title>{d.label}: {d.value}</title>
        </rect>
      ))}
    </svg>
  );
}

function HeatmapChart({ peak_hours }: { peak_hours: HourBin[] }) {
  const max = Math.max(...peak_hours.map(h => h.count), 1);
  const days = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
  // Spread 24 hours across a 7-day grid (synthetic — we only have hourly totals)
  return (
    <div className="grid grid-cols-[auto_repeat(24,1fr)] gap-0.5 text-xs">
      <div />
      {Array.from({ length: 24 }, (_, h) => (
        <div key={h} className="text-center text-surface-400 text-[10px]">{h}</div>
      ))}
      {days.map(day => (
        <>
          <div key={day} className="pr-2 text-surface-500 text-right">{day}</div>
          {peak_hours.map((h, i) => {
            const intensity = h.count / max;
            return (
              <div
                key={`${day}-${i}`}
                className="aspect-square rounded-sm"
                style={{
                  backgroundColor: `rgba(99, 102, 241, ${Math.max(0.05, intensity * 0.9)})`,
                }}
                title={`${day} ${h.hour}:00 - ${h.count} bookings`}
              />
            );
          })}
        </>
      ))}
    </div>
  );
}

export function AdminAnalyticsPage() {
  const { t } = useTranslation();
  const [data, setData] = useState<AnalyticsData | null>(null);
  const [loading, setLoading] = useState(true);
  const [range, setRange] = useState<DateRange>('30');

  useEffect(() => {
    setLoading(true);
    const base = (import.meta as any).env?.VITE_API_URL || '';
    const token = getInMemoryToken();
    const headers: Record<string, string> = {
      Accept: 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    };
    fetch(`${base}/api/v1/admin/analytics/overview?days=${range}`, { headers, credentials: 'include' })
      .then(r => r.json())
      .then(json => { if (json?.data) setData(json.data); })
      .catch(() => {})
      .finally(() => setLoading(false));
  }, [range]);

  const bookingsChartData = useMemo(() =>
    (data?.daily_bookings ?? []).map(d => ({ label: d.date, value: d.value })),
  [data]);

  const revenueChartData = useMemo(() =>
    (data?.daily_revenue ?? []).map(d => ({ label: d.date, value: d.value })),
  [data]);

  const exportCsv = () => {
    if (!data) return;
    const rows = [
      ['Date', 'Bookings', 'Revenue'],
      ...data.daily_bookings.map((d, i) => [
        d.date,
        String(d.value),
        String(data.daily_revenue[i]?.value ?? 0),
      ]),
    ];
    const csv = rows.map(r => r.join(',')).join('\n');
    const blob = new Blob([csv], { type: 'text/csv' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `analytics-${range}d.csv`;
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-6" data-testid="admin-analytics">
      <div className="flex items-center justify-between flex-wrap gap-4">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white flex items-center gap-2">
            <ChartBar weight="fill" className="w-6 h-6 text-primary-500" />
            Analytics
          </h1>
          <p className="text-sm text-surface-500 dark:text-surface-400">Comprehensive parking analytics and trends</p>
        </div>
        <div className="flex items-center gap-2">
          {(['7', '30', '90', '365'] as DateRange[]).map(r => (
            <button
              key={r}
              onClick={() => setRange(r)}
              className={`px-3 py-1.5 text-sm rounded-lg transition-colors ${
                range === r
                  ? 'bg-primary-600 text-white'
                  : 'bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400 hover:bg-surface-200 dark:hover:bg-surface-700'
              }`}
            >
              {r === '365' ? '1y' : `${r}d`}
            </button>
          ))}
          <button
            onClick={exportCsv}
            className="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400 hover:bg-surface-200 dark:hover:bg-surface-700 rounded-lg transition-colors"
          >
            <Export weight="bold" className="w-4 h-4" />
            CSV
          </button>
        </div>
      </div>

      {loading ? (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="h-28 bg-surface-100 dark:bg-surface-800 rounded-xl animate-pulse" />
          ))}
        </div>
      ) : data ? (
        <>
          {/* Stats cards */}
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            <StatCard icon={CalendarBlank} label="Total Bookings" value={String(data.total_bookings)} sub={`Last ${range} days`} />
            <StatCard icon={CurrencyDollar} label="Total Revenue" value={`${data.total_revenue.toFixed(2)}`} sub={`Last ${range} days`} />
            <StatCard icon={Clock} label="Avg Duration" value={`${Math.round(data.avg_booking_duration_minutes)} min`} />
            <StatCard icon={Users} label="Active Users" value={String(data.active_users)} sub="With bookings in period" />
          </div>

          {/* Charts row */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <div className="bg-white dark:bg-surface-900 rounded-xl p-5 border border-surface-200 dark:border-surface-800">
              <h3 className="text-sm font-medium text-surface-600 dark:text-surface-400 mb-4 flex items-center gap-2">
                <TrendUp weight="bold" className="w-4 h-4" />
                Daily Bookings
              </h3>
              <MiniBarChart data={bookingsChartData} height={160} color="var(--color-primary-500, #6366f1)" />
            </div>
            <div className="bg-white dark:bg-surface-900 rounded-xl p-5 border border-surface-200 dark:border-surface-800">
              <h3 className="text-sm font-medium text-surface-600 dark:text-surface-400 mb-4 flex items-center gap-2">
                <CurrencyDollar weight="bold" className="w-4 h-4" />
                Daily Revenue
              </h3>
              <MiniBarChart data={revenueChartData} height={160} color="var(--color-emerald-500, #10b981)" />
            </div>
          </div>

          {/* Heatmap + Top Lots */}
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <div className="bg-white dark:bg-surface-900 rounded-xl p-5 border border-surface-200 dark:border-surface-800">
              <h3 className="text-sm font-medium text-surface-600 dark:text-surface-400 mb-4">Bookings by Hour (Heatmap)</h3>
              <HeatmapChart peak_hours={data.peak_hours} />
            </div>
            <div className="bg-white dark:bg-surface-900 rounded-xl p-5 border border-surface-200 dark:border-surface-800">
              <h3 className="text-sm font-medium text-surface-600 dark:text-surface-400 mb-4">Top Lots by Utilization</h3>
              <div className="space-y-3">
                {data.top_lots.length === 0 && <p className="text-sm text-surface-400">No lot data</p>}
                {data.top_lots.map(lot => (
                  <div key={lot.lot_id} className="flex items-center gap-3">
                    <div className="flex-1 min-w-0">
                      <div className="text-sm font-medium text-surface-800 dark:text-surface-200 truncate">{lot.lot_name}</div>
                      <div className="text-xs text-surface-400">{lot.bookings} bookings / {lot.total_slots} slots</div>
                    </div>
                    <div className="w-24">
                      <div className="h-2 bg-surface-100 dark:bg-surface-800 rounded-full overflow-hidden">
                        <div
                          className="h-full rounded-full bg-primary-500"
                          style={{ width: `${Math.min(100, lot.utilization_percent)}%` }}
                        />
                      </div>
                    </div>
                    <span className="text-xs font-medium text-surface-600 dark:text-surface-300 w-12 text-right">
                      {lot.utilization_percent.toFixed(1)}%
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>

          {/* User growth */}
          <div className="bg-white dark:bg-surface-900 rounded-xl p-5 border border-surface-200 dark:border-surface-800">
            <h3 className="text-sm font-medium text-surface-600 dark:text-surface-400 mb-4 flex items-center gap-2">
              <Users weight="bold" className="w-4 h-4" />
              User Growth (12 months)
            </h3>
            <MiniBarChart
              data={data.user_growth.map(g => ({ label: g.month, value: g.count }))}
              height={120}
              color="var(--color-violet-500, #8b5cf6)"
            />
          </div>
        </>
      ) : (
        <div className="text-center py-12 text-surface-500">Failed to load analytics data</div>
      )}
    </div>
  );
}
