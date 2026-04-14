import { useEffect, useState, useCallback, useMemo } from 'react';
import { Link } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';
import {
  CalendarCheck, Car, Coins, CalendarPlus, ArrowRight,
  MapPin, ChartLine, Gauge,
} from '@phosphor-icons/react';
import { useAuth } from '../context/AuthContext';
import { api, type Booking, type UserStats } from '../api/client';
import { DashboardSkeleton } from '../components/Skeleton';
import { staggerSlow, fadeUp } from '../constants/animations';
import { useWebSocket, type WsEvent } from '../hooks/useWebSocket';
import {
  KpiCard,
  TrendCard,
  SensorFeedCard,
  RecentActivityCard,
  type ActivityRow,
  type SensorEntry,
} from '../components/KineticObservatory';

export function DashboardPage() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const [bookings, setBookings] = useState<Booking[]>([]);
  const [stats, setStats] = useState<UserStats | null>(null);
  const [loading, setLoading] = useState(true);

  const handleWsEvent = useCallback((event: WsEvent) => {
    switch (event.event) {
      case 'booking_created':
        toast.success(t('dashboard.wsBookingCreated', 'New booking created'));
        break;
      case 'booking_cancelled':
        toast(t('dashboard.wsBookingCancelled', 'A booking was cancelled'), { icon: '📋' });
        break;
      case 'occupancy_changed':
        break; // Occupancy updates handled via hook state
    }
  }, [t]);

  const { connected: wsConnected } = useWebSocket({ onEvent: handleWsEvent });

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

  // Trend period selector
  const [trendPeriod, setTrendPeriod] = useState<'7d' | '30d'>('7d');

  // Build activity chart from real booking data
  const activityTrend = useMemo(() => {
    const daysBack = trendPeriod === '7d' ? 7 : 30;
    const days: { label: string; value: number }[] = [];
    const now = new Date();
    for (let i = daysBack - 1; i >= 0; i--) {
      const d = new Date(now);
      d.setDate(d.getDate() - i);
      const dayStr = d.toLocaleDateString(undefined, { weekday: 'short' });
      const dateKey = d.toISOString().slice(0, 10);
      const count = bookings.filter(b => b.start_time.slice(0, 10) === dateKey).length;
      days.push({ label: dayStr, value: count });
    }
    return days;
  }, [bookings, trendPeriod]);

  // Recent activity rows (derived from bookings)
  const recentActivity = useMemo<ActivityRow[]>(() => {
    const mapStatus = (s: string): ActivityRow['status'] => {
      if (s === 'active') return 'in_progress';
      if (s === 'confirmed') return 'confirmed';
      if (s === 'completed') return 'completed';
      if (s === 'cancelled') return 'cancelled';
      return 'pending';
    };
    return bookings
      .slice(0, 5)
      .map((b) => {
        const start = new Date(b.start_time);
        const end = new Date(b.end_time);
        const durationMs = end.getTime() - start.getTime();
        const hours = Math.floor(durationMs / 3600000);
        const minutes = Math.floor((durationMs % 3600000) / 60000);
        return {
          id: b.id,
          vehicle: b.vehicle_plate || t('dashboard.unknownVehicle', 'Unknown vehicle'),
          owner: b.lot_name,
          slot: b.slot_number,
          checkInTime: start.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' }),
          duration: `${hours}h ${minutes}m`,
          status: mapStatus(b.status),
        };
      });
  }, [bookings, t]);

  // Live sensor feed — derived from lots (placeholder until a real sensor API exists)
  const sensorFeed = useMemo<SensorEntry[]>(() => {
    const lots = Array.from(new Set(bookings.map((b) => b.lot_name).filter(Boolean))).slice(0, 4);
    if (lots.length === 0) {
      return [
        { name: t('dashboard.entranceGateA', 'Entrance Gate A'), status: 'active' },
        { name: t('dashboard.entranceGateB', 'Entrance Gate B'), status: 'active' },
        { name: t('dashboard.exitGate', 'Exit Gate'), status: 'active' },
      ];
    }
    return lots.map((lot, idx) => ({
      name: lot,
      status: idx === lots.length - 1 && lots.length > 2 ? 'maintenance' : 'active',
    } as SensorEntry));
  }, [bookings, t]);

  // KPI derived values
  const totalBookings = stats?.total_bookings ?? bookings.length;
  const bookingsThisMonth = stats?.bookings_this_month ?? 0;
  const creditsLeft = user?.credits_balance ?? 0;
  // Delta: month-over-month estimate from current snapshot (simplified)
  const monthDelta = bookingsThisMonth > 0 ? Math.round((bookingsThisMonth / Math.max(totalBookings, 1)) * 100) : 0;

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
      {/* Greeting with time-based gradient accent */}
      <motion.div
        variants={item}
        className={`relative overflow-hidden rounded-2xl px-6 py-5 bg-gradient-to-r ${greetingGradient}`}
      >
        <div className="flex items-center gap-3">
          <h1 className="text-2xl sm:text-3xl font-bold text-surface-900 dark:text-white tracking-tight" style={{ letterSpacing: '-0.025em' }}>
            {t('dashboard.greeting', { timeOfDay, name })}
          </h1>
          {wsConnected && (
            <span
              className="inline-flex items-center gap-1.5 text-xs font-medium text-emerald-600 dark:text-emerald-400"
              title={t('dashboard.wsConnected', 'Live updates active')}
              data-testid="ws-connected-indicator"
            >
              <span className="w-2 h-2 rounded-full bg-emerald-500 pulse-dot" />
              {t('dashboard.live', 'Live')}
            </span>
          )}
        </div>
      </motion.div>

      {/* KPI Row — Kinetic Observatory style with delta badges */}
      <motion.div
        variants={item}
        className="grid grid-cols-2 lg:grid-cols-4 gap-3"
        role="region"
        aria-label={t('dashboard.statistics')}
      >
        <KpiCard
          label={t('dashboard.activeBookings')}
          value={activeBookings.length}
          icon={<ChartLine weight="bold" />}
          live={activeBookings.length > 0}
          data-testid="kpi-active-bookings"
        />
        <KpiCard
          label={t('dashboard.creditsLeft')}
          value={creditsLeft}
          icon={<Coins weight="bold" />}
          data-testid="kpi-credits"
        />
        <KpiCard
          label={t('dashboard.thisMonth')}
          value={bookingsThisMonth}
          icon={<CalendarCheck weight="bold" />}
          delta={monthDelta > 0 ? { value: monthDelta, suffix: '%' } : undefined}
          data-testid="kpi-this-month"
        />
        <KpiCard
          label={t('dashboard.totalBookings', 'Total Bookings')}
          value={totalBookings}
          icon={<Gauge weight="bold" />}
          data-testid="kpi-total"
        />
      </motion.div>

      {/* Trend chart + Live sensor feed */}
      <motion.div variants={item} className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <div className="lg:col-span-2">
          <TrendCard
            title={t('dashboard.weeklyActivityTitle', 'Weekly Activity')}
            subtitle={t('dashboard.weeklyActivitySubtitle', 'Booking volume over the selected period')}
            points={activityTrend.map((d) => d.value)}
            labels={activityTrend.map((d) => d.label).filter((_, i, arr) => arr.length <= 7 || i % Math.ceil(arr.length / 7) === 0).slice(0, 7)}
            periods={[
              { key: '7d', label: t('dashboard.period7d', '7 Days') },
              { key: '30d', label: t('dashboard.period30d', '30 Days') },
            ]}
            activePeriod={trendPeriod}
            onPeriodChange={(k) => setTrendPeriod(k as '7d' | '30d')}
          />
        </div>
        <SensorFeedCard
          title={t('dashboard.liveSensorFeed', 'Live Sensor Feed')}
          subtitle={t('dashboard.sensorFeedSubtitle', 'Real-time gate and entry status')}
          sensors={sensorFeed}
        />
      </motion.div>

      {/* Active bookings list + Quick actions */}
      <motion.div variants={item} className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <div className="lg:col-span-2 card p-6">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <h2 className="text-lg font-semibold text-surface-900 dark:text-white" style={{ letterSpacing: '-0.02em' }}>
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
                  className="flex items-center gap-4 p-3 rounded-xl bg-surface-50/60 dark:bg-surface-800/40 transition-all hover:bg-surface-100/80 dark:hover:bg-surface-700/40 group"
                >
                  <div className="w-10 h-10 flex items-center justify-center rounded-lg bg-primary-500/10 text-primary-600 dark:text-primary-400">
                    <span className="text-sm font-bold" style={{ fontVariantNumeric: 'tabular-nums' }}>
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
        </div>

        <div className="card p-6">
          <h2 className="text-lg font-semibold text-surface-900 dark:text-white mb-4" style={{ letterSpacing: '-0.02em' }}>
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
                className={`flex items-center gap-3 px-4 py-3 text-sm font-medium rounded-xl transition-all group
                  ${action.accent
                    ? 'text-primary-700 dark:text-primary-300 bg-primary-500/10 hover:bg-primary-500/15'
                    : 'text-surface-700 dark:text-surface-300 bg-surface-500/5 hover:bg-surface-500/10'
                  }`}
              >
                <action.icon weight="regular" className={`w-4 h-4 transition-transform group-hover:scale-110 ${action.accent ? 'text-primary-500' : 'text-surface-400 dark:text-surface-500'}`} />
                <span className="flex-1">{action.label}</span>
                <ArrowRight weight="bold" className="w-3.5 h-3.5 text-surface-400 opacity-0 group-hover:opacity-100 transition-opacity" />
              </Link>
            ))}
          </div>
        </div>
      </motion.div>

      {/* Recent Activity table (Kinetic Observatory pattern) */}
      <motion.div variants={item}>
        <RecentActivityCard
          title={t('dashboard.recentActivity', 'Recent Activity')}
          rows={recentActivity}
          viewAllHref="/bookings"
          emptyText={t('dashboard.noActivity', 'No recent activity yet')}
          columnLabels={{
            vehicle: t('dashboard.colVehicleOwner', 'Vehicle / Location'),
            slot: t('dashboard.colSlot', 'Slot No.'),
            checkIn: t('dashboard.colCheckIn', 'Check-In Time'),
            duration: t('dashboard.colDuration', 'Duration'),
            status: t('dashboard.colStatus', 'Status'),
          }}
        />
      </motion.div>
    </motion.div>
    </AnimatePresence>
  );
}

