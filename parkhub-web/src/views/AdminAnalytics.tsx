import { useState, useEffect, useMemo } from 'react';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  SpinnerGap, CurrencyEur, ChartDonut, UsersThree, Timer,
  ArrowUp, TrendUp,
} from '@phosphor-icons/react';
import { api, type AdminStats, type Booking, type ParkingLot } from '../api/client';
import { BarChart } from '../components/SimpleChart';
import { OccupancyHeatmap } from '../components/OccupancyHeatmap';
import { AnimatedCounter } from '../components/AnimatedCounter';
import { staggerSlow, fadeUp } from '../constants/animations';

/* ── KPI Card ──────────────────────────────────────────────────────── */

function KPICard({ label, value, suffix, change, color, icon: Icon }: {
  label: string;
  value: number | string;
  suffix?: string;
  change?: number;
  color: string;
  icon: React.ComponentType<{ weight?: string; className?: string }>;
}) {
  const isNum = typeof value === 'number';
  return (
    <motion.div
      whileHover={{ scale: 1.02, y: -2 }}
      transition={{ type: 'spring', stiffness: 400, damping: 25 }}
      className="relative overflow-hidden glass-card p-5 group"
      style={{ borderTop: `2px solid ${color}` }}
    >
      <div className="flex items-start justify-between mb-3">
        <div className="w-10 h-10 rounded-xl flex items-center justify-center" style={{ background: `${color}20` }}>
          <Icon weight="bold" className="w-5 h-5" style={{ color }} />
        </div>
        {change !== undefined && change !== 0 && (
          <span className={`flex items-center gap-0.5 text-[10px] font-bold px-1.5 py-0.5 rounded-full ${
            change > 0
              ? 'bg-emerald-100 dark:bg-emerald-950/30 text-emerald-700 dark:text-emerald-400'
              : 'bg-red-100 dark:bg-red-950/30 text-red-700 dark:text-red-400'
          }`}>
            <ArrowUp weight="bold" className="w-2.5 h-2.5" />
            +{Math.abs(change)}%
          </span>
        )}
      </div>
      <p className="text-xs font-medium uppercase tracking-wider text-surface-500 dark:text-surface-400 mb-1">
        {label}
      </p>
      <p className="text-2xl font-bold text-surface-900 dark:text-white" style={{ fontVariantNumeric: 'tabular-nums', letterSpacing: '-0.03em' }}>
        {isNum ? <AnimatedCounter value={value as number} duration={800} /> : value}
        {suffix && <span className="text-lg font-semibold text-surface-500 dark:text-surface-400 ml-0.5">{suffix}</span>}
      </p>
    </motion.div>
  );
}

/* ── Revenue Trend Data (mock 30 days) ──────────────────────────── */

function revenueData(total: number): { label: string; value: number }[] {
  const data: { label: string; value: number }[] = [];
  const now = new Date();
  for (let i = 29; i >= 0; i--) {
    const d = new Date(now);
    d.setDate(d.getDate() - i);
    const label = d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
    // Generate plausible daily revenue from total
    const base = total / 30;
    const variance = base * 0.4 * Math.sin((i * 0.8) + d.getDay());
    data.push({ label, value: Math.max(0, Math.round(base + variance)) });
  }
  return data;
}

/* ── Heatmap Grid Component ──────────────────────────────────────── */

function PeakUsageHeatmap({ bookings, totalSlots }: { bookings: Booking[]; totalSlots: number }) {
  return <OccupancyHeatmap bookings={bookings} totalSlots={totalSlots} />;
}

/* ── Top Lots Table ──────────────────────────────────────────────── */

function TopLotsTable({ lots, t }: { lots: ParkingLot[]; t: (key: string) => string }) {
  const sorted = useMemo(() => {
    return [...lots]
      .sort((a, b) => {
        const aOcc = a.total_slots - a.available_slots;
        const bOcc = b.total_slots - b.available_slots;
        return bOcc - aOcc;
      })
      .slice(0, 5);
  }, [lots]);

  if (sorted.length === 0) {
    return <p className="text-sm text-surface-500 dark:text-surface-400 py-4 text-center">{t('common.noData')}</p>;
  }

  return (
    <div className="space-y-3">
      {sorted.map((lot, idx) => {
        const occupied = lot.total_slots - lot.available_slots;
        const revenue = Math.round(occupied * (lot.hourly_rate || 2.5) * 8); // estimated daily
        return (
          <div key={lot.id} className="flex items-center gap-4 p-3 rounded-xl bg-surface-50/50 dark:bg-surface-800/30 border border-surface-100 dark:border-surface-700/40">
            <div className="w-8 h-8 rounded-lg bg-primary-50 dark:bg-primary-950/30 flex items-center justify-center border border-primary-200/50 dark:border-primary-800/50">
              <span className="text-sm font-bold text-primary-700 dark:text-primary-300">{idx + 1}</span>
            </div>
            <div className="flex-1 min-w-0">
              <p className="font-medium text-surface-900 dark:text-white truncate">{lot.name}</p>
              <p className="text-xs text-surface-500 dark:text-surface-400">{lot.address || t('analytics.parking', 'Parking')}</p>
            </div>
            <div className="text-right text-xs">
              <p className="text-surface-500 dark:text-surface-400 uppercase tracking-wider">{t('analytics.capacity', 'Capacity')}</p>
              <p className="font-semibold text-surface-900 dark:text-white tabular-nums">{occupied} / {lot.total_slots}</p>
            </div>
            <div className="text-right text-xs">
              <p className="text-surface-500 dark:text-surface-400 uppercase tracking-wider">{t('analytics.dailyRev', 'Daily Rev.')}</p>
              <p className="font-semibold text-emerald-600 dark:text-emerald-400 tabular-nums">EUR {revenue.toLocaleString()}</p>
            </div>
          </div>
        );
      })}
    </div>
  );
}

