import { useEffect, useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { WrenchIcon, PlusIcon, TrashIcon, PencilSimpleIcon, QuestionIcon, CalendarBlankIcon, WarningIcon } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';
import { HeroEyebrow } from '../components/v11/HeroEyebrow';

interface MaintenanceWindow {
  id: string;
  lot_id: string;
  lot_name?: string;
  start_time: string;
  end_time: string;
  reason: string;
  affected_slots: { type: 'all' } | { type: 'specific'; slot_ids: string[] };
  created_at: string;
}

interface Lot {
  id: string;
  name: string;
}

interface FormData {
  lot_id: string;
  start_time: string;
  end_time: string;
  reason: string;
  all_slots: boolean;
  slot_ids: string;
}

const emptyForm: FormData = {
  lot_id: '',
  start_time: '',
  end_time: '',
  reason: '',
  all_slots: true,
  slot_ids: '',
};

export function AdminMaintenancePage() {
  const { t } = useTranslation();
  const [windows, setWindows] = useState<MaintenanceWindow[]>([]);
  const [lots, setLots] = useState<Lot[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);
  const [form, setForm] = useState<FormData>(emptyForm);
  const [submitting, setSubmitting] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const [activeCount, setActiveCount] = useState(0);

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const [mainRes, lotsRes, activeRes] = await Promise.all([
        fetch('/api/v1/admin/maintenance').then(r => r.json()),
        fetch('/api/v1/lots').then(r => r.json()),
        fetch('/api/v1/maintenance/active').then(r => r.json()),
      ]);
      if (mainRes.success) setWindows(mainRes.data || []);
      if (lotsRes.success) setLots(lotsRes.data || []);
      if (activeRes.success) setActiveCount((activeRes.data || []).length);
    } catch { /* ignore */ }
    setLoading(false);
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  function openCreate() {
    setForm(emptyForm);
    setEditId(null);
    setShowForm(true);
  }

  function openEdit(w: MaintenanceWindow) {
    setForm({
      lot_id: w.lot_id,
      start_time: w.start_time.slice(0, 16),
      end_time: w.end_time.slice(0, 16),
      reason: w.reason,
      all_slots: w.affected_slots.type === 'all',
      slot_ids: w.affected_slots.type === 'specific' ? w.affected_slots.slot_ids.join(', ') : '',
    });
    setEditId(w.id);
    setShowForm(true);
  }

  async function handleSubmit() {
    if (!form.lot_id || !form.start_time || !form.end_time || !form.reason.trim()) {
      toast.error(t('maintenance.requiredFields', 'Please fill all required fields'));
      return;
    }
    setSubmitting(true);
    try {
      const body: any = {
        lot_id: form.lot_id,
        start_time: new Date(form.start_time).toISOString(),
        end_time: new Date(form.end_time).toISOString(),
        reason: form.reason,
      };
      if (!form.all_slots && form.slot_ids.trim()) {
        body.affected_slots = form.slot_ids.split(',').map(s => s.trim()).filter(Boolean);
      }

      const url = editId ? `/api/v1/admin/maintenance/${editId}` : '/api/v1/admin/maintenance';
      const method = editId ? 'PUT' : 'POST';

      const res = await fetch(url, {
        method,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      }).then(r => r.json());

      if (res.success) {
        toast.success(editId ? t('maintenance.updated', 'Updated') : t('maintenance.created', 'Created'));
        setShowForm(false);
        loadData();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } catch { toast.error(t('common.error')); }
    setSubmitting(false);
  }

  async function handleDelete(id: string) {
    try {
      const res = await fetch(`/api/v1/admin/maintenance/${id}`, { method: 'DELETE' }).then(r => r.json());
      if (res.success) {
        toast.success(t('maintenance.deleted', 'Cancelled'));
        loadData();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } catch { toast.error(t('common.error')); }
  }

  const now = new Date().toISOString();

  if (loading) {
    return (
      <div className="space-y-4">
        {Array.from({ length: 3 }, (_, i) => <div key={i} className="h-24 skeleton rounded-xl" />)}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* v11 SOTA hero — amber tone (caution = maintenance). */}
      <section className="admin-hero admin-hero--amber">
        <div className="admin-hero-left">
          <HeroEyebrow icon={WrenchIcon} label={t('maintenance.eyebrow', 'MAINTENANCE')} />
          <h1 className="admin-hero-headline">{t('maintenance.title', 'Maintenance Scheduling')}</h1>
          <p className="admin-hero-sub">{t('maintenance.subtitle', 'Schedule and manage maintenance windows')}</p>
        </div>
        <div className="admin-hero-actions">
          <button
            onClick={() => setShowHelp(!showHelp)}
            className="admin-hero-iconbtn"
            aria-label={t('common.help', 'Help')}
          >
            <QuestionIcon weight="bold" className="w-4 h-4" />
          </button>
          <button onClick={openCreate} className="admin-hero-action" data-testid="create-btn">
            <PlusIcon weight="bold" className="w-4 h-4" />
            {t('maintenance.create', 'New')}
          </button>
        </div>
      </section>

      {/* Help */}
      {showHelp && (
        <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} className="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-xl p-4">
          <p className="text-sm text-amber-800 dark:text-amber-300">
            {t('maintenance.help', 'Schedule maintenance windows to temporarily block parking slots. Bookings that overlap with maintenance are rejected. Users with affected bookings will be notified.')}
          </p>
        </motion.div>
      )}

      {/* v11 SOTA warn banner — pulses gently to keep the active state
          present in peripheral vision. */}
      {activeCount > 0 && (
        <motion.div
          initial={{ opacity: 0, y: -8 }}
          animate={{ opacity: 1, y: 0 }}
          className="v11-warn-banner"
          data-testid="active-banner"
        >
          <WarningIcon weight="bold" className="v11-warn-banner-icon" />
          <p className="v11-warn-banner-text">
            {t('maintenance.activeBanner', '{{count}} maintenance window(s) currently active', { count: activeCount })}
          </p>
        </motion.div>
      )}

      {/* Form */}
      {showForm && (
        <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-5 space-y-4" data-testid="maintenance-form">
          <h3 className="font-semibold text-surface-900 dark:text-white">
            {editId ? t('maintenance.editTitle', 'Edit Maintenance') : t('maintenance.createTitle', 'New Maintenance Window')}
          </h3>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('maintenance.lot', 'Lot')}</label>
              <select className="input" value={form.lot_id} onChange={e => setForm({ ...form, lot_id: e.target.value })} data-testid="form-lot">
                <option value="">{t('maintenance.selectLot', 'Select lot...')}</option>
                {lots.map(l => <option key={l.id} value={l.id}>{l.name}</option>)}
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('maintenance.reason', 'Reason')}</label>
              <input className="input" value={form.reason} onChange={e => setForm({ ...form, reason: e.target.value })} data-testid="form-reason" />
            </div>
            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('maintenance.start', 'Start')}</label>
              <input type="datetime-local" className="input" value={form.start_time} onChange={e => setForm({ ...form, start_time: e.target.value })} data-testid="form-start" />
            </div>
            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('maintenance.end', 'End')}</label>
              <input type="datetime-local" className="input" value={form.end_time} onChange={e => setForm({ ...form, end_time: e.target.value })} data-testid="form-end" />
            </div>
          </div>
          <div className="flex items-center gap-3">
            <label className="flex items-center gap-2 text-sm text-surface-700 dark:text-surface-300">
              <input type="checkbox" checked={form.all_slots} onChange={e => setForm({ ...form, all_slots: e.target.checked })} />
              {t('maintenance.allSlots', 'All slots')}
            </label>
          </div>
          {!form.all_slots && (
            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('maintenance.specificSlots', 'Slot IDs (comma-separated)')}</label>
              <input className="input" value={form.slot_ids} onChange={e => setForm({ ...form, slot_ids: e.target.value })} placeholder="s1, s2, s3" />
            </div>
          )}
          <div className="flex gap-2">
            <button onClick={handleSubmit} disabled={submitting} className="btn btn-primary btn-sm" data-testid="form-submit">
              {editId ? t('common.save', 'Save') : t('maintenance.create', 'Create')}
            </button>
            <button onClick={() => setShowForm(false)} className="btn btn-secondary btn-sm">{t('common.cancel', 'Cancel')}</button>
          </div>
        </motion.div>
      )}

      {/* Calendar / list view */}
      <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 divide-y divide-surface-100 dark:divide-surface-800" data-testid="maintenance-list">
        {windows.length === 0 ? (
          <div className="p-8 text-center">
            <CalendarBlankIcon weight="thin" className="w-12 h-12 mx-auto text-surface-300 dark:text-surface-600 mb-3" />
            <p className="text-sm text-surface-500 dark:text-surface-400">{t('maintenance.empty', 'No maintenance windows scheduled')}</p>
          </div>
        ) : (
          windows.map(w => {
            const isActive = w.start_time <= now && w.end_time > now;
            const isPast = w.end_time <= now;
            return (
              <div key={w.id} className={`p-4 flex items-center justify-between ${isPast ? 'opacity-50' : ''}`} data-testid="maintenance-row">
                <div className="flex items-center gap-3 min-w-0">
                  <div className={`w-2 h-2 rounded-full flex-shrink-0 ${isActive ? 'bg-amber-500 animate-pulse' : isPast ? 'bg-surface-300' : 'bg-blue-500'}`} />
                  <div className="min-w-0">
                    <p className="text-sm font-medium text-surface-900 dark:text-white truncate">
                      {w.lot_name || w.lot_id} — {w.reason}
                    </p>
                    <p className="text-xs text-surface-500 dark:text-surface-400">
                      {new Date(w.start_time).toLocaleString()} — {new Date(w.end_time).toLocaleString()}
                      {w.affected_slots.type === 'all' ? ` (${t('maintenance.allSlots', 'all slots')})` : ` (${(w.affected_slots as any).slot_ids?.length || 0} slots)`}
                    </p>
                  </div>
                </div>
                <div className="flex items-center gap-1 flex-shrink-0">
                  <button onClick={() => openEdit(w)} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-400 hover:text-primary-600" aria-label={`${t('common.edit', 'Edit')} ${w.reason}`} title={t('common.edit', 'Edit')}>
                    <PencilSimpleIcon weight="bold" className="w-4 h-4" />
                  </button>
                  <button onClick={() => handleDelete(w.id)} className="p-2 rounded-lg hover:bg-red-50 dark:hover:bg-red-900/20 text-surface-400 hover:text-red-600" aria-label={`${t('common.delete', 'Delete')} ${w.reason}`} title={t('common.delete', 'Delete')}>
                    <TrashIcon weight="bold" className="w-4 h-4" />
                  </button>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
