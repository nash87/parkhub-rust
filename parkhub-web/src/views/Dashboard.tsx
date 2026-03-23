import { useEffect, useState, useCallback, useMemo } from 'react';
import { Link } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';
import {
  CalendarCheck, Car, Coins, Clock, CalendarPlus, ArrowRight,
  TrendUp, MapPin,
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

  // Build a 7-day booking activity chart from real booking data
  const weeklyActivity = useMemo(() => {
    const days: { label: string; value: number }[] = [];
    const now = new Date();
    for (let i = 6; i >= 0; i--) {
      const d = new Date(now);
      d.setDate(d.getDate() - i);
      const dayStr = d.toLocaleDateString(undefined, { weekday: 'short' });
      const dateKey = d.toISOString().slice(0, 10);
      const count = bookings.filter(b => b.start_time.slice(0, 10) === dateKey).length;
      days.push({ label: dayStr, value: count });
    }
    return days;
  }, [bookings]);

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

      {/* Bento stats grid — asymmetric layout */}
      <motion.div
        variants={item}
        className="grid grid-cols-2 lg:grid-cols-4 gap-3"
        role="region"
        aria-label={t('dashboard.statistics')}
      >
        {/* Active Bookings — gradient accent */}
        <BentoStatCard
          label={t('dashboard.activeBookings')}
          value={activeBookings.length}
          gradient="from-primary-500 to-primary-300"
          live={activeBookings.length > 0}
        />

        {/* Credits Left */}
        <BentoStatCard
          label={t('dashboard.creditsLeft')}
          value={user?.credits_balance ?? 0}
          gradient="from-accent-500 to-accent-300"
        />

        {/* This Month */}
        <BentoStatCard
          label={t('dashboard.thisMonth')}
          value={stats?.bookings_this_month ?? 0}
          gradient="from-blue-500 to-cyan-400"
        />

        {/* Next Booking — highlighted */}
        <NextBookingCard
          label={t('dashboard.nextBooking')}
          value={activeBookings.length > 0 ? formatTime(activeBookings[0].start_time) : '—'}
        />
      </motion.div>

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

      {/* Booking Activity (last 7 days) */}
      <motion.div
        variants={item}
        className="glass-card p-6"
      >
        <div className="flex items-center gap-2 mb-4">
          <TrendUp weight="bold" className="w-4 h-4 text-primary-500" />
          <h2 className="text-lg font-semibold text-surface-900 dark:text-white tracking-tight">
            {t('dashboard.thisMonth')}
          </h2>
        </div>
        {weeklyActivity.some(d => d.value > 0) ? (
          <BarChart data={weeklyActivity} color="var(--color-teal-500, #14b8a6)" />
        ) : (
          <p className="text-sm text-surface-500 dark:text-surface-400 py-6 text-center">
            {t('dashboard.noActiveBookings')}
          </p>
        )}
      </motion.div>
    </motion.div>
    </AnimatePresence>
  );
}

/* ── Bento Stat Card with animated counter and gradient accent ──── */

function BentoStatCard({ label, value, gradient, live }: {
  label: string;
  value: number | string;
  gradient?: string;
  live?: boolean;
}) {
  const isNum = typeof value === 'number';

  return (
    <motion.div
      whileHover={{ scale: 1.02, y: -2 }}
      transition={{ type: 'spring', stiffness: 400, damping: 25 }}
      className="relative overflow-hidden glass-card p-4 group"
    >
      {/* Subtle gradient accent in top-right */}
      {gradient && (
        <div className={`absolute -top-6 -right-6 w-16 h-16 rounded-full bg-gradient-to-br ${gradient} opacity-20 group-hover:opacity-30 transition-opacity blur-xl`} />
      )}

      <div className="relative">
        <div className="flex items-center gap-1.5">
          <p className="text-sm font-medium text-surface-500 dark:text-surface-400">{label}</p>
          {live && <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 pulse-dot" />}
        </div>
        <p
          className="mt-2 text-2xl font-bold text-surface-900 dark:text-white"
          style={{ fontVariantNumeric: 'tabular-nums', letterSpacing: '-0.03em', lineHeight: 1 }}
        >
          {isNum ? <AnimatedCounter value={value as number} duration={800} /> : value}
        </p>
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
