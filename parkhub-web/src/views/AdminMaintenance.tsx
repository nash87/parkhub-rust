import { useEffect, useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Wrench, Plus, Trash, PencilSimple, Question, CalendarBlank, Warning } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

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
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="p-2 bg-amber-100 dark:bg-amber-900/30 rounded-lg">
            <Wrench weight="bold" className="w-5 h-5 text-amber-600 dark:text-amber-400" />
          </div>
          <div>
            <h2 className="text-lg font-bold text-surface-900 dark:text-white">{t('maintenance.title', 'Maintenance Scheduling')}</h2>
            <p className="text-sm text-surface-500 dark:text-surface-400">{t('maintenance.subtitle', 'Schedule and manage maintenance windows')}</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={() => setShowHelp(!showHelp)} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-400">
            <Question weight="bold" className="w-5 h-5" />
          </button>
          <button onClick={openCreate} className="btn btn-primary btn-sm" data-testid="create-btn">
            <Plus weight="bold" className="w-4 h-4" /> {t('maintenance.create', 'New')}
          </button>
        </div>
      </div>

      {/* Help */}
      {showHelp && (
        <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} className="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-xl p-4">
          <p className="text-sm text-amber-800 dark:text-amber-300">
            {t('maintenance.help', 'Schedule maintenance windows to temporarily block parking slots. Bookings that overlap with maintenance are rejected. Users with affected bookings will be notified.')}
          </p>
        </motion.div>
      )}

      {/* Active banner */}
      {activeCount > 0 && (
        <div className="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-xl px-4 py-3 flex items-center gap-2" data-testid="active-banner">
          <Warning weight="bold" className="w-5 h-5 text-amber-600" />
          <p className="text-sm text-amber-800 dark:text-amber-300 font-medium">
            {t('maintenance.activeBanner', '{{count}} maintenance window(s) currently active', { count: activeCount })}
          </p>
        </div>
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
            <CalendarBlank weight="thin" className="w-12 h-12 mx-auto text-surface-300 dark:text-surface-600 mb-3" />
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
                  <button onClick={() => openEdit(w)} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-400 hover:text-primary-600">
                    <PencilSimple weight="bold" className="w-4 h-4" />
                  </button>
                  <button onClick={() => handleDelete(w.id)} className="p-2 rounded-lg hover:bg-red-50 dark:hover:bg-red-900/20 text-surface-400 hover:text-red-600">
                    <Trash weight="bold" className="w-4 h-4" />
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
