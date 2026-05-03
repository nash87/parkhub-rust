import { useState, useEffect, useMemo } from 'react';
import { motion } from 'framer-motion';
import { SpinnerGap, UsersIcon, Buildings, CalendarCheckIcon, Lightning, type Icon } from '@phosphor-icons/react';
import { api, type AdminBooking, type AdminStats, type ParkingLot } from '../api/client';
import { useTranslation } from 'react-i18next';
import { BarChart, DonutChart, type DonutSlice } from '../components/SimpleChart';
import { OccupancyHeatmap } from '../components/OccupancyHeatmap';
import { ExportButton } from '../components/ExportButton';

const HEATMAP_STATUSES = new Set(['confirmed', 'active', 'completed']);

function StatCard({ icon: Icon, label, value, color = 'primary' }: {
  icon: Icon;
  label: string;
  value: number;
  color?: 'primary' | 'accent' | 'info' | 'success' | 'warn' | 'danger';
}) {
  // v11 SOTA stat meter: colored left-edge bar, mono UPPERCASE eyebrow,
  // hero-size value with color-mix(oklab) gradient wash. Pairs with the
  // admin-hero card landed in PR #489. Color tone drives the semantic
  // wash via the .v11-meter--{color} modifier.
  return (
    <div className={`v11-meter v11-meter--${color}`}>
      <div className="v11-meter-eyebrow">
        <Icon weight="bold" className="w-3.5 h-3.5" />
        {label}
      </div>
      <div className="v11-meter-value">{value}</div>
      <div className="v11-meter-bar" aria-hidden="true">
        <i style={{ width: `${Math.min(value > 0 ? Math.max(value / 200, 0.05) * 100 : 0, 100)}%` }}></i>
      </div>
    </div>
  );
}

/** Build real per-lot occupancy from ParkingLot data. */
function lotOccupancyFromData(lots: ParkingLot[]): DonutSlice[] {
  if (lots.length === 0) return [];
  return lots.map(lot => {
    const occupied = lot.total_slots - lot.available_slots;
    const pct = lot.total_slots > 0
      ? Math.min(Math.round((occupied / lot.total_slots) * 100), 100)
      : 0;
    return {
      label: lot.name,
      capacity: Math.max(lot.total_slots, 1),
      occupancy: pct,
    };
  });
}

/** Build real booking totals for each weekday from booking start times. */
function weeklyBookingData(bookings: AdminBooking[], t: (key: string) => string): { label: string; value: number }[] {
  const dayKeys = ['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'];
  const counts = new Map<number, number>(dayKeys.map((_, index) => [index, 0]));
  for (const booking of bookings) {
    if (!HEATMAP_STATUSES.has(booking.status)) continue;
    const date = new Date(booking.start_time);
    if (Number.isNaN(date.valueOf())) continue;
    const mondayFirstIndex = date.getDay() === 0 ? 6 : date.getDay() - 1;
    counts.set(mondayFirstIndex, (counts.get(mondayFirstIndex) ?? 0) + 1);
  }
  return dayKeys.map((key, i) => ({
    label: t(`reports.weekdays.${key}`),
    value: counts.get(i) ?? 0,
  }));
}

