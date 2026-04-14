import { useEffect, useState, useCallback, useRef } from 'react';
import { Link } from 'react-router-dom';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  QrCode, SignIn, SignOut, SpinnerGap, Clock,
  MapPin, CalendarBlank, ArrowClockwise,
} from '@phosphor-icons/react';
import toast from 'react-hot-toast';
import { format } from 'date-fns';
import { de, enUS } from 'date-fns/locale';
import { api, getInMemoryToken, type Booking } from '../api/client';
import { stagger, fadeUp } from '../constants/animations';

function authHeaders(): Record<string, string> {
  const token = getInMemoryToken();
  return {
    'Content-Type': 'application/json',
    Accept: 'application/json',
    'X-Requested-With': 'XMLHttpRequest',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
  };
}

interface CheckInStatus {
  checked_in: boolean;
  checked_in_at: string | null;
  checked_out_at: string | null;
}

export function QRCheckInPage() {
  const { t, i18n } = useTranslation();
  const dateFnsLocale = i18n.language?.startsWith('de') ? de : enUS;
  const [booking, setBooking] = useState<Booking | null>(null);
  const [checkInStatus, setCheckInStatus] = useState<CheckInStatus | null>(null);
  const [qrUrl, setQrUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [acting, setActing] = useState(false);
  const [elapsed, setElapsed] = useState('');
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const bookingsRes = await api.getBookings();
      if (bookingsRes.success && bookingsRes.data) {
        const now = Date.now();
        const active = bookingsRes.data.find(
          b => (b.status === 'active' || b.status === 'confirmed') &&
               new Date(b.start_time).getTime() <= now &&
               new Date(b.end_time).getTime() > now,
        );
        if (active) {
          setBooking(active);
          // Load check-in status
          const statusRes = await fetch(`/api/v1/bookings/${active.id}/check-in`, { headers: authHeaders(), credentials: 'include' }).then(r => r.json());
          if (statusRes.success && statusRes.data) {
            setCheckInStatus(statusRes.data);
          } else {
            setCheckInStatus({ checked_in: false, checked_in_at: null, checked_out_at: null });
          }
          // Load QR code (check r.ok to avoid rendering error HTML as image)
          try {
            const qrRes = await fetch(`/api/v1/bookings/${active.id}/qr`, { headers: authHeaders(), credentials: 'include' });
            if (qrRes.ok) {
              const qrBlob = await qrRes.blob();
              setQrUrl(URL.createObjectURL(qrBlob));
            }
          /* istanbul ignore next -- network failure path */
          } catch {
            // QR endpoint may not be compiled — non-critical
          }
        } else {
          setBooking(null);
          setCheckInStatus(null);
        }
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
    setLoading(false);
  }, [t]);

  useEffect(() => { loadData(); }, [loadData]);

  // Elapsed timer
  useEffect(() => {
    if (timerRef.current) clearInterval(timerRef.current);
    if (checkInStatus?.checked_in && checkInStatus.checked_in_at && !checkInStatus.checked_out_at) {
      const update = () => {
        const diff = Date.now() - new Date(checkInStatus.checked_in_at!).getTime();
        const hours = Math.floor(diff / 3600000);
        const minutes = Math.floor((diff % 3600000) / 60000);
        const seconds = Math.floor((diff % 60000) / 1000);
        setElapsed(`${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`);
      };
      update();
      timerRef.current = setInterval(update, 1000);
    }
    return () => { if (timerRef.current) clearInterval(timerRef.current); };
  }, [checkInStatus]);

  // Revoke QR object URL on unmount
  useEffect(() => {
    return () => { if (qrUrl) URL.revokeObjectURL(qrUrl); };
  }, [qrUrl]);

  async function handleCheckIn() {
    /* istanbul ignore next -- defensive guard; UI only renders this button when booking exists */
    if (!booking) return;
    setActing(true);
    try {
      const res = await fetch(`/api/v1/bookings/${booking.id}/check-in`, {
        method: 'POST',
        headers: authHeaders(),
        credentials: 'include',
      }).then(r => r.json());
      if (res.success) {
        toast.success(t('checkin.checkedIn'));
        loadData();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
    setActing(false);
  }

  async function handleCheckOut() {
    /* istanbul ignore next -- defensive guard; UI only renders this button when booking exists */
    if (!booking) return;
    setActing(true);
    try {
      const res = await fetch(`/api/v1/bookings/${booking.id}/check-out`, {
        method: 'POST',
        headers: authHeaders(),
        credentials: 'include',
      }).then(r => r.json());
      if (res.success) {
        toast.success(t('checkin.checkedOut'));
        loadData();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
    setActing(false);
  }

  if (loading) {
    return (
      <div className="flex justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-2 border-primary-500 border-t-transparent" />
      </div>
    );
  }

  return (
    <motion.div variants={stagger} initial="hidden" animate="show" className="space-y-6 max-w-lg mx-auto">
      {/* Header */}
      <motion.div variants={fadeUp} className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <QrCode weight="duotone" className="w-7 h-7 text-primary-500" />
          <div>
            <h1 className="text-2xl font-bold text-surface-900 dark:text-white">{t('checkin.title')}</h1>
            <p className="text-sm text-surface-500 dark:text-surface-400 mt-0.5">{t('checkin.subtitle')}</p>
          </div>
        </div>
        <button onClick={loadData} className="btn btn-secondary btn-sm">
          <ArrowClockwise weight="bold" className="w-4 h-4" />
        </button>
      </motion.div>

      {/* No active booking */}
      {!booking && (
        <motion.div variants={fadeUp} className="card p-12 text-center">
          <QrCode weight="thin" className="w-16 h-16 mx-auto mb-4 text-surface-300 dark:text-surface-600" />
          <p className="text-surface-500 dark:text-surface-400 text-lg font-medium">{t('checkin.noBooking')}</p>
          <p className="text-surface-400 dark:text-surface-500 text-sm mt-1 mb-4">{t('checkin.noBookingHint')}</p>
          <Link to="/book" className="btn btn-primary">{t('checkin.bookNow')}</Link>
        </motion.div>
      )}

      {/* Active booking */}
      {booking && (
        <>
          {/* Booking details */}
          <motion.div variants={fadeUp} className="card p-5 space-y-3">
            <div className="flex items-center gap-2">
              <MapPin weight="fill" className="w-5 h-5 text-primary-500" />
              <span className="font-semibold text-surface-900 dark:text-white text-lg">{booking.lot_name}</span>
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="glass-card p-3">
                <p className="text-xs text-surface-400 dark:text-surface-500 mb-1">{t('dashboard.slot')}</p>
                <p className="font-semibold text-surface-800 dark:text-surface-200">{booking.slot_number}</p>
              </div>
              <div className="glass-card p-3">
                <p className="text-xs text-surface-400 dark:text-surface-500 mb-1">{t('checkin.date')}</p>
                <p className="font-semibold text-surface-800 dark:text-surface-200">
                  {format(new Date(booking.start_time), 'd. MMM', { locale: dateFnsLocale })}
                </p>
              </div>
              <div className="glass-card p-3">
                <p className="text-xs text-surface-400 dark:text-surface-500 mb-1">{t('checkin.startTime')}</p>
                <p className="font-semibold text-surface-800 dark:text-surface-200 flex items-center gap-1">
                  <CalendarBlank weight="regular" className="w-3.5 h-3.5" />
                  {format(new Date(booking.start_time), 'HH:mm')}
                </p>
              </div>
              <div className="glass-card p-3">
                <p className="text-xs text-surface-400 dark:text-surface-500 mb-1">{t('checkin.endTime')}</p>
                <p className="font-semibold text-surface-800 dark:text-surface-200 flex items-center gap-1">
                  <Clock weight="regular" className="w-3.5 h-3.5" />
                  {format(new Date(booking.end_time), 'HH:mm')}
                </p>
              </div>
            </div>
          </motion.div>

          {/* Not checked in */}
          {checkInStatus && !checkInStatus.checked_in && (
            <>
              {qrUrl && (
                <motion.div variants={fadeUp} className="card p-6 flex flex-col items-center">
                  <img
                    src={qrUrl}
                    alt={t('checkin.qrAlt')}
                    className="w-56 h-56 rounded-xl border border-surface-200 dark:border-surface-700"
                    data-testid="qr-code"
                  />
                  <p className="text-xs text-surface-400 dark:text-surface-500 mt-3">{t('checkin.scanQr')}</p>
                </motion.div>
              )}
              <motion.div variants={fadeUp}>
                <button
                  onClick={handleCheckIn}
                  disabled={acting}
                  className="btn btn-primary w-full py-3 text-base"
                  data-testid="checkin-btn"
                >
                  {acting
                    ? <SpinnerGap weight="bold" className="w-5 h-5 animate-spin" />
                    : <><SignIn weight="bold" className="w-5 h-5" /> {t('checkin.checkInBtn')}</>
                  }
                </button>
              </motion.div>
            </>
          )}

          {/* Checked in */}
          {checkInStatus?.checked_in && !checkInStatus.checked_out_at && (
            <>
              <motion.div variants={fadeUp} className="stat-card p-6 text-center">
                <p className="text-xs font-medium text-surface-400 dark:text-surface-500 uppercase tracking-wider mb-2">
                  {t('checkin.elapsed')}
                </p>
                <p className="stat-value text-4xl font-mono" data-testid="elapsed-timer">{elapsed}</p>
                {checkInStatus.checked_in_at && (
                  <p className="text-sm text-surface-500 dark:text-surface-400 mt-2">
                    {t('checkin.since', { time: format(new Date(checkInStatus.checked_in_at), 'HH:mm', { locale: dateFnsLocale }) })}
                  </p>
                )}
              </motion.div>
              <motion.div variants={fadeUp}>
                <button
                  onClick={handleCheckOut}
                  disabled={acting}
                  className="btn btn-secondary w-full py-3 text-base border-red-200 text-red-600 hover:bg-red-50 dark:border-red-800 dark:text-red-400 dark:hover:bg-red-900/20"
                  data-testid="checkout-btn"
                >
                  {acting
                    ? <SpinnerGap weight="bold" className="w-5 h-5 animate-spin" />
                    : <><SignOut weight="bold" className="w-5 h-5" /> {t('checkin.checkOutBtn')}</>
                  }
                </button>
              </motion.div>
            </>
          )}
        </>
      )}
    </motion.div>
  );
}