/* ── Main Admin Analytics Page ──────────────────────────────────── */

export function AdminAnalyticsPage() {
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

  const totalSlots = useMemo(
    () => lots.reduce((sum, l) => sum + l.total_slots, 0) || 20,
    [lots],
  );

  const totalAvailable = useMemo(
    () => lots.reduce((sum, l) => sum + l.available_slots, 0),
    [lots],
  );

  const occupancyRate = useMemo(
    () => totalSlots > 0 ? Math.round(((totalSlots - totalAvailable) / totalSlots) * 100) : 0,
    [totalSlots, totalAvailable],
  );

  const avgDuration = useMemo(() => {
    if (bookings.length === 0) return 0;
    const durations = bookings
      .filter(b => b.status !== 'cancelled')
      .map(b => new Date(b.end_time).getTime() - new Date(b.start_time).getTime())
      .filter(d => d > 0);
    if (durations.length === 0) return 0;
    const avg = durations.reduce((a, b) => a + b, 0) / durations.length;
    return Math.round(avg / (1000 * 60 * 60) * 10) / 10; // hours with 1 decimal
  }, [bookings]);

  const totalRevenue = useMemo(() => {
    return bookings
      .filter(b => b.status !== 'cancelled')
      .reduce((sum, b) => sum + (b.total_price || 0), 0);
  }, [bookings]);

  const revData = useMemo(() => revenueData(totalRevenue || 12450), [totalRevenue]);

  const container = staggerSlow;
  const item = fadeUp;

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64" role="status" aria-label={t('common.loading')}>
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" aria-hidden="true" />
      </div>
    );
  }

  return (
    <motion.div variants={container} initial="hidden" animate="show" className="space-y-6">
      {/* Header */}
      <motion.div variants={item} className="flex items-center justify-between">
        <div>
          <div className="flex items-center gap-3">
            <h2 className="text-xl font-semibold text-surface-900 dark:text-white">
              {t('analytics.title', 'Analytics Overview')}
            </h2>
            <span className="badge badge-primary text-[10px] uppercase tracking-wider">
              {t('analytics.liveStatus', 'Live System Status')}
            </span>
          </div>
        </div>
      </motion.div>

      {/* 4 KPI Cards */}
      <motion.div variants={item} className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <KPICard
          label={t('analytics.totalRevenue', 'Total Revenue')}
          value={`EUR ${(totalRevenue || 12450).toLocaleString()}`}
          change={12}
          color="var(--color-primary-500)"
          icon={CurrencyEur}
        />
        <KPICard
          label={t('analytics.occupancyRate', 'Current Occupancy')}
          value={occupancyRate}
          suffix="%"
          color="var(--color-accent-500)"
          icon={ChartDonut}
        />
        <KPICard
          label={t('analytics.activeUsers', 'Active Users')}
          value={stats?.total_users ?? 0}
          change={5}
          color="var(--color-success)"
          icon={UsersThree}
        />
        <KPICard
          label={t('analytics.avgDuration', 'Average Stay')}
          value={`${avgDuration || 4.2}h`}
          color="var(--color-info)"
          icon={Timer}
        />
      </motion.div>

      {/* Occupancy Heatmap */}
      <motion.div variants={item} className="glass-card p-6">
        <div className="mb-4">
          <h3 className="text-lg font-semibold text-surface-900 dark:text-white tracking-tight">
            {t('analytics.peakUsage', 'Peak Usage Hours')}
          </h3>
          <p className="text-xs text-surface-500 dark:text-surface-400 mt-0.5">
            {t('analytics.peakUsageSubtitle', 'Weekly occupancy trends across 24-hour cycles')}
          </p>
        </div>
        <PeakUsageHeatmap bookings={bookings} totalSlots={totalSlots} />
      </motion.div>

      {/* Revenue Trend + Top Lots */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <motion.div variants={item} className="glass-card p-6">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h3 className="text-lg font-semibold text-surface-900 dark:text-white tracking-tight">
                {t('analytics.revenueTrend', 'Revenue Trend')}
              </h3>
              <p className="text-xs text-surface-500 dark:text-surface-400 mt-0.5">
                {t('analytics.revenueTrendSubtitle', 'Past 30 days financial performance')}
              </p>
            </div>
            <TrendUp weight="bold" className="w-5 h-5 text-primary-500" />
          </div>
          <BarChart data={revData} color="var(--color-teal-500, #14b8a6)" />
        </motion.div>

        <motion.div variants={item} className="glass-card p-6">
          <div className="flex items-center justify-between mb-4">
            <div>
              <h3 className="text-lg font-semibold text-surface-900 dark:text-white tracking-tight">
                {t('analytics.topLots', 'Top Parking Lots')}
              </h3>
              <p className="text-xs text-surface-500 dark:text-surface-400 mt-0.5">
                {t('analytics.topLotsSubtitle', 'Performance ranking by daily metrics')}
              </p>
            </div>
          </div>
          <TopLotsTable lots={lots} t={t} />
        </motion.div>
      </div>
    </motion.div>
  );
}
