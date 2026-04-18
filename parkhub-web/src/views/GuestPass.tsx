import { useActionState, useState, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { UserPlus, Copy, ShareNetwork, Trash, QrCode, SpinnerGap, CheckCircle, CalendarBlank, MapPin } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../context/AuthContext';
import { getInMemoryToken } from '../api/client';
import toast from 'react-hot-toast';

interface Lot {
  id: string;
  name: string;
}

interface Slot {
  id: string;
  number: string;
  status: 'available' | 'occupied' | 'reserved';
}

interface GuestBooking {
  id: string;
  lot_id: string;
  lot_name: string;
  slot_id: string;
  slot_number: string;
  guest_name: string;
  guest_email: string | null;
  guest_code: string;
  start_time: string;
  end_time: string;
  status: 'active' | 'expired' | 'cancelled';
  created_at: string;
}

interface FormData {
  guest_name: string;
  guest_email: string;
  lot_id: string;
  slot_id: string;
  start_time: string;
  end_time: string;
}

const emptyForm: FormData = {
  guest_name: '',
  guest_email: '',
  lot_id: '',
  slot_id: '',
  start_time: '',
  end_time: '',
};

const statusColors: Record<string, string> = {
  active: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
  expired: 'bg-surface-100 text-surface-500 dark:bg-surface-800 dark:text-surface-400',
  cancelled: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
};

function authHeaders(): Record<string, string> {
  const token = getInMemoryToken();
  return {
    'Content-Type': 'application/json',
    Accept: 'application/json',
    'X-Requested-With': 'XMLHttpRequest',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
  };
}

export function GuestPassPage() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const isAdmin = user && ['admin', 'superadmin'].includes(user.role);

  const [bookings, setBookings] = useState<GuestBooking[]>([]);
  const [lots, setLots] = useState<Lot[]>([]);
  const [slots, setSlots] = useState<Slot[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [form, setForm] = useState<FormData>(emptyForm);
  const [createdPass, setCreatedPass] = useState<GuestBooking | null>(null);
  const [copied, setCopied] = useState(false);

  const loadBookings = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch('/api/v1/bookings/guest', {
        headers: authHeaders(),
        credentials: 'include',
      }).then(r => r.json());
      if (res.success) setBookings(res.data || []);
    } catch { toast.error(t('common.error')); }
    setLoading(false);
  }, []);

  const loadLots = useCallback(async () => {
    try {
      const res = await fetch('/api/v1/lots', {
        headers: authHeaders(),
        credentials: 'include',
      }).then(r => r.json());
      if (res.success) setLots(res.data || []);
    } catch { toast.error(t('common.error')); }
  }, []);

  useEffect(() => {
    loadBookings();
    loadLots();
  }, [loadBookings, loadLots]);

  // Load slots when lot changes
  useEffect(() => {
    if (!form.lot_id) {
      setSlots([]);
      return;
    }
    (async () => {
      try {
        const res = await fetch(`/api/v1/lots/${form.lot_id}/slots`, {
          headers: authHeaders(),
          credentials: 'include',
        }).then(r => r.json());
        if (res.success) setSlots(res.data || []);
      } catch { toast.error(t('common.error')); }
    })();
  }, [form.lot_id]);

  // React 19: useActionState drives pass creation. Third tuple entry
  // (isSubmitting) replaces the manual setSubmitting boolean; action
  // closes over current form state (all fields are already controlled
  // React inputs — two selects, two datetime pickers, two text inputs).
  const [, createAction, isSubmitting] = useActionState(async () => {
    if (!form.guest_name.trim() || !form.lot_id || !form.slot_id || !form.start_time || !form.end_time) {
      toast.error(t('guestBooking.requiredFields'));
      return null;
    }
    try {
      const res = await fetch('/api/v1/bookings/guest', {
        method: 'POST',
        headers: authHeaders(),
        credentials: 'include',
        body: JSON.stringify({
          lot_id: form.lot_id,
          slot_id: form.slot_id,
          start_time: new Date(form.start_time).toISOString(),
          end_time: new Date(form.end_time).toISOString(),
          guest_name: form.guest_name,
          guest_email: form.guest_email || null,
        }),
      }).then(r => r.json());

      if (res.success && res.data) {
        setCreatedPass(res.data);
        setShowForm(false);
        setForm(emptyForm);
        loadBookings();
        toast.success(t('guestBooking.created'));
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
    return null;
  }, null);

  async function handleCancel(id: string) {
    try {
      const res = await fetch(`/api/v1/bookings/guest/${id}`, {
        method: 'DELETE',
        headers: authHeaders(),
        credentials: 'include',
      }).then(r => r.json());
      if (res.success) {
        toast.success(t('guestBooking.cancelled'));
        loadBookings();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
  }

  function copyCode(code: string) {
    navigator.clipboard.writeText(code).then(() => {
      setCopied(true);
      toast.success(t('guestBooking.codeCopied'));
      setTimeout(() => setCopied(false), 2000);
    });
  }

  function sharePass(pass: GuestBooking) {
    const url = `${window.location.origin}/guest-pass/${pass.guest_code}`;
    const text = t('guestBooking.shareText', { name: pass.guest_name, code: pass.guest_code });
    if (navigator.share) {
      navigator.share({ title: t('guestBooking.shareTitle'), text, url }).catch(() => {});
    } else {
      navigator.clipboard.writeText(url).then(() => toast.success(t('guestBooking.linkCopied')));
    }
  }

  return (
    <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} className="space-y-6" data-testid="guest-pass-page">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white flex items-center gap-2">
            <UserPlus weight="duotone" className="w-7 h-7 text-primary-500" />
            {t('guestBooking.title')}
          </h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">{t('guestBooking.subtitle')}</p>
        </div>
        <button onClick={() => { setShowForm(true); setCreatedPass(null); }} className="btn btn-primary flex items-center gap-2" data-testid="create-guest-btn">
          <UserPlus weight="bold" className="w-4 h-4" />
          {t('guestBooking.create')}
        </button>
      </div>

      {/* Created pass card */}
      <AnimatePresence>
        {createdPass && (
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            className="bg-white dark:bg-surface-800 rounded-2xl border-2 border-primary-300 dark:border-primary-700 p-6 shadow-lg"
            data-testid="guest-pass-card"
          >
            <div className="flex items-start justify-between mb-4">
              <div>
                <h2 className="text-lg font-bold text-surface-900 dark:text-white">{t('guestBooking.passCreated')}</h2>
                <p className="text-sm text-surface-500">{t('guestBooking.shareInstructions')}</p>
              </div>
              <button
                onClick={() => setCreatedPass(null)}
                className="text-surface-400 hover:text-surface-600"
                data-testid="dismiss-pass"
              >&times;</button>
            </div>

            <div className="flex flex-col sm:flex-row gap-6">
              {/* QR Code */}
              <div className="flex-shrink-0 flex items-center justify-center bg-white rounded-xl p-4 border border-surface-200">
                <div className="w-40 h-40 flex items-center justify-center" data-testid="qr-placeholder">
                  <QrCode weight="thin" className="w-32 h-32 text-surface-300" />
                  <span className="sr-only">{`/guest-pass/${createdPass.guest_code}`}</span>
                </div>
              </div>

              {/* Pass details */}
              <div className="flex-1 space-y-3">
                <div>
                  <span className="text-xs text-surface-400 uppercase tracking-wider">{t('guestBooking.guestName')}</span>
                  <p className="font-semibold text-surface-900 dark:text-white">{createdPass.guest_name}</p>
                </div>
                <div className="flex gap-4">
                  <div>
                    <span className="text-xs text-surface-400 uppercase tracking-wider">{t('guestBooking.lot')}</span>
                    <p className="text-sm text-surface-700 dark:text-surface-300">{createdPass.lot_name}</p>
                  </div>
                  <div>
                    <span className="text-xs text-surface-400 uppercase tracking-wider">{t('guestBooking.slot')}</span>
                    <p className="text-sm text-surface-700 dark:text-surface-300">{createdPass.slot_number}</p>
                  </div>
                </div>
                <div>
                  <span className="text-xs text-surface-400 uppercase tracking-wider">{t('guestBooking.dateRange')}</span>
                  <p className="text-sm text-surface-700 dark:text-surface-300">
                    {new Date(createdPass.start_time).toLocaleString()} — {new Date(createdPass.end_time).toLocaleString()}
                  </p>
                </div>
                <div>
                  <span className="text-xs text-surface-400 uppercase tracking-wider">{t('guestBooking.code')}</span>
                  <div className="flex items-center gap-2 mt-1">
                    <code className="bg-surface-100 dark:bg-surface-700 px-3 py-1.5 rounded-lg text-lg font-mono font-bold tracking-widest text-primary-600 dark:text-primary-400" data-testid="guest-code">
                      {createdPass.guest_code}
                    </code>
                    <button onClick={() => copyCode(createdPass.guest_code)} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700" data-testid="copy-code-btn">
                      {copied ? <CheckCircle size={18} className="text-green-500" /> : <Copy size={18} className="text-surface-500" />}
                    </button>
                  </div>
                </div>
                <button onClick={() => sharePass(createdPass)} className="btn btn-secondary flex items-center gap-2 mt-2" data-testid="share-pass-btn">
                  <ShareNetwork size={16} />
                  {t('guestBooking.share')}
                </button>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Create form */}
      <AnimatePresence>
        {showForm && (
          <motion.div initial={{ opacity: 0, y: -8 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0, y: -8 }} className="bg-white dark:bg-surface-800 rounded-xl border border-surface-200 dark:border-surface-700 p-6" data-testid="guest-form">
            <h2 className="text-lg font-semibold text-surface-900 dark:text-white mb-4">{t('guestBooking.formTitle')}</h2>
            <form action={createAction} className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('guestBooking.guestName')} *</label>
                <input
                  type="text"
                  value={form.guest_name}
                  onChange={e => setForm({ ...form, guest_name: e.target.value })}
                  className="input"
                  required
                  data-testid="input-guest-name"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('guestBooking.guestEmail')}</label>
                <input
                  type="email"
                  value={form.guest_email}
                  onChange={e => setForm({ ...form, guest_email: e.target.value })}
                  className="input"
                  data-testid="input-guest-email"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('guestBooking.lot')} *</label>
                <select
                  value={form.lot_id}
                  onChange={e => setForm({ ...form, lot_id: e.target.value, slot_id: '' })}
                  className="input"
                  required
                  data-testid="select-lot"
                >
                  <option value="">{t('guestBooking.selectLot')}</option>
                  {lots.map(lot => (
                    <option key={lot.id} value={lot.id}>{lot.name}</option>
                  ))}
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('guestBooking.slot')} *</label>
                <select
                  value={form.slot_id}
                  onChange={e => setForm({ ...form, slot_id: e.target.value })}
                  className="input"
                  required
                  disabled={!form.lot_id}
                  data-testid="select-slot"
                >
                  <option value="">{t('guestBooking.selectSlot')}</option>
                  {slots.filter(s => s.status === 'available').map(slot => (
                    <option key={slot.id} value={slot.id}>{slot.number}</option>
                  ))}
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('guestBooking.startTime')} *</label>
                <input
                  type="datetime-local"
                  value={form.start_time}
                  onChange={e => setForm({ ...form, start_time: e.target.value })}
                  className="input"
                  required
                  data-testid="input-start-time"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('guestBooking.endTime')} *</label>
                <input
                  type="datetime-local"
                  value={form.end_time}
                  onChange={e => setForm({ ...form, end_time: e.target.value })}
                  className="input"
                  required
                  data-testid="input-end-time"
                />
              </div>
              <div className="sm:col-span-2 flex justify-end gap-2">
                <button type="button" onClick={() => { setShowForm(false); setForm(emptyForm); }} className="btn btn-secondary">{t('common.cancel')}</button>
                <button type="submit" disabled={isSubmitting} className="btn btn-primary" data-testid="submit-guest-btn">
                  {isSubmitting ? <SpinnerGap size={16} className="animate-spin inline mr-1" /> : null}
                  {isSubmitting ? t('guestBooking.creating') : t('common.save')}
                </button>
              </div>
            </form>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Existing guest bookings */}
      <div>
        <h2 className="text-lg font-semibold text-surface-900 dark:text-white mb-3">{t('guestBooking.existing')}</h2>
        {loading ? (
          <div className="flex justify-center py-12">
            <div className="w-8 h-8 border-2 border-primary-500 border-t-transparent rounded-full animate-spin" />
          </div>
        ) : bookings.length === 0 ? (
          <div className="text-center py-12 text-surface-400" data-testid="empty-state">
            <UserPlus className="w-12 h-12 mx-auto mb-3 opacity-40" />
            <p>{t('guestBooking.empty')}</p>
          </div>
        ) : (
          <div className="space-y-3" data-testid="bookings-list">
            {bookings.map(b => (
              <motion.div key={b.id} initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="bg-white dark:bg-surface-800 rounded-xl border border-surface-200 dark:border-surface-700 p-4 flex items-center justify-between">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-3">
                    <span className="font-semibold text-surface-900 dark:text-white">{b.guest_name}</span>
                    <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${statusColors[b.status] || statusColors.active}`}>
                      {t(`guestBooking.status.${b.status}`)}
                    </span>
                    <code className="text-xs bg-surface-100 dark:bg-surface-700 px-2 py-0.5 rounded font-mono text-primary-600 dark:text-primary-400">{b.guest_code}</code>
                  </div>
                  <div className="flex items-center gap-4 mt-1 text-sm text-surface-500">
                    <span className="flex items-center gap-1"><MapPin className="w-3.5 h-3.5" />{b.lot_name} / {b.slot_number}</span>
                    <span className="flex items-center gap-1"><CalendarBlank className="w-3.5 h-3.5" />{new Date(b.start_time).toLocaleString()} — {new Date(b.end_time).toLocaleString()}</span>
                  </div>
                </div>
                <div className="flex items-center gap-2 ml-4">
                  <button onClick={() => sharePass(b)} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700 text-surface-500" title={t('guestBooking.share')}>
                    <ShareNetwork className="w-5 h-5" />
                  </button>
                  {isAdmin && b.status === 'active' && (
                    <button onClick={() => handleCancel(b.id)} className="p-2 rounded-lg hover:bg-red-50 dark:hover:bg-red-900/20 text-red-500" title={t('guestBooking.cancel')} data-testid={`cancel-${b.id}`}>
                      <Trash className="w-5 h-5" />
                    </button>
                  )}
                </div>
              </motion.div>
            ))}
          </div>
        )}
      </div>
    </motion.div>
  );
}
