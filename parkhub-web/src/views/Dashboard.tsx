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

  const container = { hidden: { opacity: 0 }, show: { opacity: 1, transition: { staggerChildren: 0.06 } } };
  const item = { hidden: { opacity: 0, y: 12 }, show: { opacity: 1, y: 0, transition: { ease: [0.22, 1, 0.36, 1] } } };

  if (loading) return (
    <div className="space-y-6">
      <div className="h-8 w-72 skeleton" />
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
        {[1,2,3,4].map(i => <div key={i} className="h-24 skeleton" />)}
      </div>
      <div className="h-64 skeleton" />
    </div>
  );

  return (
    <motion.div variants={container} initial="hidden" animate="show" className="space-y-6">
      {/* Greeting */}
      <motion.div variants={item}>
        <p className="text-xs font-semibold text-accent-600 dark:text-accent-400 uppercase tracking-widest mb-1">
          {t('nav.dashboard')}
        </p>
        <h1 className="text-2xl sm:text-3xl font-bold text-surface-900 dark:text-white tracking-tight">
          {t('dashboard.greeting', { timeOfDay, name })}
        </h1>
      </motion.div>

      {/* Stats grid */}
      <motion.div variants={item} className="grid grid-cols-2 lg:grid-cols-4 gap-3">
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
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        {/* Active bookings */}
        <motion.div variants={item} className="lg:col-span-2 card p-5">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-sm font-semibold text-surface-900 dark:text-white flex items-center gap-2 uppercase tracking-wide">
              <CalendarCheck weight="bold" className="w-4 h-4 text-accent-500" />
              {t('dashboard.activeBookings')}
            </h2>
            <Link to="/bookings" className="text-xs text-accent-600 hover:text-accent-500 font-semibold flex items-center gap-1 cursor-pointer uppercase tracking-wide">
              {t('nav.bookings')} <ArrowRight weight="bold" className="w-3 h-3" />
            </Link>
          </div>

          {activeBookings.length === 0 ? (
            <div className="text-center py-10">
              <CalendarPlus weight="light" className="w-14 h-14 text-surface-200 dark:text-surface-700 mx-auto mb-3" />
              <p className="text-surface-500 dark:text-surface-400 mb-4 text-sm">{t('dashboard.noActiveBookings')}</p>
              <Link to="/book" className="btn btn-primary cursor-pointer">{t('dashboard.bookSpot')}</Link>
            </div>
          ) : (
            <div className="space-y-2">
              {activeBookings.slice(0, 5).map(b => (
                <div key={b.id} className="flex items-center gap-4 p-3 bg-surface-50 dark:bg-surface-800/40 rounded-md hover:bg-surface-100 dark:hover:bg-surface-800/70 transition-colors border border-transparent hover:border-surface-200 dark:hover:border-surface-700">
                  <div className="w-10 h-10 bg-accent-100 dark:bg-accent-900/20 flex items-center justify-center border border-accent-200 dark:border-accent-800/40">
                    <span className="text-sm font-bold text-accent-700 dark:text-accent-400 font-[Outfit]">{b.slot_number}</span>
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="font-medium text-surface-900 dark:text-white text-sm truncate">{b.lot_name}</p>
                    <div className="flex items-center gap-2 text-xs text-surface-500 dark:text-surface-400">
                      <MapPin weight="regular" className="w-3 h-3" />
                      {t('dashboard.slot')} {b.slot_number}
                      {b.vehicle_plate && <><span className="mx-1">&middot;</span><Car weight="regular" className="w-3 h-3" />{b.vehicle_plate}</>}
                    </div>
                  </div>
                  <span className="badge badge-success">{t('bookings.statusActive')}</span>
                </div>
              ))}
            </div>
          )}
        </motion.div>

        {/* Quick actions */}
        <motion.div variants={item} className="card p-5">
          <h2 className="text-sm font-semibold text-surface-900 dark:text-white mb-4 uppercase tracking-wide">
            {t('dashboard.quickActions')}
          </h2>
          <div className="space-y-1">
            {[
              { to: '/book', icon: CalendarPlus, label: t('dashboard.bookSpot'), accent: 'bg-accent-100 dark:bg-accent-900/20 text-accent-600 dark:text-accent-400' },
              { to: '/vehicles', icon: Car, label: t('dashboard.myVehicles'), accent: 'bg-primary-100 dark:bg-primary-900/20 text-primary-600 dark:text-primary-400' },
              { to: '/bookings', icon: CalendarCheck, label: t('dashboard.viewBookings'), accent: 'bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400' },
              { to: '/credits', icon: CoinVertical, label: t('nav.credits'), accent: 'bg-emerald-50 dark:bg-emerald-900/20 text-emerald-600 dark:text-emerald-400' },
            ].map((action, i) => (
              <Link
                key={i}
                to={action.to}
                className="flex items-center gap-3 p-2.5 rounded-md hover:bg-surface-50 dark:hover:bg-surface-800/50 transition-colors group cursor-pointer border border-transparent hover:border-surface-200 dark:hover:border-surface-800"
              >
                <div className={`w-9 h-9 flex items-center justify-center ${action.accent}`}>
                  <action.icon weight="bold" className="w-4 h-4" />
                </div>
                <span className="font-medium text-surface-700 dark:text-surface-300 group-hover:text-surface-900 dark:group-hover:text-white transition-colors text-sm">
                  {action.label}
                </span>
                <ArrowRight weight="bold" className="w-3.5 h-3.5 text-surface-400 ml-auto opacity-0 group-hover:opacity-100 transition-opacity" />
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
  const colors: Record<string, { bg: string; text: string; icon: string }> = {
    accent: { bg: 'bg-accent-100 dark:bg-accent-900/20', text: 'text-accent-700 dark:text-accent-400', icon: 'text-accent-600 dark:text-accent-400' },
    primary: { bg: 'bg-primary-100 dark:bg-primary-900/20', text: 'text-primary-700 dark:text-primary-400', icon: 'text-primary-600 dark:text-primary-400' },
    info: { bg: 'bg-blue-50 dark:bg-blue-900/20', text: 'text-blue-700 dark:text-blue-400', icon: 'text-blue-600 dark:text-blue-400' },
    success: { bg: 'bg-emerald-50 dark:bg-emerald-900/20', text: 'text-emerald-700 dark:text-emerald-400', icon: 'text-emerald-600 dark:text-emerald-400' },
  };

  const c = colors[color] || colors.primary;

  return (
    <div className="stat-card">
      <div className="flex items-start justify-between">
        <div>
          <p className="text-[11px] font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider mb-2">{label}</p>
          {isText ? (
            <p className="text-lg font-bold text-surface-900 dark:text-white">{value}</p>
          ) : (
            <p className={`stat-value ${c.text}`}>{value}</p>
          )}
        </div>
        <div className={`w-9 h-9 flex items-center justify-center ${c.bg}`}>
          <Icon weight="bold" className={`w-4 h-4 ${c.icon}`} />
        </div>
      </div>
    </div>
  );
}

function formatTime(dateStr: string) {
  const d = new Date(dateStr);
  return d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
}
