import { useEffect, useState, useCallback } from 'react';
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

  const container = staggerSlow;
  const item = fadeUp;

  if (loading) return <div role="status" aria-label={t('dashboard.loadingDashboard')}><DashboardSkeleton /></div>;

  return (
    <AnimatePresence mode="wait">
    <motion.div key="dashboard-loaded" variants={container} initial="hidden" animate="show" className="space-y-8">
      {/* Greeting */}
      <motion.div variants={item}>
        <h1 className="text-2xl sm:text-3xl font-bold text-surface-900 dark:text-white">
          {t('dashboard.greeting', { timeOfDay, name })}
        </h1>
      </motion.div>

      {/* Stats grid */}
      <motion.div variants={item} className="grid grid-cols-2 lg:grid-cols-4 gap-4" role="region" aria-label={t('dashboard.statistics')}>
        <StatCard
          label={t('dashboard.activeBookings')}
          value={activeBookings.length}
        />
        <StatCard
          label={t('dashboard.creditsLeft')}
          value={user?.credits_balance ?? 0}
        />
        <StatCard
          label={t('dashboard.thisMonth')}
          value={stats?.bookings_this_month ?? 0}
        />
        <NextBookingCard
          label={t('dashboard.nextBooking')}
          value={activeBookings.length > 0 ? formatTime(activeBookings[0].start_time) : '—'}
        />
      </motion.div>

      {/* Active bookings + Quick actions */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Active bookings */}
        <motion.div
          variants={item}
          className="lg:col-span-2 bg-white dark:bg-surface-800 border border-surface-200 dark:border-surface-700 p-6"
          style={{ borderRadius: 12 }}
        >
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-surface-900 dark:text-white">
              {t('dashboard.activeBookings')}
            </h2>
            <Link to="/bookings" className="text-sm text-teal-600 hover:text-teal-500 dark:text-teal-400 font-medium flex items-center gap-1">
              {t('nav.bookings')} <ArrowRight weight="bold" className="w-3.5 h-3.5" />
            </Link>
          </div>

          {activeBookings.length === 0 ? (
            <div className="py-12">
              <p className="text-surface-500 dark:text-surface-400 mb-4">{t('dashboard.noActiveBookings')}</p>
              <Link to="/book" className="btn btn-primary">{t('dashboard.bookSpot')}</Link>
            </div>
          ) : (
            <div className="space-y-2">
              {activeBookings.slice(0, 5).map(b => (
                <div
                  key={b.id}
                  className="flex items-center gap-4 p-3 bg-surface-50 dark:bg-surface-800/50 border border-surface-100 dark:border-surface-700 transition-colors hover:bg-surface-100 dark:hover:bg-surface-700/50"
                  style={{ borderRadius: 8 }}
                >
                  <div
                    className="w-10 h-10 flex items-center justify-center bg-surface-100 dark:bg-surface-700 border border-surface-200 dark:border-surface-600"
                    style={{ borderRadius: 8 }}
                  >
                    <span className="text-sm font-bold text-surface-700 dark:text-surface-200" style={{ fontVariantNumeric: 'tabular-nums' }}>
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
                </div>
              ))}
            </div>
          )}
        </motion.div>

        {/* Quick actions */}
        <motion.div
          variants={item}
          className="bg-white dark:bg-surface-800 border border-surface-200 dark:border-surface-700 p-6"
          style={{ borderRadius: 12 }}
        >
          <h2 className="text-lg font-semibold text-surface-900 dark:text-white mb-4">
            {t('dashboard.quickActions')}
          </h2>
          <div className="space-y-2">
            {[
              { to: '/book', icon: CalendarPlus, label: t('dashboard.bookSpot') },
              { to: '/vehicles', icon: Car, label: t('dashboard.myVehicles') },
              { to: '/bookings', icon: CalendarCheck, label: t('dashboard.viewBookings') },
              { to: '/credits', icon: Coins, label: t('nav.credits') },
            ].map((action) => (
              <Link
                key={action.to}
                to={action.to}
                className="flex items-center gap-3 px-3 py-2.5 text-sm font-medium text-surface-700 dark:text-surface-300 border border-surface-200 dark:border-surface-600 bg-white dark:bg-surface-800 hover:bg-surface-50 dark:hover:bg-surface-700/50 hover:border-surface-300 dark:hover:border-surface-500 transition-colors"
                style={{ borderRadius: 8 }}
              >
                <action.icon weight="regular" className="w-4 h-4 text-surface-500 dark:text-surface-400" />
                <span className="flex-1">{action.label}</span>
                <ArrowRight weight="bold" className="w-3.5 h-3.5 text-surface-400" />
              </Link>
            ))}
          </div>
        </motion.div>
      </div>
    </motion.div>
    </AnimatePresence>
  );
}

function StatCard({ label, value }: {
  label: string; value: number | string;
}) {
  return (
    <div
      className="bg-white dark:bg-surface-800 border border-surface-200 dark:border-surface-700 p-4"
      style={{ borderRadius: 12 }}
    >
      <p className="text-sm font-medium text-surface-500 dark:text-surface-400">{label}</p>
      <p
        className="mt-2 text-2xl font-bold text-surface-900 dark:text-white"
        style={{ fontVariantNumeric: 'tabular-nums', letterSpacing: '-0.02em', lineHeight: 1 }}
      >
        {value}
      </p>
    </div>
  );
}

function NextBookingCard({ label, value }: {
  label: string; value: string;
}) {
  return (
    <div
      className="bg-white dark:bg-surface-800 border border-surface-200 dark:border-surface-700 p-4"
      style={{ borderRadius: 12, borderLeft: '3px solid #0d9488' }}
    >
      <p className="text-sm font-medium text-surface-500 dark:text-surface-400">{label}</p>
      <p
        className="mt-2 text-2xl font-bold text-surface-900 dark:text-white"
        style={{ fontVariantNumeric: 'tabular-nums', letterSpacing: '-0.02em', lineHeight: 1 }}
      >
        {value}
      </p>
    </div>
  );
}

function formatTime(dateStr: string) {
  const d = new Date(dateStr);
  return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
}
