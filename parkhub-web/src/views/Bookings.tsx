import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  CalendarBlank, Clock, Car, X, SpinnerGap, CheckCircle, XCircle,
  ArrowClockwise, Warning, MapPin, CalendarPlus, Repeat, Timer,
  CalendarCheck, MagnifyingGlass, Funnel,
} from '@phosphor-icons/react';
import { api, type Booking, type Vehicle } from '../api/client';
import { BookingsSkeleton } from '../components/Skeleton';
import toast from 'react-hot-toast';
import { format, formatDistanceToNow, isFuture } from 'date-fns';
import { de, enUS } from 'date-fns/locale';

export function BookingsPage() {
  const { t, i18n } = useTranslation();
  const dateFnsLocale = i18n.language?.startsWith('de') ? de : enUS;
  const [bookings, setBookings] = useState<Booking[]>([]);
  const [vehicles, setVehicles] = useState<Vehicle[]>([]);
  const [loading, setLoading] = useState(true);
  const [cancelling, setCancelling] = useState<string | null>(null);
  const [filterStatus, setFilterStatus] = useState('all');
  const [searchLot, setSearchLot] = useState('');

  useEffect(() => {
    loadData();
  }, []);

  async function loadData() {
    const [bRes, vRes] = await Promise.all([api.getBookings(), api.getVehicles()]);
    if (bRes.success && bRes.data) setBookings(bRes.data);
    if (vRes.success && vRes.data) setVehicles(vRes.data);
    setLoading(false);
  }

  async function handleCancel(id: string) {
    setCancelling(id);
    const res = await api.cancelBooking(id);
    if (res.success) {
      setBookings(prev => prev.map(b => b.id === id ? { ...b, status: 'cancelled' } : b));
      toast.success(t('bookings.cancelled'));
    } else {
      toast.error(t('bookings.cancelFailed'));
    }
    setCancelling(null);
  }

  const filtered = bookings.filter(b => {
    if (filterStatus !== 'all' && b.status !== filterStatus) return false;
    if (searchLot && !b.lot_name.toLowerCase().includes(searchLot.toLowerCase())) return false;
    return true;
  });

  const isActiveOrConfirmed = (s: string) => s === 'active' || s === 'confirmed';
  const now = Date.now();
  const active = filtered.filter(b => isActiveOrConfirmed(b.status) && !isFuture(new Date(b.start_time)));
  const upcoming = filtered.filter(b => isActiveOrConfirmed(b.status) && isFuture(new Date(b.start_time)));
  const past = filtered.filter(b => b.status === 'completed' || b.status === 'cancelled');

  const container = { hidden: { opacity: 0 }, show: { opacity: 1, transition: { staggerChildren: 0.06 } } };
  const item = { hidden: { opacity: 0, y: 20 }, show: { opacity: 1, y: 0 } };

  if (loading) return <BookingsSkeleton />;

  return (
    <AnimatePresence mode="wait">
    <motion.div key="bookings-loaded" variants={container} initial="hidden" animate="show" className="space-y-8">
      <motion.div variants={item} className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white">{t('bookings.title')}</h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">{t('bookings.subtitle')}</p>
        </div>
        <button onClick={loadData} className="btn btn-secondary">
          <ArrowClockwise weight="bold" className="w-4 h-4" /> {t('common.refresh')}
        </button>
      </motion.div>

      {/* Filters */}
      <motion.div variants={item} className="card p-4">
        <div className="flex items-center gap-2 mb-3" aria-live="polite">
          <Funnel weight="bold" className="w-4 h-4 text-surface-500" />
          <span className="text-sm font-medium text-surface-700 dark:text-surface-300">{t('common.filter')}</span>
          <span className="ml-auto text-xs text-surface-400">{t('bookingFilters.totalCount', { count: filtered.length })}</span>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div className="relative">
            <MagnifyingGlass weight="regular" className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
            <input type="text" value={searchLot} onChange={e => setSearchLot(e.target.value)} placeholder={t('bookingFilters.searchLot')} className="input pl-9 text-sm" />
          </div>
          <select value={filterStatus} onChange={e => setFilterStatus(e.target.value)} className="input">
            <option value="all">{t('bookingFilters.statusAll')}</option>
            <option value="active">{t('bookingFilters.statusActive')}</option>
            <option value="confirmed">{t('bookingFilters.statusConfirmed')}</option>
            <option value="cancelled">{t('bookingFilters.statusCancelled')}</option>
            <option value="completed">{t('bookingFilters.statusCompleted')}</option>
          </select>
        </div>
      </motion.div>

      {/* Active */}
      <Section icon={Clock} title={t('bookings.active')} count={active.length} color="text-emerald-600">
        {active.length === 0 ? (
          <Empty icon={CalendarBlank} text={t('bookings.noActive')} showAction t={t} />
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            <AnimatePresence>
              {active.map(b => (
                <BookingCard key={b.id} booking={b} now={now} vehicles={vehicles}
                  onCancel={handleCancel} cancelling={cancelling} t={t} dateFnsLocale={dateFnsLocale} />
              ))}
            </AnimatePresence>
          </div>
        )}
      </Section>

      {/* Upcoming */}
      <Section icon={CalendarPlus} title={t('bookings.upcoming')} count={upcoming.length} color="text-primary-600">
        {upcoming.length === 0 ? (
          <Empty icon={CalendarCheck} text={t('bookings.noUpcoming')} t={t} />
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            <AnimatePresence>
              {upcoming.map(b => (
                <BookingCard key={b.id} booking={b} now={now} vehicles={vehicles}
                  onCancel={handleCancel} cancelling={cancelling} t={t} dateFnsLocale={dateFnsLocale} />
              ))}
            </AnimatePresence>
          </div>
        )}
      </Section>

      {/* Past */}
      <Section icon={CalendarBlank} title={t('bookings.past')} count={past.length} color="text-surface-400">
        {past.length === 0 ? (
          <Empty icon={CheckCircle} text={t('bookings.noPast')} t={t} />
        ) : (
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {past.map(b => (
              <BookingCard key={b.id} booking={b} now={now} vehicles={vehicles}
                onCancel={handleCancel} cancelling={cancelling} t={t} dateFnsLocale={dateFnsLocale} />
            ))}
          </div>
        )}
      </Section>
    </motion.div>
    </AnimatePresence>
  );
}

