import { useEffect, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  Swap, Check, X, SpinnerGap, Plus, ArrowClockwise,
  CalendarBlank, Clock, ChatText,
} from '@phosphor-icons/react';
import toast from 'react-hot-toast';
import { format } from 'date-fns';
import { de, enUS } from 'date-fns/locale';
import { api, type Booking } from '../api/client';
import { stagger, fadeUp, modalVariants, modalTransition } from '../constants/animations';

interface SwapRequest {
  id: string;
  requester_id: string;
  source_booking_id: string;
  target_booking_id: string;
  source_booking: { lot_name: string; slot_number: string; start_time: string; end_time: string };
  target_booking: { lot_name: string; slot_number: string; start_time: string; end_time: string };
  message: string | null;
  status: 'pending' | 'accepted' | 'declined';
  created_at: string;
}

const statusConfig: Record<string, { cls: string }> = {
  pending: { cls: 'badge badge-warning' },
  accepted: { cls: 'badge badge-success' },
  declined: { cls: 'badge badge-error' },
};

export function SwapRequestsPage() {
  const { t, i18n } = useTranslation();
  const dateFnsLocale = i18n.language?.startsWith('de') ? de : enUS;
  const [requests, setRequests] = useState<SwapRequest[]>([]);
  const [bookings, setBookings] = useState<Booking[]>([]);
  const [loading, setLoading] = useState(true);
  const [acting, setActing] = useState<string | null>(null);
  const [showModal, setShowModal] = useState(false);
  const [selectedBooking, setSelectedBooking] = useState<string>('');
  const [targetBookingId, setTargetBookingId] = useState('');
  const [swapMessage, setSwapMessage] = useState('');
  const [creating, setCreating] = useState(false);

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const [swapRes, bookingsRes] = await Promise.all([
        fetch('/api/v1/swap-requests').then(r => r.json()),
        api.getBookings(),
      ]);
      if (swapRes.success && swapRes.data) setRequests(swapRes.data);
      if (bookingsRes.success && bookingsRes.data) setBookings(bookingsRes.data);
    } catch {
      toast.error(t('common.error'));
    }
    setLoading(false);
  }, [t]);

  useEffect(() => { loadData(); }, [loadData]);

  async function handleAccept(id: string) {
    setActing(id);
    try {
      const res = await fetch(`/api/v1/swap-requests/${id}/accept`, { method: 'POST' }).then(r => r.json());
      if (res.success) {
        setRequests(prev => prev.map(r => r.id === id ? { ...r, status: 'accepted' } : r));
        toast.success(t('swap.accepted'));
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } catch {
      toast.error(t('common.error'));
    }
    setActing(null);
  }

  async function handleDecline(id: string) {
    setActing(id);
    try {
      const res = await fetch(`/api/v1/swap-requests/${id}/decline`, { method: 'POST' }).then(r => r.json());
      if (res.success) {
        setRequests(prev => prev.map(r => r.id === id ? { ...r, status: 'declined' } : r));
        toast.success(t('swap.declined'));
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } catch {
      toast.error(t('common.error'));
    }
    setActing(null);
  }

  async function handleCreate() {
    if (!selectedBooking || !targetBookingId) return;
    setCreating(true);
    try {
      const res = await fetch(`/api/v1/bookings/${selectedBooking}/swap-request`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ target_booking_id: targetBookingId, message: swapMessage || null }),
      }).then(r => r.json());
      if (res.success) {
        toast.success(t('swap.created'));
        setShowModal(false);
        setSelectedBooking('');
        setTargetBookingId('');
        setSwapMessage('');
        loadData();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } catch {
      toast.error(t('common.error'));
    }
    setCreating(false);
  }

  const activeBookings = bookings.filter(b => b.status === 'active' || b.status === 'confirmed');

  if (loading) {
    return (
      <div className="flex justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-2 border-primary-500 border-t-transparent" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <motion.div variants={stagger} initial="hidden" animate="show" className="space-y-6">
        {/* Header */}
        <motion.div variants={fadeUp} className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <Swap weight="duotone" className="w-7 h-7 text-primary-500" />
            <div>
              <h1 className="text-2xl font-bold text-surface-900 dark:text-white">{t('swap.title')}</h1>
              <p className="text-sm text-surface-500 dark:text-surface-400 mt-0.5">{t('swap.subtitle')}</p>
            </div>
          </div>
          <div className="flex items-center gap-2 self-start sm:self-auto">
            <button onClick={loadData} className="btn btn-secondary">
              <ArrowClockwise weight="bold" className="w-4 h-4" /> {t('common.refresh')}
            </button>
            <button onClick={() => setShowModal(true)} className="btn btn-primary">
              <Plus weight="bold" className="w-4 h-4" /> {t('swap.create')}
            </button>
          </div>
        </motion.div>

        {/* Request list */}
        {requests.length === 0 ? (
          <motion.div variants={fadeUp} className="card p-12 text-center">
            <Swap weight="thin" className="w-16 h-16 mx-auto mb-4 text-surface-300 dark:text-surface-600" />
            <p className="text-surface-500 dark:text-surface-400 text-lg font-medium">{t('swap.empty')}</p>
            <p className="text-surface-400 dark:text-surface-500 text-sm mt-1">{t('swap.emptyHint')}</p>
          </motion.div>
        ) : (
          <div className="space-y-3">
            <AnimatePresence>
              {requests.map(req => {
                const cfg = statusConfig[req.status] || statusConfig.pending;
                return (
                  <motion.div
                    key={req.id}
                    initial={{ opacity: 0, y: 12 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, x: -60 }}
                    className="card p-4"
                  >
                    <div className="flex items-start justify-between mb-3">
                      <div className="flex items-center gap-2">
                        <Swap weight="bold" className="w-4 h-4 text-primary-500" />
                        <span className="font-semibold text-surface-900 dark:text-white">
                          {req.source_booking.lot_name}
                        </span>
                        <span className="text-surface-400 dark:text-surface-500">→</span>
                        <span className="font-semibold text-surface-900 dark:text-white">
                          {req.target_booking.lot_name}
                        </span>
                      </div>
                      <span className={cfg.cls}>{t(`swap.status.${req.status}`)}</span>
                    </div>

                    <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 mb-3">
                      {/* Source */}
                      <div className="glass-card p-3">
                        <p className="text-xs font-medium text-surface-400 dark:text-surface-500 mb-1">{t('swap.yourSlot')}</p>
                        <div className="flex items-center gap-2 text-sm text-surface-700 dark:text-surface-300">
                          <CalendarBlank weight="regular" className="w-3.5 h-3.5" />
                          {format(new Date(req.source_booking.start_time), 'd. MMM yyyy', { locale: dateFnsLocale })}
                        </div>
                        <div className="flex items-center gap-2 text-sm text-surface-600 dark:text-surface-400 mt-1">
                          <Clock weight="regular" className="w-3.5 h-3.5" />
                          {t('dashboard.slot')} {req.source_booking.slot_number} &middot;{' '}
                          {format(new Date(req.source_booking.start_time), 'HH:mm')} — {format(new Date(req.source_booking.end_time), 'HH:mm')}
                        </div>
                      </div>
                      {/* Target */}
                      <div className="glass-card p-3">
                        <p className="text-xs font-medium text-surface-400 dark:text-surface-500 mb-1">{t('swap.theirSlot')}</p>
                        <div className="flex items-center gap-2 text-sm text-surface-700 dark:text-surface-300">
                          <CalendarBlank weight="regular" className="w-3.5 h-3.5" />
                          {format(new Date(req.target_booking.start_time), 'd. MMM yyyy', { locale: dateFnsLocale })}
                        </div>
                        <div className="flex items-center gap-2 text-sm text-surface-600 dark:text-surface-400 mt-1">
                          <Clock weight="regular" className="w-3.5 h-3.5" />
                          {t('dashboard.slot')} {req.target_booking.slot_number} &middot;{' '}
                          {format(new Date(req.target_booking.start_time), 'HH:mm')} — {format(new Date(req.target_booking.end_time), 'HH:mm')}
                        </div>
                      </div>
                    </div>

                    {req.message && (
                      <div className="flex items-start gap-2 text-sm text-surface-600 dark:text-surface-400 mb-3">
                        <ChatText weight="regular" className="w-4 h-4 mt-0.5 flex-shrink-0" />
                        <p>{req.message}</p>
                      </div>
                    )}

                    {req.status === 'pending' && (
                      <div className="flex items-center gap-2 pt-3 border-t border-surface-100 dark:border-surface-800">
                        <button
                          onClick={() => handleAccept(req.id)}
                          disabled={acting === req.id}
                          className="btn btn-sm btn-primary"
                        >
                          {acting === req.id
                            ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" />
                            : <><Check weight="bold" className="w-4 h-4" /> {t('swap.accept')}</>
                          }
                        </button>
                        <button
                          onClick={() => handleDecline(req.id)}
                          disabled={acting === req.id}
                          className="btn btn-sm btn-ghost text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
                        >
                          <X weight="bold" className="w-4 h-4" /> {t('swap.decline')}
                        </button>
                      </div>
                    )}
                  </motion.div>
                );
              })}
            </AnimatePresence>
          </div>
        )}
      </motion.div>

      {/* Create Swap Modal */}
      <AnimatePresence>
        {showModal && (
          <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm p-4">
            <motion.div
              variants={modalVariants}
              initial="initial"
              animate="animate"
              exit="exit"
              transition={modalTransition}
              className="card w-full max-w-md p-6 space-y-4"
              data-testid="swap-modal"
            >
              <div className="flex items-center justify-between">
                <h2 className="text-lg font-semibold text-surface-900 dark:text-white">{t('swap.createTitle')}</h2>
                <button onClick={() => setShowModal(false)} className="btn btn-ghost btn-sm">
                  <X weight="bold" className="w-4 h-4" />
                </button>
              </div>

              <div>
                <label className="text-sm font-medium text-surface-700 dark:text-surface-300">{t('swap.yourBooking')}</label>
                <select
                  value={selectedBooking}
                  onChange={e => setSelectedBooking(e.target.value)}
                  className="input mt-1"
                  data-testid="select-source"
                >
                  <option value="">{t('swap.selectBooking')}</option>
                  {activeBookings.map(b => (
                    <option key={b.id} value={b.id}>
                      {b.lot_name} — {t('dashboard.slot')} {b.slot_number} ({format(new Date(b.start_time), 'dd.MM HH:mm')})
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="text-sm font-medium text-surface-700 dark:text-surface-300">{t('swap.targetBookingId')}</label>
                <input
                  type="text"
                  value={targetBookingId}
                  onChange={e => setTargetBookingId(e.target.value)}
                  placeholder={t('swap.targetPlaceholder')}
                  className="input mt-1"
                  data-testid="input-target"
                />
              </div>

              <div>
                <label className="text-sm font-medium text-surface-700 dark:text-surface-300">{t('swap.messageLabel')}</label>
                <textarea
                  value={swapMessage}
                  onChange={e => setSwapMessage(e.target.value)}
                  placeholder={t('swap.messagePlaceholder')}
                  className="input mt-1 h-20 resize-none"
                  data-testid="input-message"
                />
              </div>

              <div className="flex justify-end gap-2 pt-2">
                <button onClick={() => setShowModal(false)} className="btn btn-secondary">
                  {t('common.cancel')}
                </button>
                <button
                  onClick={handleCreate}
                  disabled={creating || !selectedBooking || !targetBookingId}
                  className="btn btn-primary"
                  data-testid="submit-swap"
                >
                  {creating
                    ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" />
                    : <><Swap weight="bold" className="w-4 h-4" /> {t('swap.send')}</>
                  }
                </button>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </div>
  );
}
