import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Plus, PencilSimple, Trash, SpinnerGap, Check, X,
  MagnifyingGlass, CurrencyEur,
} from '@phosphor-icons/react';
import { api, type ParkingLot, type CreateLotRequest, type UpdateLotRequest, type LotStatus } from '../api/client';
import toast from 'react-hot-toast';

interface LotForm {
  name: string;
  address: string;
  total_slots: number;
  hourly_rate: string;
  daily_max: string;
  monthly_pass: string;
  currency: string;
  status: LotStatus;
}

const emptyForm: LotForm = {
  name: '',
  address: '',
  total_slots: 10,
  hourly_rate: '',
  daily_max: '',
  monthly_pass: '',
  currency: 'EUR',
  status: 'open',
};

const statusConfig: Record<LotStatus, { label: string; color: string; bg: string }> = {
  open:        { label: 'Open',        color: 'text-green-600 dark:text-green-400',  bg: 'bg-green-100 dark:bg-green-900/30' },
  closed:      { label: 'Closed',      color: 'text-red-600 dark:text-red-400',      bg: 'bg-red-100 dark:bg-red-900/30' },
  full:        { label: 'Full',        color: 'text-orange-600 dark:text-orange-400', bg: 'bg-orange-100 dark:bg-orange-900/30' },
  maintenance: { label: 'Maintenance', color: 'text-amber-600 dark:text-amber-400',  bg: 'bg-amber-100 dark:bg-amber-900/30' },
};

