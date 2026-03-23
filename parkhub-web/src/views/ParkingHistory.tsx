import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  ClockCounterClockwise, Star, Clock, TrendUp, CalendarBlank,
  FunnelSimple, CaretLeft, CaretRight, SpinnerGap, Coins,
} from '@phosphor-icons/react';
import { api, type Booking, type ParkingLot, type PersonalParkingStats } from '../api/client';
import { useTranslation } from 'react-i18next';
import { stagger, fadeUp } from '../constants/animations';
import { OnboardingHint } from '../components/OnboardingHint';

export function ParkingHistoryPage() {
  const { t } = useTranslation();
  const [bookings, setBookings] = useState<Booking[]>([]);
  const [stats, setStats] = useState<PersonalParkingStats | null>(null);
  const [lots, setLots] = useState<ParkingLot[]>([]);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [total, setTotal] = useState(0);

  // Filters
  const [filterLot, setFilterLot] = useState('');
  const [filterFrom, setFilterFrom] = useState('');
  const [filterTo, setFilterTo] = useState('');

  useEffect(() => {
    api.getLots().then(res => {
      if (res.success && res.data) setLots(res.data);
    });
    api.getBookingStats().then(res => {
      if (res.success && res.data) setStats(res.data);
    });
  }, []);

  useEffect(() => {
    setLoading(true);
    api.getBookingHistory({
      lot_id: filterLot || undefined,
      from: filterFrom ? new Date(filterFrom).toISOString() : undefined,
      to: filterTo ? new Date(filterTo + 'T23:59:59').toISOString() : undefined,
      page,
      per_page: 10,
    }).then(res => {
      if (res.success && res.data) {
        setBookings(res.data.items);
        setTotalPages(res.data.total_pages);
        setTotal(res.data.total);
      }
    }).finally(() => setLoading(false));
  }, [page, filterLot, filterFrom, filterTo]);

  const container = stagger;
  const item = fadeUp;

  const dayNames: Record<string, string> = {
    Monday: t('history.monday'),
    Tuesday: t('history.tuesday'),
    Wednesday: t('history.wednesday'),
    Thursday: t('history.thursday'),
    Friday: t('history.friday'),
    Saturday: t('history.saturday'),
    Sunday: t('history.sunday'),
  };

  const maxTrend = stats?.monthly_trend ? Math.max(...stats.monthly_trend.map(m => m.bookings), 1) : 1;

  return (
    <AnimatePresence mode="wait">
      <motion.div key="history" variants={container} initial="hidden" animate="show" className="space-y-8">
        {/* Header */}
        <motion.div variants={item} className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-xl bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center">
              <ClockCounterClockwise weight="fill" className="w-5 h-5 text-primary-600 dark:text-primary-400" />
            </div>
            <div>
              <h1 className="text-2xl font-bold text-surface-900 dark:text-white">{t('history.title')}</h1>
              <p className="text-surface-500 dark:text-surface-400 mt-0.5">{t('history.subtitle')}</p>
            </div>
          </div>
          <OnboardingHint hintKey="history" text={t('history.help')} />
        </motion.div>

        {/* Stats Cards */}
        {stats && (
          <motion.div variants={item} className="grid grid-cols-2 sm:grid-cols-4 gap-4">
            <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-4">
              <div className="flex items-center gap-2 text-surface-500 dark:text-surface-400 mb-1">
                <CalendarBlank weight="regular" className="w-4 h-4" />
                <span className="text-xs font-medium">{t('history.totalBookings')}</span>
              </div>
              <p className="text-2xl font-bold text-surface-900 dark:text-white">{stats.total_bookings}</p>
            </div>
            <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-4">
              <div className="flex items-center gap-2 text-surface-500 dark:text-surface-400 mb-1">
                <Star weight="regular" className="w-4 h-4" />
                <span className="text-xs font-medium">{t('history.favoriteLot')}</span>
              </div>
              <p className="text-lg font-bold text-surface-900 dark:text-white truncate">{stats.favorite_lot || '—'}</p>
            </div>
            <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-4">
              <div className="flex items-center gap-2 text-surface-500 dark:text-surface-400 mb-1">
                <Clock weight="regular" className="w-4 h-4" />
                <span className="text-xs font-medium">{t('history.avgDuration')}</span>
              </div>
              <p className="text-2xl font-bold text-surface-900 dark:text-white">{Math.round(stats.avg_duration_minutes)}<span className="text-sm font-normal text-surface-500"> min</span></p>
            </div>
            <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-4">
              <div className="flex items-center gap-2 text-surface-500 dark:text-surface-400 mb-1">
                <Coins weight="regular" className="w-4 h-4" />
                <span className="text-xs font-medium">{t('history.creditsSpent')}</span>
              </div>
              <p className="text-2xl font-bold text-surface-900 dark:text-white">{stats.credits_spent}</p>
            </div>
          </motion.div>
        )}

        {/* Monthly Trend + Busiest Day */}
        {stats && (
          <motion.div variants={item} className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {/* Monthly Trend */}
            <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-5">
              <div className="flex items-center gap-2 mb-4">
                <TrendUp weight="regular" className="w-4 h-4 text-primary-600 dark:text-primary-400" />
                <h3 className="text-sm font-semibold text-surface-900 dark:text-white">{t('history.monthlyTrend')}</h3>
              </div>
              <div className="flex items-end gap-2 h-32">
                {stats.monthly_trend.map((m) => (
                  <div key={m.month} className="flex-1 flex flex-col items-center gap-1">
                    <span className="text-xs font-medium text-surface-900 dark:text-white">{m.bookings}</span>
                    <div
                      className="w-full bg-primary-500 dark:bg-primary-400 rounded-t transition-all"
                      style={{ height: `${Math.max((m.bookings / maxTrend) * 100, 4)}%` }}
                    />
                    <span className="text-[10px] text-surface-400 dark:text-surface-500">{m.month.slice(5)}</span>
                  </div>
                ))}
              </div>
            </div>

            {/* Busiest Day */}
            <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-5">
              <div className="flex items-center gap-2 mb-4">
                <CalendarBlank weight="regular" className="w-4 h-4 text-primary-600 dark:text-primary-400" />
                <h3 className="text-sm font-semibold text-surface-900 dark:text-white">{t('history.busiestDay')}</h3>
              </div>
              <div className="flex items-center justify-center h-32">
                <div className="text-center">
                  <p className="text-4xl font-bold text-primary-600 dark:text-primary-400">
                    {stats.busiest_day ? dayNames[stats.busiest_day] || stats.busiest_day : '—'}
                  </p>
                  <p className="text-sm text-surface-500 dark:text-surface-400 mt-2">{t('history.busiestDayDesc')}</p>
                </div>
              </div>
            </div>
          </motion.div>
        )}

        {/* Filters */}
        <motion.div variants={item} className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-4">
          <div className="flex items-center gap-2 mb-3">
            <FunnelSimple weight="regular" className="w-4 h-4 text-surface-500" />
            <span className="text-sm font-medium text-surface-700 dark:text-surface-300">{t('history.filters')}</span>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
            <select
              value={filterLot}
              onChange={e => { setFilterLot(e.target.value); setPage(1); }}
              className="rounded-lg border border-surface-200 dark:border-surface-700 bg-surface-50 dark:bg-surface-800 text-sm px-3 py-2 text-surface-900 dark:text-white"
              aria-label={t('history.filterLot')}
            >
              <option value="">{t('history.allLots')}</option>
              {lots.map(l => <option key={l.id} value={l.id}>{l.name}</option>)}
            </select>
            <input
              type="date"
              value={filterFrom}
              onChange={e => { setFilterFrom(e.target.value); setPage(1); }}
              className="rounded-lg border border-surface-200 dark:border-surface-700 bg-surface-50 dark:bg-surface-800 text-sm px-3 py-2 text-surface-900 dark:text-white"
              aria-label={t('history.dateFrom')}
            />
            <input
              type="date"
              value={filterTo}
              onChange={e => { setFilterTo(e.target.value); setPage(1); }}
              className="rounded-lg border border-surface-200 dark:border-surface-700 bg-surface-50 dark:bg-surface-800 text-sm px-3 py-2 text-surface-900 dark:text-white"
              aria-label={t('history.dateTo')}
            />
          </div>
        </motion.div>

        {/* Timeline */}
        <motion.div variants={item}>
          {loading ? (
            <div className="flex items-center justify-center py-16">
              <SpinnerGap weight="bold" className="w-8 h-8 animate-spin text-primary-500" />
            </div>
          ) : bookings.length === 0 ? (
            <div className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-16 text-center">
              <ClockCounterClockwise weight="light" className="w-20 h-20 text-surface-200 dark:text-surface-700 mx-auto" />
              <p className="text-surface-500 dark:text-surface-400 mt-4">{t('history.noHistory')}</p>
            </div>
          ) : (
            <div className="space-y-3">
              {bookings.map((b) => (
                <div
                  key={b.id}
                  className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-4 flex items-center gap-4"
                >
                  <div className={`w-2 h-2 rounded-full flex-shrink-0 ${
                    b.status === 'completed' ? 'bg-green-500' :
                    b.status === 'cancelled' ? 'bg-red-500' :
                    'bg-surface-400'
                  }`} />
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <p className="font-medium text-surface-900 dark:text-white truncate">{b.lot_name}</p>
                      <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${
                        b.status === 'completed'
                          ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
                          : 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400'
                      }`}>
                        {t(`history.status.${b.status}`)}
                      </span>
                    </div>
                    <p className="text-sm text-surface-500 dark:text-surface-400 mt-0.5">
                      {t('history.slot')} {b.slot_number} &middot; {new Date(b.start_time).toLocaleDateString()} {new Date(b.start_time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })} — {new Date(b.end_time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                    </p>
                  </div>
                  {b.total_price != null && (
                    <div className="text-right flex-shrink-0">
                      <p className="font-semibold text-surface-900 dark:text-white">{b.total_price} {b.currency || 'EUR'}</p>
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </motion.div>

        {/* Pagination */}
        {totalPages > 1 && (
          <motion.div variants={item} className="flex items-center justify-between">
            <span className="text-sm text-surface-500 dark:text-surface-400">
              {t('history.showing', { from: (page - 1) * 10 + 1, to: Math.min(page * 10, total), total })}
            </span>
            <div className="flex items-center gap-2">
              <button
                onClick={() => setPage(p => Math.max(1, p - 1))}
                disabled={page <= 1}
                className="p-2 rounded-lg border border-surface-200 dark:border-surface-700 disabled:opacity-40 hover:bg-surface-50 dark:hover:bg-surface-800 transition-colors"
                aria-label={t('history.prevPage')}
              >
                <CaretLeft weight="bold" className="w-4 h-4" />
              </button>
              <span className="text-sm font-medium text-surface-900 dark:text-white px-2">
                {page} / {totalPages}
              </span>
              <button
                onClick={() => setPage(p => Math.min(totalPages, p + 1))}
                disabled={page >= totalPages}
                className="p-2 rounded-lg border border-surface-200 dark:border-surface-700 disabled:opacity-40 hover:bg-surface-50 dark:hover:bg-surface-800 transition-colors"
                aria-label={t('history.nextPage')}
              >
                <CaretRight weight="bold" className="w-4 h-4" />
              </button>
            </div>
          </motion.div>
        )}
      </motion.div>
    </AnimatePresence>
  );
}