function Section({ icon: Icon, title, count, color, children }: any) {
  return (
    <section>
      <h2 className="text-lg font-semibold text-surface-900 dark:text-white mb-4 flex items-center gap-2">
        <Icon weight="fill" className={`w-5 h-5 ${color}`} />
        {title}
        <span className="badge badge-gray text-xs">{count}</span>
      </h2>
      {children}
    </section>
  );
}

function Empty({ icon: Icon, text, showAction, t }: any) {
  return (
    <div className="card p-12 text-center">
      <motion.div animate={{ y: [0, -4, 0] }} transition={{ duration: 2, repeat: Infinity, ease: 'easeInOut' }}>
        <Icon weight="light" className="w-20 h-20 text-surface-200 dark:text-surface-700 mx-auto" />
      </motion.div>
      <p className="text-surface-500 dark:text-surface-400 mb-4 mt-4">{text}</p>
      {showAction && (
        <motion.div className="inline-block" whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}>
          <Link to="/book" className="btn btn-primary"><CalendarPlus weight="bold" className="w-4 h-4" />{t('bookings.bookNow')}</Link>
        </motion.div>
      )}
    </div>
  );
}

function BookingCard({ booking, now, vehicles, onCancel, cancelling, t, dateFnsLocale }: any) {
  const isActiveOrConfirmed = booking.status === 'active' || booking.status === 'confirmed';
  const isPast = booking.status === 'completed' || booking.status === 'cancelled';
  const isExpiring = isActiveOrConfirmed && new Date(booking.end_time).getTime() - now < 30 * 60 * 1000;
  const isUpcoming = isActiveOrConfirmed && isFuture(new Date(booking.start_time));

  const statusCfg: Record<string, { label: string; cls: string }> = {
    active: { label: t('bookings.statusActive'), cls: 'badge-success' },
    confirmed: { label: t('bookings.statusActive'), cls: 'badge-success' },
    completed: { label: t('bookings.statusCompleted'), cls: 'badge-gray' },
    cancelled: { label: t('bookings.statusCancelled'), cls: 'badge-error' },
  };
  const cfg = statusCfg[booking.status] || statusCfg.active;

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, x: -100 }}
      className={`card p-5 border-l-4 transition-all hover:shadow-md ${
        isPast ? 'border-l-surface-300 dark:border-l-surface-600 opacity-80' :
        isExpiring ? 'border-l-accent-500' : 'border-l-primary-500'
      }`}
    >
      <div className="flex items-start justify-between mb-3">
        <div className="flex items-center gap-3">
          <div className={`w-12 h-12 rounded-xl flex items-center justify-center ${
            isPast ? 'bg-surface-100 dark:bg-surface-800' : isExpiring ? 'bg-accent-100 dark:bg-accent-900/30' : 'bg-primary-100 dark:bg-primary-900/30'
          }`}>
            <span className={`text-lg font-bold ${
              isPast ? 'text-surface-500' : isExpiring ? 'text-accent-600 dark:text-accent-400' : 'text-primary-600 dark:text-primary-400'
            }`}>{booking.slot_number}</span>
          </div>
          <div>
            <p className="font-semibold text-surface-900 dark:text-white">{booking.lot_name}</p>
            <div className="flex items-center gap-1.5 text-sm text-surface-500 dark:text-surface-400">
              <MapPin weight="regular" className="w-3.5 h-3.5" /> {t('dashboard.slot')} {booking.slot_number}
            </div>
          </div>
        </div>
        <span className={`badge ${cfg.cls}`}>{cfg.label}</span>
      </div>

      <div className="flex items-center gap-4 text-sm text-surface-600 dark:text-surface-400 mb-3">
        {booking.vehicle_plate && (
          <span className="flex items-center gap-1"><Car weight="regular" className="w-4 h-4" /> {booking.vehicle_plate}</span>
        )}
        <span className="flex items-center gap-1">
          <Timer weight="regular" className="w-4 h-4" />
          {format(new Date(booking.start_time), 'HH:mm')} — {format(new Date(booking.end_time), 'HH:mm')}
        </span>
      </div>

      <div className="flex items-center justify-between pt-3 border-t border-surface-100 dark:border-surface-800">
        <p className={`text-sm ${isExpiring ? 'text-accent-600 dark:text-accent-400 font-medium' : 'text-surface-500 dark:text-surface-400'}`}>
          {isExpiring && <Warning weight="fill" className="w-3.5 h-3.5 inline mr-1" />}
          {isUpcoming
            ? t('bookings.startsIn', { time: formatDistanceToNow(new Date(booking.start_time), { addSuffix: true, locale: dateFnsLocale }) })
            : isPast
            ? format(new Date(booking.start_time), 'd. MMMM yyyy', { locale: dateFnsLocale })
            : t('bookings.endsIn', { time: formatDistanceToNow(new Date(booking.end_time), { addSuffix: true, locale: dateFnsLocale }) })
          }
        </p>
        {isActiveOrConfirmed && (
          <button
            onClick={() => onCancel(booking.id)}
            disabled={cancelling === booking.id}
            className="btn btn-sm btn-ghost text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
          >
            {cancelling === booking.id
              ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" />
              : <><X weight="bold" className="w-4 h-4" /> {t('bookings.cancelBtn')}</>
            }
          </button>
        )}
      </div>
    </motion.div>
  );
}
