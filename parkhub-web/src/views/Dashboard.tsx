import { useEffect, useState, useCallback, useMemo } from 'react';
import { Link } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';
import {
  CalendarCheck, Car, Coins, Clock, CalendarPlus, ArrowRight,
  TrendUp, MapPin, Export, Sliders, Broadcast, ArrowUp, ArrowDown,
} from '@phosphor-icons/react';
import { useAuth } from '../context/AuthContext';
import { api, type Booking, type UserStats } from '../api/client';
import { DashboardSkeleton } from '../components/Skeleton';
import { staggerSlow, fadeUp } from '../constants/animations';
import { useWebSocket, type WsEvent } from '../hooks/useWebSocket';
import { BarChart } from '../components/SimpleChart';
import { AnimatedCounter } from '../components/AnimatedCounter';

export function DashboardPage() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const [bookings, setBookings] = useState<Booking[]>([]);
  const [stats, setStats] = useState<UserStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [chartRange, setChartRange] = useState<'7d' | '30d'>('7d');

  const handleWsEvent = useCallback((event: WsEvent) => {
    switch (event.event) {
      case 'booking_created':
        toast.success(t('dashboard.wsBookingCreated', 'New booking created'));
        break;
      case 'booking_cancelled':
        toast(t('dashboard.wsBookingCancelled', 'A booking was cancelled'), { icon: '📋' });
        break;
      case 'occupancy_changed':
        toast(t('dashboard.wsOccupancyChanged', 'Occupancy updated'), { icon: '🅿' });
        break;
    }
  }, [t]);

  useWebSocket({ onEvent: handleWsEvent });

  useEffect(() => {
    Promise.all([api.getBookings(), api.getUserStats()]).then(([bRes, sRes]) => {
      if (bRes.success && bRes.data) setBookings(bRes.data);
      if (sRes.success && sRes.data) setStats(sRes.data);
    }).finally(() => setLoading(false));
  }, []);

  const hour = new Date().getHours();
  const timeOfDay = hour < 12 ? t('dashboard.morning') : hour < 18 ? t('dashboard.afternoon') : t('dashboard.evening');
  const activeBookings = bookings.filter(b => b.status === 'active' || b.status === 'confirmed');
  const name = user?.name?.split(' ')[0] || user?.username || '';

  // Build booking activity chart from real booking data
  const chartDays = chartRange === '7d' ? 7 : 30;
  const activityData = useMemo(() => {
    const days: { label: string; value: number }[] = [];
    const now = new Date();
    for (let i = chartDays - 1; i >= 0; i--) {
      const d = new Date(now);
      d.setDate(d.getDate() - i);
      const dayStr = chartDays <= 7
        ? d.toLocaleDateString(undefined, { weekday: 'short' })
        : d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
      const dateKey = d.toISOString().slice(0, 10);
      const count = bookings.filter(b => b.start_time.slice(0, 10) === dateKey).length;
      days.push({ label: dayStr, value: count });
    }
    return days;
  }, [bookings, chartDays]);

  // Sparkline data for stat cards (last 7 days booking counts)
  const sparklineData = useMemo(() => {
    const counts: number[] = [];
    const now = new Date();
    for (let i = 6; i >= 0; i--) {
      const d = new Date(now);
      d.setDate(d.getDate() - i);
      const dateKey = d.toISOString().slice(0, 10);
      counts.push(bookings.filter(b => b.start_time.slice(0, 10) === dateKey).length);
    }
    return counts;
  }, [bookings]);

  // Mock sensor data for garage status
  const sensorData = useMemo(() => [
    { name: t('dashboard.sensorEntrance1', 'Entrance 1'), status: 'active' as const },
    { name: t('dashboard.sensorEntrance2', 'Entrance 2'), status: 'active' as const },
    { name: t('dashboard.sensorExitA', 'Exit Gate A'), status: 'maintenance' as const },
  ], [t]);

  const container = staggerSlow;
  const item = fadeUp;

  // Time-based gradient accent for greeting
  const greetingGradient = hour < 12
    ? 'from-amber-400/10 via-orange-400/5 to-transparent dark:from-amber-500/8 dark:via-orange-500/3'
    : hour < 18
    ? 'from-sky-400/10 via-blue-400/5 to-transparent dark:from-sky-500/8 dark:via-blue-500/3'
    : 'from-indigo-400/10 via-purple-400/5 to-transparent dark:from-indigo-500/8 dark:via-purple-500/3';

  if (loading) return <div role="status" aria-label={t('dashboard.loadingDashboard')}><DashboardSkeleton /></div>;

  return (
    <AnimatePresence mode="wait">
    <motion.div key="dashboard-loaded" variants={container} initial="hidden" animate="show" className="space-y-6">
      {/* Greeting with quick action buttons */}
      <motion.div
        variants={item}
        className={`relative overflow-hidden rounded-2xl px-6 py-5 bg-gradient-to-r ${greetingGradient}`}
      >
        <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
          <div>
            <div className="flex items-center gap-2 mb-1">
              <span className="w-2 h-2 rounded-full bg-emerald-500 pulse-dot" />
              <span className="text-xs font-medium uppercase tracking-wider text-surface-500 dark:text-surface-400">
                {t('dashboard.systemLive', 'System Live')}
              </span>
            </div>
            <h1 className="text-2xl sm:text-3xl font-bold text-surface-900 dark:text-white tracking-tight" style={{ letterSpacing: '-0.025em' }}>
              {t('dashboard.greeting', { timeOfDay, name })}
            </h1>
          </div>
          <div className="flex items-center gap-2 flex-wrap">
            <Link
              to="/book"
              className="btn btn-ghost btn-sm border border-surface-200/60 dark:border-surface-700/60 text-surface-700 dark:text-surface-300 hover:border-primary-400 dark:hover:border-primary-600"
            >
              <CalendarPlus weight="bold" className="w-3.5 h-3.5" />
              {t('dashboard.addBooking', 'Add Booking')}
            </Link>
            <button
              className="btn btn-ghost btn-sm border border-surface-200/60 dark:border-surface-700/60 text-surface-700 dark:text-surface-300 hover:border-primary-400 dark:hover:border-primary-600"
              onClick={() => toast.success(t('dashboard.exportStarted', 'Export started'))}
            >
              <Export weight="bold" className="w-3.5 h-3.5" />
              {t('dashboard.exportCsv', 'Export CSV')}
            </button>
            <Link
              to="/admin/settings"
              className="btn btn-ghost btn-sm border border-surface-200/60 dark:border-surface-700/60 text-surface-700 dark:text-surface-300 hover:border-primary-400 dark:hover:border-primary-600"
            >
              <Sliders weight="bold" className="w-3.5 h-3.5" />
              {t('dashboard.manageRates', 'Manage Rates')}
            </Link>
          </div>
        </div>
      </motion.div>

      {/* Bento stats grid with sparklines and change badges */}
      <motion.div
        variants={item}
        className="grid grid-cols-2 lg:grid-cols-4 gap-3"
        role="region"
        aria-label={t('dashboard.statistics')}
      >
        <BentoStatCard
          label={t('dashboard.activeBookings')}
          value={activeBookings.length}
          gradient="from-primary-500 to-primary-300"
          borderColor="var(--color-primary-500)"
          live={activeBookings.length > 0}
          sparkline={sparklineData}
          change={activeBookings.length > 0 ? 12 : 0}
        />

        <BentoStatCard
          label={t('dashboard.creditsLeft')}
          value={user?.credits_balance ?? 0}
          gradient="from-accent-500 to-accent-300"
          borderColor="var(--color-accent-500)"
          sparkline={sparklineData}
        />

        <BentoStatCard
          label={t('dashboard.thisMonth')}
          value={stats?.bookings_this_month ?? 0}
          gradient="from-blue-500 to-cyan-400"
          borderColor="var(--color-blue-500, #3b82f6)"
          change={8}
        />

        <NextBookingCard
          label={t('dashboard.nextBooking')}
          value={activeBookings.length > 0 ? formatTime(activeBookings[0].start_time) : '—'}
        />
      </motion.div>

      {/* Occupancy chart + Sensor feed — bento layout */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        {/* Occupancy Trends — spans 2 cols */}
        <motion.div
          variants={item}
          className="lg:col-span-2 glass-card p-6"
        >
          <div className="flex items-center justify-between mb-4">
            <div>
              <div className="flex items-center gap-2">
                <TrendUp weight="bold" className="w-4 h-4 text-primary-500" />
                <h2 className="text-lg font-semibold text-surface-900 dark:text-white tracking-tight">
                  {t('dashboard.occupancyTrends', 'Weekly Occupancy Trends')}
                </h2>
              </div>
              <p className="text-xs text-surface-500 dark:text-surface-400 mt-0.5">
                {t('dashboard.occupancySubtitle', 'Daily volume of bookings across all lots')}
              </p>
            </div>
            <div className="flex rounded-lg border border-surface-200/60 dark:border-surface-700/60 overflow-hidden" role="tablist" aria-label={t('dashboard.chartRange', 'Chart range')}>
              <button
                role="tab"
                aria-selected={chartRange === '7d'}
                onClick={() => setChartRange('7d')}
                className={`px-3 py-1.5 text-xs font-medium transition-colors ${
                  chartRange === '7d'
                    ? 'bg-primary-50 dark:bg-primary-950/30 text-primary-700 dark:text-primary-300'
                    : 'text-surface-500 dark:text-surface-400 hover:bg-surface-50 dark:hover:bg-surface-800/40'
                }`}
              >
                {t('dashboard.7days', '7 Days')}
              </button>
              <button
                role="tab"
                aria-selected={chartRange === '30d'}
                onClick={() => setChartRange('30d')}
                className={`px-3 py-1.5 text-xs font-medium transition-colors ${
                  chartRange === '30d'
                    ? 'bg-primary-50 dark:bg-primary-950/30 text-primary-700 dark:text-primary-300'
                    : 'text-surface-500 dark:text-surface-400 hover:bg-surface-50 dark:hover:bg-surface-800/40'
                }`}
              >
                {t('dashboard.30days', '30 Days')}
              </button>
            </div>
          </div>
          {activityData.some(d => d.value > 0) ? (
            <BarChart data={activityData} color="var(--color-teal-500, #14b8a6)" />
          ) : (
            <p className="text-sm text-surface-500 dark:text-surface-400 py-6 text-center">
              {t('dashboard.noActiveBookings')}
            </p>
          )}
        </motion.div>

        {/* Main Garage Status — sensor feed */}
        <motion.div
          variants={item}
          className="glass-card p-6"
        >
          <h2 className="text-lg font-semibold text-surface-900 dark:text-white mb-1 tracking-tight">
            {t('dashboard.garageStatus', 'Main Garage Status')}
          </h2>
          <p className="text-xs text-emerald-600 dark:text-emerald-400 mb-5">
            {t('dashboard.garageStatusHint', 'Zone A-4 is nearly full')}
          </p>

          <div className="space-y-4">
            <div className="flex items-center gap-2 mb-3">
              <Broadcast weight="bold" className="w-4 h-4 text-primary-500" />
              <h3 className="text-sm font-semibold text-surface-900 dark:text-white">
                {t('dashboard.liveSensorFeed', 'Live Sensor Feed')}
              </h3>
            </div>
            {sensorData.map((sensor) => (
              <div key={sensor.name} className="flex items-center justify-between">
                <span className="text-sm text-surface-600 dark:text-surface-400">{sensor.name}</span>
                <span className={`text-xs font-semibold ${
                  sensor.status === 'active'
                    ? 'text-emerald-600 dark:text-emerald-400'
                    : 'text-amber-600 dark:text-amber-400'
                }`}>
                  {sensor.status === 'active'
                    ? t('dashboard.sensorActive', 'Active')
                    : t('dashboard.sensorMaintenance', 'Maintenance')}
                </span>
              </div>
            ))}
          </div>
        </motion.div>
      </div>

      {/* Active bookings + Quick actions — bento layout */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        {/* Active bookings — spans 2 cols */}
        <motion.div
          variants={item}
          className="lg:col-span-2 glass-card p-6"
        >
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <h2 className="text-lg font-semibold text-surface-900 dark:text-white tracking-tight">
                {t('dashboard.activeBookings')}
              </h2>
              {activeBookings.length > 0 && (
                <span className="flex items-center gap-1.5 text-xs font-medium text-emerald-600 dark:text-emerald-400">
                  <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 pulse-dot" />
                  {activeBookings.length}
                </span>
              )}
            </div>
            <Link to="/bookings" className="text-sm text-primary-600 hover:text-primary-500 dark:text-primary-400 font-medium flex items-center gap-1 transition-colors">
              {t('nav.bookings')} <ArrowRight weight="bold" className="w-3.5 h-3.5" />
            </Link>
          </div>

          {activeBookings.length === 0 ? (
            <div className="py-12 text-center">
              <p className="text-surface-500 dark:text-surface-400 mb-4">{t('dashboard.noActiveBookings')}</p>
              <Link to="/book" className="btn btn-primary">{t('dashboard.bookSpot')}</Link>
            </div>
          ) : (
            <div className="space-y-2">
              {activeBookings.slice(0, 5).map((b, i) => (
                <motion.div
                  key={b.id}
                  initial={{ opacity: 0, y: 8 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: i * 0.05, type: 'spring', stiffness: 300, damping: 24 }}
                  className="flex items-center gap-4 p-3 rounded-xl bg-surface-50/80 dark:bg-surface-800/40 border border-surface-100 dark:border-surface-700/50 transition-all hover:bg-surface-100 dark:hover:bg-surface-700/40 hover:shadow-sm group"
                >
                  <div className="w-10 h-10 flex items-center justify-center rounded-lg bg-primary-50 dark:bg-primary-950/30 border border-primary-200/50 dark:border-primary-800/50 group-hover:border-primary-300 dark:group-hover:border-primary-700 transition-colors">
                    <span className="text-sm font-bold text-primary-700 dark:text-primary-300" style={{ fontVariantNumeric: 'tabular-nums' }}>
                      {b.slot_number}
                    </span>
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="font-medium text-surface-900 dark:text-white truncate">{b.lot_name}</p>
                    <div className="flex items-center gap-2 text-sm text-surface-500 dark:text-surface-400">
                      <MapPin weight="regular" className="w-3.5 h-3.5" />
                      {t('dashboard.slot')} {b.slot_number}
                      {b.vehicle_plate && <><span className="mx-1">·</span><Car weight="regular" className="w-3.5 h-3.5" />{b.vehicle_plate}</>}
                    </div>
                  </div>
                  <div className="text-right">
                    <span className="badge badge-success">{t('bookings.statusActive')}</span>
                  </div>
                </motion.div>
              ))}
            </div>
          )}
        </motion.div>

        {/* Quick actions */}
        <motion.div
          variants={item}
          className="glass-card p-6"
        >
          <h2 className="text-lg font-semibold text-surface-900 dark:text-white mb-4 tracking-tight">
            {t('dashboard.quickActions')}
          </h2>
          <div className="space-y-2">
            {[
              { to: '/book', icon: CalendarPlus, label: t('dashboard.bookSpot'), accent: true },
              { to: '/vehicles', icon: Car, label: t('dashboard.myVehicles') },
              { to: '/bookings', icon: CalendarCheck, label: t('dashboard.viewBookings') },
              { to: '/credits', icon: Coins, label: t('nav.credits') },
            ].map((action) => (
              <Link
                key={action.to}
                to={action.to}
                className={`flex items-center gap-3 px-4 py-3 text-sm font-medium rounded-xl border transition-all group
                  ${action.accent
                    ? 'text-primary-700 dark:text-primary-300 border-primary-200/60 dark:border-primary-800/60 bg-primary-50/50 dark:bg-primary-950/20 hover:bg-primary-100/80 dark:hover:bg-primary-950/40 hover:border-primary-300 dark:hover:border-primary-700 hover:shadow-sm'
                    : 'text-surface-700 dark:text-surface-300 border-surface-200/60 dark:border-surface-700/60 bg-white/50 dark:bg-surface-800/30 hover:bg-surface-50 dark:hover:bg-surface-700/40 hover:border-surface-300 dark:hover:border-surface-600 hover:shadow-sm'
                  }`}
              >
                <action.icon weight="regular" className={`w-4 h-4 transition-transform group-hover:scale-110 ${action.accent ? 'text-primary-500' : 'text-surface-400 dark:text-surface-500'}`} />
                <span className="flex-1">{action.label}</span>
                <ArrowRight weight="bold" className="w-3.5 h-3.5 text-surface-400 opacity-0 group-hover:opacity-100 transition-opacity" />
              </Link>
            ))}
          </div>
        </motion.div>
      </div>

      {/* Recent Activity table */}
      <motion.div
        variants={item}
        className="glass-card p-6"
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-surface-900 dark:text-white tracking-tight">
            {t('dashboard.recentActivity', 'Recent Activity')}
          </h2>
          <Link to="/bookings" className="text-sm text-primary-600 hover:text-primary-500 dark:text-primary-400 font-medium">
            {t('dashboard.viewAll', 'View All')}
          </Link>
        </div>
        <div className="overflow-x-auto -mx-2 px-2">
          <table className="w-full text-sm" style={{ minWidth: '36rem' }}>
            <thead>
              <tr className="text-left text-xs font-medium text-surface-500 dark:text-surface-400 uppercase tracking-wider">
                <th className="pb-3 pr-4">{t('dashboard.colVehicle', 'Vehicle / Owner')}</th>
                <th className="pb-3 pr-4">{t('dashboard.colSlot', 'Slot No.')}</th>
                <th className="pb-3 pr-4">{t('dashboard.colCheckIn', 'Check-in Time')}</th>
                <th className="pb-3 pr-4">{t('dashboard.colDuration', 'Duration')}</th>
                <th className="pb-3">{t('dashboard.colStatus', 'Status')}</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-surface-100 dark:divide-surface-800/50">
              {bookings.slice(0, 5).map((b) => (
                <tr key={b.id} className="group">
                  <td className="py-3 pr-4">
                    <div className="flex items-center gap-3">
                      <div className="w-8 h-8 rounded-full bg-surface-100 dark:bg-surface-800 flex items-center justify-center">
                        <Car weight="fill" className="w-4 h-4 text-surface-500 dark:text-surface-400" />
                      </div>
                      <div>
                        <p className="font-medium text-surface-900 dark:text-white">{b.lot_name}</p>
                        <p className="text-xs text-surface-500 dark:text-surface-400">{b.vehicle_plate || '—'}</p>
                      </div>
                    </div>
                  </td>
                  <td className="py-3 pr-4 font-medium text-surface-900 dark:text-white tabular-nums">{b.slot_number}</td>
                  <td className="py-3 pr-4 text-surface-600 dark:text-surface-400 tabular-nums">{formatTime(b.start_time)}</td>
                  <td className="py-3 pr-4 text-surface-600 dark:text-surface-400 tabular-nums">{formatDuration(b.start_time, b.end_time)}</td>
                  <td className="py-3">
                    <span className={`badge ${
                      b.status === 'active' ? 'badge-success'
                      : b.status === 'confirmed' ? 'badge-info'
                      : b.status === 'cancelled' ? 'badge-error'
                      : 'badge-gray'
                    }`}>
                      {b.status === 'active' ? t('dashboard.statusInProgress', 'In Progress')
                       : b.status === 'confirmed' ? t('dashboard.statusConfirmed', 'Confirmed')
                       : b.status === 'cancelled' ? t('dashboard.statusCancelled', 'Cancelled')
                       : t('dashboard.statusPending', 'Pending')}
                    </span>
                  </td>
                </tr>
              ))}
              {bookings.length === 0 && (
                <tr>
                  <td colSpan={5} className="py-8 text-center text-surface-500 dark:text-surface-400">
                    {t('dashboard.noActiveBookings')}
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      </motion.div>
    </motion.div>
    </AnimatePresence>
  );
}

/* ── Sparkline mini-chart (inline SVG) ──── */

function Sparkline({ data, color = 'var(--color-primary-500)' }: { data: number[]; color?: string }) {
  if (data.length < 2) return null;
  const max = Math.max(...data, 1);
  const h = 24;
  const w = 56;
  const step = w / (data.length - 1);
  const points = data.map((v, i) => `${i * step},${h - (v / max) * h}`).join(' ');
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} className="block opacity-40 group-hover:opacity-60 transition-opacity">
      <polyline fill="none" stroke={color} strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" points={points} />
    </svg>
  );
}

/* ── Bento Stat Card with animated counter, sparkline and gradient top border ──── */

function BentoStatCard({ label, value, gradient, borderColor, live, sparkline, change }: {
  label: string;
  value: number | string;
  gradient?: string;
  borderColor?: string;
  live?: boolean;
  sparkline?: number[];
  change?: number;
}) {
  const isNum = typeof value === 'number';

  return (
    <motion.div
      whileHover={{ scale: 1.02, y: -2 }}
      transition={{ type: 'spring', stiffness: 400, damping: 25 }}
      className="relative overflow-hidden glass-card p-4 group"
      style={borderColor ? { borderTop: `2px solid ${borderColor}` } : undefined}
    >
      {/* Subtle gradient accent in top-right */}
      {gradient && (
        <div className={`absolute -top-6 -right-6 w-16 h-16 rounded-full bg-gradient-to-br ${gradient} opacity-20 group-hover:opacity-30 transition-opacity blur-xl`} />
      )}

      <div className="relative">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-1.5">
            <p className="text-sm font-medium text-surface-500 dark:text-surface-400">{label}</p>
            {live && <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 pulse-dot" />}
          </div>
          {change !== undefined && change !== 0 && (
            <span className={`flex items-center gap-0.5 text-[10px] font-bold px-1.5 py-0.5 rounded-full ${
              change > 0
                ? 'bg-emerald-100 dark:bg-emerald-950/30 text-emerald-700 dark:text-emerald-400'
                : 'bg-red-100 dark:bg-red-950/30 text-red-700 dark:text-red-400'
            }`}>
              {change > 0 ? <ArrowUp weight="bold" className="w-2.5 h-2.5" /> : <ArrowDown weight="bold" className="w-2.5 h-2.5" />}
              {Math.abs(change)}%
            </span>
          )}
        </div>
        <div className="flex items-end justify-between mt-2">
          <p
            className="text-2xl font-bold text-surface-900 dark:text-white"
            style={{ fontVariantNumeric: 'tabular-nums', letterSpacing: '-0.03em', lineHeight: 1 }}
          >
            {isNum ? <AnimatedCounter value={value as number} duration={800} /> : value}
          </p>
          {sparkline && <Sparkline data={sparkline} color={borderColor} />}
        </div>
      </div>
    </motion.div>
  );
}

function NextBookingCard({ label, value }: {
  label: string; value: string;
}) {
  return (
    <motion.div
      whileHover={{ scale: 1.02, y: -2 }}
      transition={{ type: 'spring', stiffness: 400, damping: 25 }}
      className="relative overflow-hidden glass-card p-4 gradient-border group"
    >
      <div className="absolute -top-6 -right-6 w-16 h-16 rounded-full bg-gradient-to-br from-primary-500 to-accent-400 opacity-15 group-hover:opacity-25 transition-opacity blur-xl" />
      <div className="relative">
        <div className="flex items-center gap-1.5">
          <p className="text-sm font-medium text-surface-500 dark:text-surface-400">{label}</p>
          {value !== '—' && <Clock weight="bold" className="w-3 h-3 text-primary-500" />}
        </div>
        <p
          className="mt-2 text-2xl font-bold text-surface-900 dark:text-white"
          style={{ fontVariantNumeric: 'tabular-nums', letterSpacing: '-0.03em', lineHeight: 1 }}
        >
          {value}
        </p>
      </div>
    </motion.div>
  );
}

function formatTime(dateStr: string) {
  const d = new Date(dateStr);
  return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
}

function formatDuration(startStr: string, endStr: string): string {
  const start = new Date(startStr);
  const end = new Date(endStr);
  const diffMs = end.getTime() - start.getTime();
  if (diffMs <= 0) return '—';
  const hours = Math.floor(diffMs / (1000 * 60 * 60));
  const mins = Math.floor((diffMs % (1000 * 60 * 60)) / (1000 * 60));
  if (hours === 0) return `${mins}m`;
  return `${hours}h ${mins}m`;
}
