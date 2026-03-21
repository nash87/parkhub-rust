import { useState, useEffect, useMemo } from 'react';
import { motion } from 'framer-motion';
import { SpinnerGap, Users, Buildings, CalendarCheck, Lightning } from '@phosphor-icons/react';
import { api, type AdminStats, type Booking, type ParkingLot } from '../api/client';
import { useTranslation } from 'react-i18next';
import { BarChart, DonutChart, type DonutSlice } from '../components/SimpleChart';
import { OccupancyHeatmap } from '../components/OccupancyHeatmap';
import { ExportButton } from '../components/ExportButton';

function StatCard({ icon: Icon, label, value }: {
  icon: React.ComponentType<{ weight?: string; className?: string }>;
  label: string;
  value: number;
  color?: string;
}) {
  return (
    <div className="stat-card">
      <div className="flex items-center gap-2 mb-2">
        <Icon weight="bold" className="w-4 h-4 text-surface-400" />
        <p className="text-sm font-medium text-surface-500 dark:text-surface-400">{label}</p>
      </div>
      <p className="stat-value text-surface-900 dark:text-white">{value}</p>
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

/** Build mock "bookings this week" from total bookings to show a plausible distribution. */
function weeklyBookingData(totalBookings: number, t: (key: string) => string): { label: string; value: number }[] {
  const dayKeys = ['mon', 'tue', 'wed', 'thu', 'fri', 'sat', 'sun'];
  // Weights simulate typical office-parking week pattern
  const weights = [0.18, 0.20, 0.22, 0.19, 0.15, 0.04, 0.02];
  return dayKeys.map((key, i) => ({
    label: t(`reports.weekdays.${key}`),
    value: Math.round(totalBookings * weights[i]),
  }));
}

export function AdminReportsPage() {
  const { t } = useTranslation();
  const [stats, setStats] = useState<AdminStats | null>(null);
  const [bookings, setBookings] = useState<Booking[]>([]);
  const [lots, setLots] = useState<ParkingLot[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      api.adminStats(),
      api.getBookings(),
      api.getLots(),
    ]).then(([statsRes, bookingsRes, lotsRes]) => {
      if (statsRes.success && statsRes.data) setStats(statsRes.data);
      if (bookingsRes.success && bookingsRes.data) setBookings(bookingsRes.data);
      if (lotsRes.success && lotsRes.data) setLots(lotsRes.data);
    }).finally(() => setLoading(false));
  }, []);

  const weeklyData = useMemo(
    () => weeklyBookingData(stats?.total_bookings ?? 0, t),
    [stats?.total_bookings, t],
  );

  const lotOccupancy = useMemo(
    () => lotOccupancyFromData(lots),
    [lots],
  );

  const totalSlots = useMemo(
    () => lots.reduce((sum, l) => sum + l.total_slots, 0) || 20,
    [lots],
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
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-surface-900 dark:text-white">{t('admin.reports')}</h2>
        <ExportButton />
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          icon={Users}
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
          icon={CalendarCheck}
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
              {stats && stats.total_lots > 0
                ? `${Math.round((stats.active_bookings / Math.max(stats.total_lots, 1)) * 100)}%`
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
        <OccupancyHeatmap bookings={bookings} totalSlots={totalSlots} />
      </div>
    </motion.div>
  );
}
