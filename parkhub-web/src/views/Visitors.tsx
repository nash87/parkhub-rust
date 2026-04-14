import { useEffect, useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { UserPlus, QrCode, Trash, CheckCircle, Question, MagnifyingGlass, CalendarBlank, Envelope } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../context/AuthContext';
import toast from 'react-hot-toast';

interface Visitor {
  id: string;
  host_user_id: string;
  name: string;
  email: string;
  vehicle_plate: string | null;
  visit_date: string;
  purpose: string | null;
  status: 'pending' | 'checked_in' | 'expired' | 'cancelled';
  qr_code: string | null;
  pass_url: string | null;
  checked_in_at: string | null;
  created_at: string;
}

interface FormData {
  name: string;
  email: string;
  vehicle_plate: string;
  visit_date: string;
  purpose: string;
}

const emptyForm: FormData = {
  name: '',
  email: '',
  vehicle_plate: '',
  visit_date: '',
  purpose: '',
};

const statusColors: Record<string, string> = {
  pending: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  checked_in: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
  expired: 'bg-surface-100 text-surface-500 dark:bg-surface-800 dark:text-surface-400',
  cancelled: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
};

export function VisitorsPage() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const isAdmin = user && ['admin', 'superadmin'].includes(user.role);
  const [visitors, setVisitors] = useState<Visitor[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [form, setForm] = useState<FormData>(emptyForm);
  const [submitting, setSubmitting] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const [showQr, setShowQr] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const [viewMode, setViewMode] = useState<'my' | 'admin'>('my');

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const url = viewMode === 'admin' && isAdmin ? '/api/v1/admin/visitors' : '/api/v1/visitors';
      const res = await fetch(url).then(r => r.json());
      if (res.success) setVisitors(res.data || []);
    } catch { /* ignore */ }
    setLoading(false);
  }, [viewMode, isAdmin]);

  useEffect(() => { loadData(); }, [loadData]);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!form.name.trim() || !form.email.trim() || !form.visit_date) {
      toast.error(t('visitors.requiredFields'));
      return;
    }
    setSubmitting(true);
    try {
      const body: Record<string, unknown> = {
        name: form.name,
        email: form.email,
        visit_date: new Date(form.visit_date).toISOString(),
      };
      if (form.vehicle_plate) body.vehicle_plate = form.vehicle_plate;
      if (form.purpose) body.purpose = form.purpose;

      const res = await fetch('/api/v1/visitors/register', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      }).then(r => r.json());

      if (res.success) {
        toast.success(t('visitors.registered'));
        setShowForm(false);
        setForm(emptyForm);
        loadData();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
    setSubmitting(false);
  }

  async function handleCheckIn(id: string) {
    try {
      const res = await fetch(`/api/v1/visitors/${id}/check-in`, { method: 'PUT' }).then(r => r.json());
      if (res.success) {
        toast.success(t('visitors.checkedIn'));
        loadData();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } catch { toast.error(t('common.error')); }
  }

  async function handleCancel(id: string) {
    try {
      const res = await fetch(`/api/v1/visitors/${id}`, { method: 'DELETE' }).then(r => r.json());
      if (res.success) {
        toast.success(t('visitors.cancelled'));
        loadData();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } catch { toast.error(t('common.error')); }
  }

  const filtered = visitors.filter(v => {
    if (!search) return true;
    const q = search.toLowerCase();
    return v.name.toLowerCase().includes(q) || v.email.toLowerCase().includes(q) || (v.vehicle_plate && v.vehicle_plate.toLowerCase().includes(q));
  });

  return (
    <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white flex items-center gap-2">
            <UserPlus weight="duotone" className="w-7 h-7 text-primary-500" />
            {t('visitors.title')}
            <button onClick={() => setShowHelp(!showHelp)} className="text-surface-400 hover:text-primary-500 transition-colors" aria-label="Help">
              <Question weight="fill" className="w-5 h-5" />
            </button>
          </h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">{t('visitors.subtitle')}</p>
        </div>
        <div className="flex items-center gap-2">
          {isAdmin && (
            <div className="flex rounded-lg overflow-hidden border border-surface-200 dark:border-surface-700">
              <button
                className={`px-3 py-1.5 text-sm font-medium ${viewMode === 'my' ? 'bg-primary-500 text-white' : 'text-surface-600 dark:text-surface-300'}`}
                onClick={() => setViewMode('my')}
              >{t('visitors.myVisitors')}</button>
              <button
                className={`px-3 py-1.5 text-sm font-medium ${viewMode === 'admin' ? 'bg-primary-500 text-white' : 'text-surface-600 dark:text-surface-300'}`}
                onClick={() => setViewMode('admin')}
              >{t('visitors.allVisitors')}</button>
            </div>
          )}
          <button onClick={() => setShowForm(true)} className="btn-primary flex items-center gap-2">
            <UserPlus weight="bold" className="w-4 h-4" />
            {t('visitors.register')}
          </button>
        </div>
      </div>

      {/* Help tooltip */}
      {showHelp && (
        <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} className="bg-primary-50 dark:bg-primary-900/20 border border-primary-200 dark:border-primary-800 rounded-xl p-4 text-sm text-primary-700 dark:text-primary-300">
          <p className="font-semibold mb-1">{t('visitors.aboutTitle')}</p>
          <p>{t('visitors.help')}</p>
        </motion.div>
      )}

      {/* Search */}
      <div className="relative">
        <MagnifyingGlass className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
        <input
          type="text"
          value={search}
          onChange={e => setSearch(e.target.value)}
          placeholder={t('visitors.searchPlaceholder')}
          className="w-full pl-10 pr-4 py-2.5 rounded-xl bg-surface-50 dark:bg-surface-800 border border-surface-200 dark:border-surface-700 text-sm"
        />
      </div>

      {/* Register form */}
      {showForm && (
        <motion.div initial={{ opacity: 0, y: -8 }} animate={{ opacity: 1, y: 0 }} className="bg-white dark:bg-surface-800 rounded-xl border border-surface-200 dark:border-surface-700 p-6">
          <h2 className="text-lg font-semibold text-surface-900 dark:text-white mb-4">{t('visitors.registerTitle')}</h2>
          <form onSubmit={handleSubmit} className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('visitors.name')} *</label>
              <input type="text" value={form.name} onChange={e => setForm({ ...form, name: e.target.value })} className="input-field" required />
            </div>
            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('visitors.email')} *</label>
              <input type="email" value={form.email} onChange={e => setForm({ ...form, email: e.target.value })} className="input-field" required />
            </div>
            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('visitors.plate')}</label>
              <input type="text" value={form.vehicle_plate} onChange={e => setForm({ ...form, vehicle_plate: e.target.value })} className="input-field" />
            </div>
            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('visitors.date')} *</label>
              <input type="datetime-local" value={form.visit_date} onChange={e => setForm({ ...form, visit_date: e.target.value })} className="input-field" required />
            </div>
            <div className="sm:col-span-2">
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('visitors.purpose')}</label>
              <input type="text" value={form.purpose} onChange={e => setForm({ ...form, purpose: e.target.value })} className="input-field" placeholder={t('visitors.purposePlaceholder')} />
            </div>
            <div className="sm:col-span-2 flex justify-end gap-2">
              <button type="button" onClick={() => { setShowForm(false); setForm(emptyForm); }} className="btn-secondary">{t('common.cancel')}</button>
              <button type="submit" disabled={submitting} className="btn-primary">{submitting ? '...' : t('common.save')}</button>
            </div>
          </form>
        </motion.div>
      )}

      {/* QR modal */}
      {showQr && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40" onClick={() => setShowQr(null)}>
          <motion.div initial={{ scale: 0.9 }} animate={{ scale: 1 }} className="bg-white dark:bg-surface-800 rounded-2xl p-8 max-w-sm" onClick={e => e.stopPropagation()}>
            <h3 className="text-lg font-semibold mb-4 text-center">{t('visitors.qrTitle')}</h3>
            <img src={showQr} alt="Visitor QR Code" className="w-64 h-64 mx-auto" />
            <button onClick={() => setShowQr(null)} className="btn-secondary w-full mt-4">{t('common.close')}</button>
          </motion.div>
        </div>
      )}

      {/* Visitors list */}
      {loading ? (
        <div className="flex justify-center py-12">
          <div className="w-8 h-8 border-2 border-primary-500 border-t-transparent rounded-full animate-spin" />
        </div>
      ) : filtered.length === 0 ? (
        <div className="text-center py-12 text-surface-400">
          <UserPlus className="w-12 h-12 mx-auto mb-3 opacity-40" />
          <p>{t('visitors.empty')}</p>
        </div>
      ) : (
        <div className="space-y-3">
          {filtered.map(v => (
            <motion.div key={v.id} initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="bg-white dark:bg-surface-800 rounded-xl border border-surface-200 dark:border-surface-700 p-4 flex items-center justify-between">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-3">
                  <span className="font-semibold text-surface-900 dark:text-white">{v.name}</span>
                  <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${statusColors[v.status] || statusColors.pending}`}>
                    {t(`visitors.status.${v.status}`)}
                  </span>
                </div>
                <div className="flex items-center gap-4 mt-1 text-sm text-surface-500">
                  <span className="flex items-center gap-1"><Envelope className="w-3.5 h-3.5" />{v.email}</span>
                  <span className="flex items-center gap-1"><CalendarBlank className="w-3.5 h-3.5" />{new Date(v.visit_date).toLocaleString()}</span>
                  {v.vehicle_plate && <span>{v.vehicle_plate}</span>}
                  {v.purpose && <span className="italic">{v.purpose}</span>}
                </div>
              </div>
              <div className="flex items-center gap-2 ml-4">
                {v.qr_code && (
                  <button onClick={() => setShowQr(v.qr_code)} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700 text-surface-500" title={t('visitors.showQr')}>
                    <QrCode className="w-5 h-5" />
                  </button>
                )}
                {v.status === 'pending' && (
                  <>
                    <button onClick={() => handleCheckIn(v.id)} className="p-2 rounded-lg hover:bg-green-50 dark:hover:bg-green-900/20 text-green-600" title={t('visitors.checkIn')}>
                      <CheckCircle className="w-5 h-5" />
                    </button>
                    <button onClick={() => handleCancel(v.id)} className="p-2 rounded-lg hover:bg-red-50 dark:hover:bg-red-900/20 text-red-500" title={t('visitors.cancelVisitor')}>
                      <Trash className="w-5 h-5" />
                    </button>
                  </>
                )}
              </div>
            </motion.div>
          ))}
        </div>
      )}
    </motion.div>
  );
}

export function AdminVisitorsPage() {
  const { t } = useTranslation();
  const [visitors, setVisitors] = useState<Visitor[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [statusFilter, setStatusFilter] = useState('');
  const [showHelp, setShowHelp] = useState(false);

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const params = new URLSearchParams();
      if (search) params.set('search', search);
      if (statusFilter) params.set('status', statusFilter);
      const res = await fetch(`/api/v1/admin/visitors?${params}`).then(r => r.json());
      if (res.success) setVisitors(res.data || []);
    } catch { /* ignore */ }
    setLoading(false);
  }, [search, statusFilter]);

  useEffect(() => { loadData(); }, [loadData]);

  const stats = {
    total: visitors.length,
    pending: visitors.filter(v => v.status === 'pending').length,
    checkedIn: visitors.filter(v => v.status === 'checked_in').length,
  };

  return (
    <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white flex items-center gap-2">
            <UserPlus weight="duotone" className="w-7 h-7 text-primary-500" />
            {t('visitors.adminTitle')}
            <button onClick={() => setShowHelp(!showHelp)} className="text-surface-400 hover:text-primary-500 transition-colors" aria-label="Help">
              <Question weight="fill" className="w-5 h-5" />
            </button>
          </h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">{t('visitors.adminSubtitle')}</p>
        </div>
      </div>

      {showHelp && (
        <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} className="bg-primary-50 dark:bg-primary-900/20 border border-primary-200 dark:border-primary-800 rounded-xl p-4 text-sm text-primary-700 dark:text-primary-300">
          <p className="font-semibold mb-1">{t('visitors.aboutTitle')}</p>
          <p>{t('visitors.help')}</p>
        </motion.div>
      )}

      {/* Stats */}
      <div className="grid grid-cols-3 gap-4">
        <div className="bg-white dark:bg-surface-800 rounded-xl border border-surface-200 dark:border-surface-700 p-4 text-center">
          <div className="text-2xl font-bold text-surface-900 dark:text-white">{stats.total}</div>
          <div className="text-sm text-surface-500">{t('visitors.totalVisitors')}</div>
        </div>
        <div className="bg-white dark:bg-surface-800 rounded-xl border border-surface-200 dark:border-surface-700 p-4 text-center">
          <div className="text-2xl font-bold text-amber-600">{stats.pending}</div>
          <div className="text-sm text-surface-500">{t('visitors.status.pending')}</div>
        </div>
        <div className="bg-white dark:bg-surface-800 rounded-xl border border-surface-200 dark:border-surface-700 p-4 text-center">
          <div className="text-2xl font-bold text-green-600">{stats.checkedIn}</div>
          <div className="text-sm text-surface-500">{t('visitors.status.checked_in')}</div>
        </div>
      </div>

      {/* Filters */}
      <div className="flex gap-3">
        <div className="relative flex-1">
          <MagnifyingGlass className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
          <input type="text" value={search} onChange={e => setSearch(e.target.value)} placeholder={t('visitors.searchPlaceholder')} className="w-full pl-10 pr-4 py-2.5 rounded-xl bg-surface-50 dark:bg-surface-800 border border-surface-200 dark:border-surface-700 text-sm" />
        </div>
        <select value={statusFilter} onChange={e => setStatusFilter(e.target.value)} className="rounded-xl bg-surface-50 dark:bg-surface-800 border border-surface-200 dark:border-surface-700 px-3 py-2 text-sm">
          <option value="">{t('visitors.allStatuses')}</option>
          <option value="pending">{t('visitors.status.pending')}</option>
          <option value="checked_in">{t('visitors.status.checked_in')}</option>
          <option value="expired">{t('visitors.status.expired')}</option>
          <option value="cancelled">{t('visitors.status.cancelled')}</option>
        </select>
      </div>

      {/* Table */}
      {loading ? (
        <div className="flex justify-center py-12">
          <div className="w-8 h-8 border-2 border-primary-500 border-t-transparent rounded-full animate-spin" />
        </div>
      ) : visitors.length === 0 ? (
        <div className="text-center py-12 text-surface-400">
          <UserPlus className="w-12 h-12 mx-auto mb-3 opacity-40" />
          <p>{t('visitors.empty')}</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-surface-200 dark:border-surface-700 text-left text-surface-500">
                <th className="pb-2 font-medium">{t('visitors.name')}</th>
                <th className="pb-2 font-medium">{t('visitors.email')}</th>
                <th className="pb-2 font-medium">{t('visitors.plate')}</th>
                <th className="pb-2 font-medium">{t('visitors.date')}</th>
                <th className="pb-2 font-medium">{t('visitors.purpose')}</th>
                <th className="pb-2 font-medium">{t('common.status')}</th>
              </tr>
            </thead>
            <tbody>
              {visitors.map(v => (
                <tr key={v.id} className="border-b border-surface-100 dark:border-surface-800">
                  <td className="py-3 font-medium text-surface-900 dark:text-white">{v.name}</td>
                  <td className="py-3 text-surface-600 dark:text-surface-300">{v.email}</td>
                  <td className="py-3 text-surface-600 dark:text-surface-300">{v.vehicle_plate || '-'}</td>
                  <td className="py-3 text-surface-600 dark:text-surface-300">{new Date(v.visit_date).toLocaleString()}</td>
                  <td className="py-3 text-surface-600 dark:text-surface-300">{v.purpose || '-'}</td>
                  <td className="py-3">
                    <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${statusColors[v.status] || statusColors.pending}`}>
                      {t(`visitors.status.${v.status}`)}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </motion.div>
  );
}