function StatusBadge({ status }: { status: string }) {
  const cfg = statusConfig[status as LotStatus] || statusConfig.open;
  return (
    <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${cfg.bg} ${cfg.color}`}>
      {cfg.label}
    </span>
  );
}

function formatPrice(value?: number, currency?: string) {
  if (value == null) return '-';
  return new Intl.NumberFormat('en-US', { style: 'currency', currency: currency || 'EUR' }).format(value);
}

export function AdminLotsPage() {
  const [lots, setLots] = useState<ParkingLot[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<LotForm>({ ...emptyForm });
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  useEffect(() => { load(); }, []);

  async function load() {
    try {
      const res = await api.getLots();
      if (res.success && res.data) setLots(res.data);
    } finally {
      setLoading(false);
    }
  }

  const filtered = lots.filter(lot =>
    lot.name.toLowerCase().includes(search.toLowerCase()) ||
    (lot.address || '').toLowerCase().includes(search.toLowerCase())
  );

  function openCreate() {
    setEditingId(null);
    setForm({ ...emptyForm });
    setShowForm(true);
  }

  function openEdit(lot: ParkingLot) {
    setEditingId(lot.id);
    setForm({
      name: lot.name,
      address: lot.address || '',
      total_slots: lot.total_slots,
      hourly_rate: lot.hourly_rate != null ? String(lot.hourly_rate) : '',
      daily_max: lot.daily_max != null ? String(lot.daily_max) : '',
      monthly_pass: lot.monthly_pass != null ? String(lot.monthly_pass) : '',
      currency: lot.currency || 'EUR',
      status: (lot.status as LotStatus) || 'open',
    });
    setShowForm(true);
  }

  function closeForm() {
    setShowForm(false);
    setEditingId(null);
    setForm({ ...emptyForm });
  }

  function updateField<K extends keyof LotForm>(key: K, value: LotForm[K]) {
    setForm(prev => ({ ...prev, [key]: value }));
  }

  async function handleSave() {
    if (!form.name.trim()) {
      toast.error('Name is required.');
      return;
    }
    if (form.total_slots < 1) {
      toast.error('Total slots must be at least 1.');
      return;
    }

    setSaving(true);
    try {
      const payload: CreateLotRequest = {
        name: form.name.trim(),
        address: form.address.trim() || undefined,
        total_slots: form.total_slots,
        hourly_rate: form.hourly_rate ? Number(form.hourly_rate) : undefined,
        daily_max: form.daily_max ? Number(form.daily_max) : undefined,
        monthly_pass: form.monthly_pass ? Number(form.monthly_pass) : undefined,
        currency: form.currency || 'EUR',
        status: form.status,
      };

      const res = editingId
        ? await api.updateLot(editingId, payload as UpdateLotRequest)
        : await api.createLot(payload);

      if (res.success) {
        toast.success(editingId ? 'Lot updated.' : 'Lot created.');
        closeForm();
        await load();
      } else {
        toast.error(res.error?.message || 'Failed to save lot.');
      }
    } finally {
      setSaving(false);
    }
  }

  async function handleDelete(id: string) {
    if (!confirm('Delete this parking lot? All associated slots and bookings will be removed.')) return;
    setDeletingId(id);
    try {
      const res = await api.deleteLot(id);
      if (res.success) {
        setLots(prev => prev.filter(l => l.id !== id));
        toast.success('Lot deleted.');
        if (editingId === id) closeForm();
      } else {
        toast.error(res.error?.message || 'Failed to delete lot.');
      }
    } finally {
      setDeletingId(null);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" />
      </div>
    );
  }

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <h2 className="text-xl font-semibold text-surface-900 dark:text-white">Parking Lots</h2>
          <span className="text-sm text-surface-400">({lots.length})</span>
        </div>

        <div className="flex items-center gap-3">
          <div className="relative">
            <MagnifyingGlass weight="bold" className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
            <input
              type="text"
              value={search}
              onChange={e => setSearch(e.target.value)}
              placeholder="Search lots..."
              className="input pl-9 w-56"
              aria-label="Search parking lots"
            />
          </div>
          <button onClick={openCreate} className="btn btn-primary">
            <Plus weight="bold" className="w-4 h-4" />
            New Lot
          </button>
        </div>
      </div>

      {/* Create / Edit Form */}
      <AnimatePresence>
        {showForm && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="overflow-hidden"
          >
            <div className="card p-6 space-y-5">
              <div className="flex items-center justify-between">
                <h3 className="text-lg font-semibold text-surface-900 dark:text-white">
                  {editingId ? 'Edit Parking Lot' : 'New Parking Lot'}
                </h3>
                <button onClick={closeForm} className="p-1.5 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors">
                  <X weight="bold" className="w-5 h-5 text-surface-400" />
                </button>
              </div>

              {/* Row 1: Name + Address */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-5">
                <div>
                  <label htmlFor="lot-name" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">Name *</label>
                  <input
                    id="lot-name"
                    type="text"
                    value={form.name}
                    onChange={e => updateField('name', e.target.value)}
                    className="input"
                    placeholder="Main Office Garage"
                  />
                </div>
                <div>
                  <label htmlFor="lot-address" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">Address</label>
                  <input
                    id="lot-address"
                    type="text"
                    value={form.address}
                    onChange={e => updateField('address', e.target.value)}
                    className="input"
                    placeholder="123 Main Street"
                  />
                </div>
              </div>

              {/* Row 2: Slots + Status + Currency */}
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-5">
                <div>
                  <label htmlFor="lot-total-slots" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">Total Slots *</label>
                  <input
                    id="lot-total-slots"
                    type="number"
                    min={1}
                    max={1000}
                    value={form.total_slots}
                    onChange={e => updateField('total_slots', Math.max(1, Number(e.target.value)))}
                    className="input"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">Status</label>
                  <div className="flex flex-wrap gap-2">
                    {(Object.keys(statusConfig) as LotStatus[]).map(s => {
                      const cfg = statusConfig[s];
                      const isSelected = form.status === s;
                      return (
                        <button
                          key={s}
                          type="button"
                          onClick={() => updateField('status', s)}
                          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium border-2 transition-all ${
                            isSelected
                              ? `${cfg.bg} ${cfg.color} border-current`
                              : 'border-surface-200 dark:border-surface-700 text-surface-500 dark:text-surface-400 hover:border-surface-300 dark:hover:border-surface-600'
                          }`}
                        >
                          {cfg.label}
                        </button>
                      );
                    })}
                  </div>
                </div>
                <div>
                  <label htmlFor="lot-currency" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">Currency</label>
                  <select
                    id="lot-currency"
                    value={form.currency}
                    onChange={e => updateField('currency', e.target.value)}
                    className="input"
                  >
                    <option value="EUR">EUR</option>
                    <option value="USD">USD</option>
                    <option value="GBP">GBP</option>
                    <option value="CHF">CHF</option>
                  </select>
                </div>
              </div>

              {/* Row 3: Pricing */}
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-5">
                <div>
                  <label htmlFor="lot-hourly-rate" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">Hourly Rate</label>
                  <div className="relative">
                    <CurrencyEur weight="bold" className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
                    <input
                      id="lot-hourly-rate"
                      type="number"
                      min={0}
                      step="0.01"
                      value={form.hourly_rate}
                      onChange={e => updateField('hourly_rate', e.target.value)}
                      className="input pl-9"
                      placeholder="2.50"
                    />
                  </div>
                </div>
                <div>
                  <label htmlFor="lot-daily-max" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">Daily Max</label>
                  <div className="relative">
                    <CurrencyEur weight="bold" className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
                    <input
                      id="lot-daily-max"
                      type="number"
                      min={0}
                      step="0.01"
                      value={form.daily_max}
                      onChange={e => updateField('daily_max', e.target.value)}
                      className="input pl-9"
                      placeholder="15.00"
                    />
                  </div>
                </div>
                <div>
                  <label htmlFor="lot-monthly-pass" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">Monthly Pass</label>
                  <div className="relative">
                    <CurrencyEur weight="bold" className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
                    <input
                      id="lot-monthly-pass"
                      type="number"
                      min={0}
                      step="0.01"
                      value={form.monthly_pass}
                      onChange={e => updateField('monthly_pass', e.target.value)}
                      className="input pl-9"
                      placeholder="200.00"
                    />
                  </div>
                </div>
              </div>

              {/* Actions */}
              <div className="flex gap-3 pt-2">
                <button onClick={handleSave} disabled={saving} className="btn btn-primary">
                  {saving
                    ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" />
                    : <Check weight="bold" className="w-4 h-4" />}
                  {editingId ? 'Save' : 'Create'}
                </button>
                <button onClick={closeForm} className="btn btn-secondary">Cancel</button>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Lots Table */}
      <div className="card overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-surface-200 dark:border-surface-700">
                <th className="text-left px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">Lot</th>
                <th className="text-left px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">Slots</th>
                <th className="text-left px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">Status</th>
                <th className="text-left px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">Pricing</th>
                <th className="text-right px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-surface-100 dark:divide-surface-800">
              {filtered.map((lot, i) => (
                <motion.tr
                  key={lot.id}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  transition={{ delay: i * 0.02 }}
                  className="hover:bg-surface-50 dark:hover:bg-surface-800/50 transition-colors"
                >
                  <td className="px-5 py-4">
                    <div className="min-w-0">
                      <p className="text-sm font-medium text-surface-900 dark:text-white truncate">{lot.name}</p>
                      {lot.address && (
                        <p className="text-xs text-surface-500 dark:text-surface-400 truncate">{lot.address}</p>
                      )}
                    </div>
                  </td>
                  <td className="px-5 py-4">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-semibold text-surface-900 dark:text-white">{lot.available_slots}</span>
                      <span className="text-xs text-surface-400">/ {lot.total_slots}</span>
                    </div>
                    <div className="w-20 h-1.5 bg-surface-200 dark:bg-surface-700 rounded-full mt-1.5 overflow-hidden">
                      <div
                        className="h-full rounded-full transition-all bg-primary-500"
                        style={{ width: `${lot.total_slots > 0 ? Math.round((lot.available_slots / lot.total_slots) * 100) : 0}%` }}
                      />
                    </div>
                  </td>
                  <td className="px-5 py-4">
                    <StatusBadge status={lot.status} />
                  </td>
                  <td className="px-5 py-4">
                    <div className="space-y-0.5 text-xs text-surface-600 dark:text-surface-400">
                      <p>Hourly: {formatPrice(lot.hourly_rate, lot.currency)}</p>
                      <p>Daily max: {formatPrice(lot.daily_max, lot.currency)}</p>
                      <p>Monthly: {formatPrice(lot.monthly_pass, lot.currency)}</p>
                    </div>
                  </td>
                  <td className="px-5 py-4">
                    <div className="flex items-center justify-end gap-1">
                      <button
                        onClick={() => openEdit(lot)}
                        className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors text-surface-400 hover:text-primary-600"
                        title="Edit lot"
                      >
                        <PencilSimple weight="bold" className="w-4 h-4" />
                      </button>
                      <button
                        onClick={() => handleDelete(lot.id)}
                        disabled={deletingId === lot.id}
                        className="p-2 rounded-lg hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors text-surface-400 hover:text-red-600 disabled:opacity-50"
                        title="Delete lot"
                      >
                        {deletingId === lot.id
                          ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" />
                          : <Trash weight="bold" className="w-4 h-4" />}
                      </button>
                    </div>
                  </td>
                </motion.tr>
              ))}
            </tbody>
          </table>
        </div>

        {filtered.length === 0 && (
          <div className="p-8 text-center">
            <p className="text-sm text-surface-500 dark:text-surface-400">
              {search ? 'No lots match your search.' : 'No parking lots yet. Create one to get started.'}
            </p>
          </div>
        )}
      </div>
    </motion.div>
  );
}
