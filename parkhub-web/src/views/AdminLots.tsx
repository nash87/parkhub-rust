import { useState, useEffect, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Plus, PencilSimple, Trash, SpinnerGap, Check, X,
  MagnifyingGlass, CurrencyEur, TrendUp, Clock,
} from '@phosphor-icons/react';
import { api, type ParkingLot, type CreateLotRequest, type UpdateLotRequest, type LotStatus, type DynamicPricingRules, type OperatingHoursData, type DayHoursData } from '../api/client';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';

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

export function AdminLotsPage() {
  const { t } = useTranslation();
  const [lots, setLots] = useState<ParkingLot[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<LotForm>({ ...emptyForm });
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [confirmState, setConfirmState] = useState<{open: boolean, action: () => void}>({open: false, action: () => {}});
  const [dynamicPricing, setDynamicPricing] = useState<DynamicPricingRules>({
    enabled: false, base_price: 2.50, surge_multiplier: 1.5,
    discount_multiplier: 0.8, surge_threshold: 80, discount_threshold: 20,
  });
  const defaultDayHours: DayHoursData = { open: '07:00', close: '22:00', closed: false };
  const [opHours, setOpHours] = useState<OperatingHoursData>({
    is_24h: true,
    monday: { ...defaultDayHours }, tuesday: { ...defaultDayHours },
    wednesday: { ...defaultDayHours }, thursday: { ...defaultDayHours },
    friday: { ...defaultDayHours },
    saturday: { open: '09:00', close: '18:00', closed: false },
    sunday: { open: '09:00', close: '18:00', closed: true },
  });

  const statusConfig = useMemo<Record<LotStatus, { label: string; color: string; bg: string }>>(() => ({
    open:        { label: t('admin.statusOpen'),        color: 'text-green-600 dark:text-green-400',  bg: 'bg-green-100 dark:bg-green-900/30' },
    closed:      { label: t('admin.statusClosed'),      color: 'text-red-600 dark:text-red-400',      bg: 'bg-red-100 dark:bg-red-900/30' },
    full:        { label: t('admin.statusFull'),        color: 'text-orange-600 dark:text-orange-400', bg: 'bg-orange-100 dark:bg-orange-900/30' },
    maintenance: { label: t('admin.statusMaintenance'), color: 'text-amber-600 dark:text-amber-400',  bg: 'bg-amber-100 dark:bg-amber-900/30' },
  }), [t]);

  useEffect(() => { load(); }, []);

  async function load() {
    try {
      const res = await api.getLots();
      if (res.success && res.data) setLots(res.data);
    } finally {
      setLoading(false);
    }
  }

  const filtered = useMemo(() => lots.filter(lot =>
    lot.name.toLowerCase().includes(search.toLowerCase()) ||
    (lot.address || '').toLowerCase().includes(search.toLowerCase())
  ), [lots, search]);

  function openCreate() {
    setEditingId(null);
    setForm({ ...emptyForm });
    setShowForm(true);
  }

  async function openEdit(lot: ParkingLot) {
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
    // Fetch dynamic pricing rules and operating hours
    const [dpRes, ohRes] = await Promise.all([
      api.getAdminDynamicPricing(lot.id),
      api.getLotHours(lot.id),
    ]);
    if (dpRes.success && dpRes.data) {
      setDynamicPricing(dpRes.data);
    } else {
      setDynamicPricing({
        enabled: false, base_price: 2.50, surge_multiplier: 1.5,
        discount_multiplier: 0.8, surge_threshold: 80, discount_threshold: 20,
      });
    }
    if (ohRes.success && ohRes.data) {
      setOpHours(ohRes.data);
    }
  }

  function closeForm() {
    setShowForm(false);
    setEditingId(null);
    setForm({ ...emptyForm });
  }

  function updateField<K extends keyof LotForm>(key: K, value: LotForm[K]) {
    setForm(prev => ({ ...prev, [key]: value }));
  }

  function formatPrice(value?: number, currency?: string) {
    if (value == null) return '-';
    return new Intl.NumberFormat('en-US', { style: 'currency', currency: currency || 'EUR' }).format(value);
  }

  async function handleSave() {
    if (!form.name.trim()) {
      toast.error(t('admin.lotNameRequired'));
      return;
    }
    if (form.total_slots < 1) {
      toast.error(t('admin.lotSlotsMin'));
      return;
    }
    if ((form.hourly_rate && Number(form.hourly_rate) < 0) ||
        (form.daily_max && Number(form.daily_max) < 0) ||
        (form.monthly_pass && Number(form.monthly_pass) < 0)) {
      toast.error(t('admin.lotSaveFailed'));
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
        // Save dynamic pricing rules and operating hours if editing
        if (editingId) {
          const [dpRes, ohRes] = await Promise.all([
            api.updateAdminDynamicPricing(editingId, dynamicPricing),
            api.updateAdminLotHours(editingId, opHours),
          ]);
          if (!dpRes.success) {
            toast.error(t('admin.dynamicPricingSaveFailed'));
          }
          if (!ohRes.success) {
            toast.error(t('admin.operatingHoursSaveFailed'));
          }
        }
        toast.success(editingId ? t('admin.lotUpdated') : t('admin.lotCreated'));
        closeForm();
        await load();
      } else {
        toast.error(res.error?.message || t('admin.lotSaveFailed'));
      }
    } finally {
      setSaving(false);
    }
  }

  function handleDelete(id: string) {
    setConfirmState({
      open: true,
      action: async () => {
        setConfirmState({open: false, action: () => {}});
        setDeletingId(id);
        try {
          const res = await api.deleteLot(id);
          if (res.success) {
            setLots(prev => prev.filter(l => l.id !== id));
            toast.success(t('admin.lotDeleted'));
            if (editingId === id) closeForm();
          } else {
            toast.error(res.error?.message || t('admin.lotDeleteFailed'));
          }
        } finally {
          setDeletingId(null);
        }
      },
    });
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64" role="status" aria-label={t('common.loading')}>
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" aria-hidden="true" />
      </div>
    );
  }

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <h2 className="text-xl font-semibold text-surface-900 dark:text-white">{t('admin.lots')}</h2>
          <span className="text-sm text-surface-500 dark:text-surface-400">({lots.length})</span>
        </div>

        <div className="flex flex-col sm:flex-row items-stretch sm:items-center gap-3">
          <div className="relative">
            <MagnifyingGlass weight="bold" className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
            <input
              type="text"
              value={search}
              onChange={e => setSearch(e.target.value)}
              placeholder={t('admin.searchLots')}
              className="input pl-9 w-full sm:w-56"
              aria-label={t('admin.searchLots')}
            />
          </div>
          <button onClick={openCreate} className="btn btn-primary self-start sm:self-auto">
            <Plus weight="bold" className="w-4 h-4" />
            {t('admin.newLot')}
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
                  {editingId ? t('admin.editLot') : t('admin.newLot')}
                </h3>
                <button onClick={closeForm} className="p-1.5 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors" aria-label={t('common.close')}>
                  <X weight="bold" className="w-5 h-5 text-surface-400" aria-hidden="true" />
                </button>
              </div>

              {/* Row 1: Name + Address */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-5">
                <div>
                  <label htmlFor="lot-name" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">{t('admin.lotName')} *</label>
                  <input
                    id="lot-name"
                    type="text"
                    value={form.name}
                    onChange={e => updateField('name', e.target.value)}
                    className="input"
                  />
                </div>
                <div>
                  <label htmlFor="lot-address" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">{t('admin.lotAddress')}</label>
                  <input
                    id="lot-address"
                    type="text"
                    value={form.address}
                    onChange={e => updateField('address', e.target.value)}
                    className="input"
                  />
                </div>
              </div>

              {/* Row 2: Slots + Status + Currency */}
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-5">
                <div>
                  <label htmlFor="lot-total-slots" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">{t('admin.totalSlots')} *</label>
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
                  <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">{t('admin.status')}</label>
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
                  <label htmlFor="lot-currency" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">{t('admin.currency')}</label>
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
                  <label htmlFor="lot-hourly-rate" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">{t('admin.hourlyRate')}</label>
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
                  <label htmlFor="lot-daily-max" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">{t('admin.dailyMax')}</label>
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
                  <label htmlFor="lot-monthly-pass" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">{t('admin.monthlyPass')}</label>
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

              {/* Row 4: Dynamic Pricing (only when editing) */}
              {editingId && (
                <div className="border-t border-surface-200 dark:border-surface-700 pt-5 space-y-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <h4 className="text-sm font-semibold text-surface-900 dark:text-white flex items-center gap-2">
                        <TrendUp weight="bold" className="w-4 h-4 text-primary-600" />
                        {t('admin.dynamicPricing')}
                      </h4>
                      <p className="text-xs text-surface-500 dark:text-surface-400 mt-0.5">{t('admin.dynamicPricingDesc')}</p>
                    </div>
                    <label className="relative inline-flex items-center cursor-pointer">
                      <input type="checkbox" checked={dynamicPricing.enabled}
                        onChange={e => setDynamicPricing(prev => ({ ...prev, enabled: e.target.checked }))}
                        className="sr-only peer" />
                      <div className="w-10 h-5 bg-surface-300 dark:bg-surface-600 peer-checked:bg-primary-600 rounded-full transition-colors after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-transform peer-checked:after:translate-x-5" />
                    </label>
                  </div>
                  {dynamicPricing.enabled && (
                    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
                      <div>
                        <label htmlFor="dp-base-price" className="block text-xs font-medium text-surface-700 dark:text-surface-300 mb-1">{t('admin.basePrice')}</label>
                        <input id="dp-base-price" type="number" min={0} step="0.01"
                          value={dynamicPricing.base_price} onChange={e => setDynamicPricing(prev => ({ ...prev, base_price: Number(e.target.value) }))}
                          className="input text-sm" />
                      </div>
                      <div>
                        <label htmlFor="dp-surge-mult" className="block text-xs font-medium text-surface-700 dark:text-surface-300 mb-1">
                          {t('admin.surgeMultiplier')}
                          <span className="block text-[10px] text-surface-400 font-normal">{t('admin.surgeMultiplierDesc')}</span>
                        </label>
                        <input id="dp-surge-mult" type="number" min={1} step="0.1"
                          value={dynamicPricing.surge_multiplier} onChange={e => setDynamicPricing(prev => ({ ...prev, surge_multiplier: Number(e.target.value) }))}
                          className="input text-sm" />
                      </div>
                      <div>
                        <label htmlFor="dp-discount-mult" className="block text-xs font-medium text-surface-700 dark:text-surface-300 mb-1">
                          {t('admin.discountMultiplier')}
                          <span className="block text-[10px] text-surface-400 font-normal">{t('admin.discountMultiplierDesc')}</span>
                        </label>
                        <input id="dp-discount-mult" type="number" min={0.01} max={1} step="0.05"
                          value={dynamicPricing.discount_multiplier} onChange={e => setDynamicPricing(prev => ({ ...prev, discount_multiplier: Number(e.target.value) }))}
                          className="input text-sm" />
                      </div>
                      <div>
                        <label htmlFor="dp-surge-thresh" className="block text-xs font-medium text-surface-700 dark:text-surface-300 mb-1">
                          {t('admin.surgeThreshold')}
                        </label>
                        <input id="dp-surge-thresh" type="number" min={0} max={100} step={5}
                          value={dynamicPricing.surge_threshold} onChange={e => setDynamicPricing(prev => ({ ...prev, surge_threshold: Number(e.target.value) }))}
                          className="input text-sm" />
                      </div>
                      <div>
                        <label htmlFor="dp-discount-thresh" className="block text-xs font-medium text-surface-700 dark:text-surface-300 mb-1">
                          {t('admin.discountThreshold')}
                        </label>
                        <input id="dp-discount-thresh" type="number" min={0} max={100} step={5}
                          value={dynamicPricing.discount_threshold} onChange={e => setDynamicPricing(prev => ({ ...prev, discount_threshold: Number(e.target.value) }))}
                          className="input text-sm" />
                      </div>
                    </div>
                  )}
                </div>
              )}

              {/* Row 5: Operating Hours (only when editing) */}
              {editingId && (
                <div className="border-t border-surface-200 dark:border-surface-700 pt-5 space-y-4">
                  <div className="flex items-center justify-between">
                    <div>
                      <h4 className="text-sm font-semibold text-surface-900 dark:text-white flex items-center gap-2">
                        <Clock weight="bold" className="w-4 h-4 text-primary-600" />
                        {t('admin.operatingHours')}
                      </h4>
                      <p className="text-xs text-surface-500 dark:text-surface-400 mt-0.5">{t('admin.operatingHoursDesc')}</p>
                    </div>
                    <label className="relative inline-flex items-center cursor-pointer">
                      <input type="checkbox" checked={opHours.is_24h}
                        onChange={e => setOpHours(prev => ({ ...prev, is_24h: e.target.checked }))}
                        className="sr-only peer" />
                      <div className="w-10 h-5 bg-surface-300 dark:bg-surface-600 peer-checked:bg-primary-600 rounded-full transition-colors after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-transform peer-checked:after:translate-x-5" />
                      <span className="ml-2 text-xs text-surface-600 dark:text-surface-400">{t('admin.is24h')}</span>
                    </label>
                  </div>
                  {!opHours.is_24h && (
                    <div className="space-y-2">
                      {(['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday'] as const).map(day => {
                        const dh = (opHours[day] as DayHoursData | undefined) || { open: '07:00', close: '22:00', closed: false };
                        return (
                          <div key={day} className="flex items-center gap-3 text-sm">
                            <span className="w-24 font-medium text-surface-700 dark:text-surface-300">{t(`admin.${day}`)}</span>
                            <label className="flex items-center gap-1.5 cursor-pointer">
                              <input type="checkbox" checked={dh.closed}
                                onChange={e => setOpHours(prev => ({ ...prev, [day]: { ...dh, closed: e.target.checked } }))}
                                className="w-4 h-4 rounded border-surface-300 text-red-600 focus:ring-red-500" />
                              <span className="text-xs text-surface-500">{t('admin.closedDay')}</span>
                            </label>
                            {!dh.closed && (
                              <>
                                <input type="time" value={dh.open}
                                  onChange={e => setOpHours(prev => ({ ...prev, [day]: { ...dh, open: e.target.value } }))}
                                  className="input text-xs w-28 py-1" />
                                <span className="text-surface-400">-</span>
                                <input type="time" value={dh.close}
                                  onChange={e => setOpHours(prev => ({ ...prev, [day]: { ...dh, close: e.target.value } }))}
                                  className="input text-xs w-28 py-1" />
                              </>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>
              )}

              {/* Actions */}
              <div className="flex gap-3 pt-2">
                <button onClick={handleSave} disabled={saving} className="btn btn-primary">
                  {saving
                    ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" />
                    : <Check weight="bold" className="w-4 h-4" />}
                  {editingId ? t('common.save') : t('admin.create')}
                </button>
                <button onClick={closeForm} className="btn btn-secondary">{t('common.cancel')}</button>
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
                <th className="text-left px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">{t('admin.lots')}</th>
                <th className="text-left px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">{t('admin.totalSlots')}</th>
                <th className="text-left px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">{t('admin.status')}</th>
                <th className="text-left px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">{t('admin.pricing')}</th>
                <th className="text-right px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider"></th>
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
                      <span className="text-xs text-surface-500 dark:text-surface-400">/ {lot.total_slots}</span>
                    </div>
                    <div className="w-20 h-1.5 bg-surface-200 dark:bg-surface-700 rounded-full mt-1.5 overflow-hidden">
                      <div
                        className="h-full rounded-full transition-all bg-primary-500"
                        style={{ width: `${lot.total_slots > 0 ? Math.round((lot.available_slots / lot.total_slots) * 100) : 0}%` }}
                      />
                    </div>
                  </td>
                  <td className="px-5 py-4">
                    {(() => {
                      const cfg = statusConfig[lot.status as LotStatus] || statusConfig.open;
                      return (
                        <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${cfg.bg} ${cfg.color}`}>
                          {cfg.label}
                        </span>
                      );
                    })()}
                  </td>
                  <td className="px-5 py-4">
                    <div className="space-y-0.5 text-xs text-surface-600 dark:text-surface-400">
                      <p>{t('admin.hourlyRate')}: {formatPrice(lot.hourly_rate, lot.currency)}</p>
                      <p>{t('admin.dailyMax')}: {formatPrice(lot.daily_max, lot.currency)}</p>
                      <p>{t('admin.monthlyPass')}: {formatPrice(lot.monthly_pass, lot.currency)}</p>
                    </div>
                  </td>
                  <td className="px-5 py-4">
                    <div className="flex items-center justify-end gap-1">
                      <button
                        onClick={() => openEdit(lot)}
                        className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors text-surface-400 hover:text-primary-600"
                        aria-label={`${t('admin.editLotBtn')} ${lot.name}`}
                      >
                        <PencilSimple weight="bold" className="w-4 h-4" aria-hidden="true" />
                      </button>
                      <button
                        onClick={() => handleDelete(lot.id)}
                        disabled={deletingId === lot.id}
                        className="p-2 rounded-lg hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors text-surface-400 hover:text-red-600 disabled:opacity-50"
                        aria-label={`${t('admin.deleteLotBtn')} ${lot.name}`}
                      >
                        {deletingId === lot.id
                          ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" />
                          : <Trash weight="bold" className="w-4 h-4" aria-hidden="true" />}
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
              {search ? t('admin.noLotsMatch') : t('admin.noLotsYet')}
            </p>
          </div>
        )}
      </div>
      <ConfirmDialog
        open={confirmState.open}
        title={t('common.delete')}
        message={t('admin.lotDeleteConfirm')}
        variant="danger"
        onConfirm={confirmState.action}
        onCancel={() => setConfirmState({open: false, action: () => {}})}
      />
    </motion.div>
  );
}
