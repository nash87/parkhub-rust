import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  CalendarCheck, Car, CoinVertical, Clock, CalendarPlus, ArrowRight,
  TrendUp, MapPin,
} from '@phosphor-icons/react';
import { useAuth } from '../context/AuthContext';
import { api, type Booking, type UserStats } from '../api/client';

export function DashboardPage() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const [bookings, setBookings] = useState<Booking[]>([]);
  const [stats, setStats] = useState<UserStats | null>(null);
  const [loading, setLoading] = useState(true);

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

  const container = { hidden: { opacity: 0 }, show: { opacity: 1, transition: { staggerChildren: 0.08 } } };
  const item = { hidden: { opacity: 0, y: 16 }, show: { opacity: 1, y: 0 } };

  if (loading) return (
    <div className="space-y-6">
      <div className="h-10 w-72 skeleton rounded-xl" />
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {[1,2,3,4].map(i => <div key={i} className="h-28 skeleton rounded-2xl" />)}
      </div>
      <div className="h-64 skeleton rounded-2xl" />
    </div>
  );

  return (
    <motion.div variants={container} initial="hidden" animate="show" className="space-y-8">
      {/* Greeting */}
      <motion.div variants={item}>
        <h1 className="text-2xl sm:text-3xl font-bold text-surface-900 dark:text-white">
          {t('dashboard.greeting', { timeOfDay, name })}
        </h1>
      </motion.div>

      {/* Stats grid */}
      <motion.div variants={item} className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          icon={CalendarCheck}
          label={t('dashboard.activeBookings')}
          value={activeBookings.length}
          color="accent"
        />
        <StatCard
          icon={CoinVertical}
          label={t('dashboard.creditsLeft')}
          value={user?.credits_balance ?? 0}
          color="primary"
        />
        <StatCard
          icon={TrendUp}
          label={t('dashboard.thisMonth')}
          value={stats?.bookings_this_month ?? 0}
          color="info"
        />
        <StatCard
          icon={Clock}
          label={t('dashboard.nextBooking')}
          value={activeBookings.length > 0 ? formatTime(activeBookings[0].start_time) : '\u2014'}
          color="success"
          isText
        />
      </motion.div>

      {/* Active bookings + Quick actions */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Active bookings */}
        <motion.div variants={item} className="lg:col-span-2 card p-6">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-surface-900 dark:text-white flex items-center gap-2">
              <CalendarCheck weight="bold" className="w-5 h-5 text-accent-500" />
              {t('dashboard.activeBookings')}
            </h2>
            <Link to="/bookings" className="text-sm text-accent-600 hover:text-accent-500 font-medium flex items-center gap-1 cursor-pointer">
              {t('nav.bookings')} <ArrowRight weight="bold" className="w-3.5 h-3.5" />
            </Link>
          </div>

          {activeBookings.length === 0 ? (
            <div className="text-center py-12">
              <CalendarPlus weight="light" className="w-16 h-16 text-surface-200 dark:text-surface-700 mx-auto mb-3" />
              <p className="text-surface-500 dark:text-surface-400 mb-4">{t('dashboard.noActiveBookings')}</p>
              <Link to="/book" className="btn btn-primary cursor-pointer">{t('dashboard.bookSpot')}</Link>
            </div>
          ) : (
            <div className="space-y-3">
              {activeBookings.slice(0, 5).map(b => (
                <div key={b.id} className="flex items-center gap-4 p-4 bg-surface-50 dark:bg-surface-800/50 rounded-xl hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors">
                  <div className="w-12 h-12 rounded-xl bg-accent-50 dark:bg-accent-900/20 flex items-center justify-center border border-accent-200 dark:border-accent-800/40">
                    <span className="text-lg font-bold text-accent-700 dark:text-accent-400 font-[Outfit]">{b.slot_number}</span>
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="font-medium text-surface-900 dark:text-white truncate">{b.lot_name}</p>
                    <div className="flex items-center gap-2 text-sm text-surface-500 dark:text-surface-400">
                      <MapPin weight="regular" className="w-3.5 h-3.5" />
                      {t('dashboard.slot')} {b.slot_number}
                      {b.vehicle_plate && <><span className="mx-1">&middot;</span><Car weight="regular" className="w-3.5 h-3.5" />{b.vehicle_plate}</>}
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
        <motion.div variants={item} className="card p-6">
          <h2 className="text-lg font-semibold text-surface-900 dark:text-white mb-4">
            {t('dashboard.quickActions')}
          </h2>
          <div className="space-y-2">
            {[
              { to: '/book', icon: CalendarPlus, label: t('dashboard.bookSpot'), accent: 'bg-accent-50 dark:bg-accent-900/20 text-accent-600 dark:text-accent-400' },
              { to: '/vehicles', icon: Car, label: t('dashboard.myVehicles'), accent: 'bg-primary-100 dark:bg-primary-900/20 text-primary-600 dark:text-primary-400' },
              { to: '/bookings', icon: CalendarCheck, label: t('dashboard.viewBookings'), accent: 'bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400' },
              { to: '/credits', icon: CoinVertical, label: t('nav.credits'), accent: 'bg-emerald-50 dark:bg-emerald-900/20 text-emerald-600 dark:text-emerald-400' },
            ].map((action, i) => (
              <Link
                key={i}
                to={action.to}
                className="flex items-center gap-3 p-3 rounded-xl hover:bg-surface-50 dark:hover:bg-surface-800/50 transition-colors group cursor-pointer"
              >
                <div className={`w-10 h-10 rounded-xl flex items-center justify-center ${action.accent}`}>
                  <action.icon weight="bold" className="w-5 h-5" />
                </div>
                <span className="font-medium text-surface-700 dark:text-surface-300 group-hover:text-surface-900 dark:group-hover:text-white transition-colors">
                  {action.label}
                </span>
                <ArrowRight weight="bold" className="w-4 h-4 text-surface-400 ml-auto opacity-0 group-hover:opacity-100 transition-opacity" />
              </Link>
            ))}
          </div>
        </motion.div>
      </div>
    </motion.div>
  );
}

function StatCard({ icon: Icon, label, value, color, isText }: {
  icon: React.ElementType; label: string; value: number | string; color: string; isText?: boolean;
}) {
  const colors: Record<string, { bg: string; text: string }> = {
    accent: { bg: 'bg-accent-50 dark:bg-accent-900/20', text: 'text-accent-600 dark:text-accent-400' },
    primary: { bg: 'bg-primary-100 dark:bg-primary-900/20', text: 'text-primary-600 dark:text-primary-400' },
    info: { bg: 'bg-blue-50 dark:bg-blue-900/20', text: 'text-blue-600 dark:text-blue-400' },
    success: { bg: 'bg-emerald-50 dark:bg-emerald-900/20', text: 'text-emerald-600 dark:text-emerald-400' },
  };

  const c = colors[color] || colors.primary;

  return (
    <div className="stat-card">
      <div className="flex items-start justify-between">
        <div>
          <p className="text-sm font-medium text-surface-500 dark:text-surface-400">{label}</p>
          {isText ? (
            <p className="mt-2 text-lg font-bold text-surface-900 dark:text-white">{value}</p>
          ) : (
            <p className={`mt-2 stat-value ${c.text}`}>{value}</p>
          )}
        </div>
        <div className={`w-10 h-10 rounded-xl flex items-center justify-center ${c.bg} ${c.text}`}>
          <Icon weight="bold" className="w-5 h-5" />
        </div>
      </div>
    </div>
  );
}

function formatTime(dateStr: string) {
  const d = new Date(dateStr);
  return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
}