export function AdminReportsPage() {
  const { t } = useTranslation();
  const [stats, setStats] = useState<AdminStats | null>(null);
  const [bookings, setBookings] = useState<AdminBooking[]>([]);
  const [lots, setLots] = useState<ParkingLot[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      api.adminStats(),
      api.adminBookings(500),
      api.getLots(),
    ]).then(([statsRes, bookingsRes, lotsRes]) => {
      if (statsRes.success && statsRes.data) setStats(statsRes.data);
      if (bookingsRes.success && bookingsRes.data) setBookings(bookingsRes.data.items ?? []);
      if (lotsRes.success && lotsRes.data) setLots(lotsRes.data);
    }).finally(() => setLoading(false));
  }, []);

  const weeklyData = useMemo(
    () => weeklyBookingData(bookings, t),
    [bookings, t],
  );

  const heatmapBookings = useMemo(
    () => bookings.filter(booking => HEATMAP_STATUSES.has(booking.status)),
    [bookings],
  );

  const lotOccupancy = useMemo(
    () => lotOccupancyFromData(lots),
    [lots],
  );

  const totalSlots = useMemo(
    () => lots.reduce((sum, l) => sum + l.total_slots, 0) || stats?.total_slots || 0,
    [lots, stats?.total_slots],
  );

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64" role="status" aria-label={t('common.loading')}>
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" aria-hidden="true" />
      </div>
    );
  }

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-8">
      {/* v11 SOTA hero — emerald tone (data + insight, paired with v11-meter stats below). */}
      <section className="admin-hero admin-hero--emerald">
        <div className="admin-hero-left">
          <div className="admin-hero-eyebrow">
            <span className="admin-hero-dot" aria-hidden="true"></span>
            <CalendarCheckIcon weight="bold" className="w-3.5 h-3.5" />
            {t('admin.reportsEyebrow', 'BUSINESS METRICS')}
          </div>
          <h1 className="admin-hero-headline">{t('admin.reports')}</h1>
          <p className="admin-hero-sub">{t('admin.reportsSubtitle', 'Booking volume, occupancy heatmaps, and revenue trends')}</p>
        </div>
        <div className="admin-hero-actions">
          <ExportButton />
        </div>
      </section>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          icon={UsersIcon}
          label={t('admin.totalUsers')}
          value={stats?.total_users ?? 0}
          color="primary"
        />
        <StatCard
          icon={Buildings}
          label={t('admin.totalLots')}
          value={stats?.total_lots ?? 0}
          color="accent"
        />
        <StatCard
          icon={CalendarCheckIcon}
          label={t('admin.totalBookings')}
          value={stats?.total_bookings ?? 0}
          color="info"
        />
        <StatCard
          icon={Lightning}
          label={t('admin.activeBookings')}
          value={stats?.active_bookings ?? 0}
          color="success"
        />
      </div>

      {/* Summary Card */}
      <div className="card p-6">
        <h3 className="text-sm font-semibold text-surface-900 dark:text-white uppercase tracking-wide mb-4">{t('admin.overview')}</h3>
        <div className="space-y-4">
          <div className="flex items-center justify-between py-3 border-b border-surface-100 dark:border-surface-800">
            <span className="text-sm text-surface-600 dark:text-surface-400">{t('admin.utilizationRate')}</span>
            <span className="text-sm font-semibold text-surface-900 dark:text-white">
              {stats && Number.isFinite(stats.occupancy_percent)
                ? `${Math.round(stats.occupancy_percent)}%`
                : '0%'}
            </span>
          </div>
          <div className="flex items-center justify-between py-3 border-b border-surface-100 dark:border-surface-800">
            <span className="text-sm text-surface-600 dark:text-surface-400">{t('admin.avgBookingsPerUser')}</span>
            <span className="text-sm font-semibold text-surface-900 dark:text-white">
              {stats && stats.total_users > 0
                ? (stats.total_bookings / stats.total_users).toFixed(1)
                : '0'}
            </span>
          </div>
          <div className="flex items-center justify-between py-3">
            <span className="text-sm text-surface-600 dark:text-surface-400">{t('admin.activeBookingRate')}</span>
            <span className="text-sm font-semibold text-surface-900 dark:text-white">
              {stats && stats.total_bookings > 0
                ? `${Math.round((stats.active_bookings / stats.total_bookings) * 100)}%`
                : '0%'}
            </span>
          </div>
        </div>
      </div>

      {/* Bookings This Week Chart */}
      <div className="card p-6">
        <h3 className="text-sm font-semibold text-surface-900 dark:text-white uppercase tracking-wide mb-4">
          {t('admin.bookingsThisWeek')}
        </h3>
        <BarChart data={weeklyData} />
      </div>

      {/* Lot Occupancy Donut Chart */}
      {lotOccupancy.length > 0 && (
        <div className="card p-6">
          <h3 className="text-sm font-semibold text-surface-900 dark:text-white uppercase tracking-wide mb-6">
            {t('admin.lotOccupancy')}
          </h3>
          <div className="flex flex-col sm:flex-row items-center gap-8">
            <div className="flex-shrink-0">
              <DonutChart slices={lotOccupancy} size={180} strokeWidth={24} />
            </div>
            <ul className="flex-1 space-y-2 w-full">
              {lotOccupancy.map(lot => (
                <li key={lot.label} className="flex items-center justify-between text-sm">
                  <span className="flex items-center gap-2">
                    <span
                      className="inline-block w-2.5 h-2.5 rounded-full"
                      style={{ background: lot.occupancy >= 80 ? 'var(--color-red-500,#ef4444)' : lot.occupancy >= 60 ? 'var(--color-amber-400,#fbbf24)' : 'var(--color-emerald-500,#10b981)' }}
                    />
                    <span className="text-surface-700 dark:text-surface-300 font-medium">{lot.label}</span>
                  </span>
                  <span className="font-semibold text-surface-900 dark:text-white tabular-nums">{lot.occupancy}%</span>
                </li>
              ))}
            </ul>
          </div>
          <p className="mt-4 text-xs text-surface-500 dark:text-surface-400">
            Color: <span className="text-emerald-500 font-medium">green</span> &lt;60% &middot; <span className="text-amber-400 font-medium">yellow</span> 60–80% &middot; <span className="text-red-500 font-medium">red</span> &ge;80%
          </p>
        </div>
      )}

      {/* Occupancy Heatmap */}
      <div className="card p-6">
        <h3 className="text-sm font-semibold text-surface-900 dark:text-white uppercase tracking-wide mb-1">
          {t('heatmap.title')}
        </h3>
        <p className="text-xs text-surface-500 dark:text-surface-400 mb-4">
          {t('heatmap.subtitle')}
        </p>
        <OccupancyHeatmap bookings={heatmapBookings} totalSlots={totalSlots} />
      </div>
    </motion.div>
  );
}
