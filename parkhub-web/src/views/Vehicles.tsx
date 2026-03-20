import { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Car, Plus, Trash, Star, X, SpinnerGap } from '@phosphor-icons/react';
import { api, type Vehicle } from '../api/client';
import { VehiclesSkeleton } from '../components/Skeleton';
import { stagger, fadeUp } from '../constants/animations';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

const COLOR_OPTIONS = [
  { value: 'Schwarz', bg: 'bg-gray-900' },
  { value: 'Wei\u00df', bg: 'bg-white border border-surface-300' },
  { value: 'Silber', bg: 'bg-gray-400' },
  { value: 'Grau', bg: 'bg-gray-500' },
  { value: 'Blau', bg: 'bg-blue-600' },
  { value: 'Rot', bg: 'bg-red-600' },
  { value: 'Gr\u00fcn', bg: 'bg-green-600' },
];

const colorMap: Record<string, string> = {
  Schwarz: 'bg-gray-900', 'Wei\u00df': 'bg-white border border-surface-300',
  Silber: 'bg-gray-400', Grau: 'bg-gray-500', Blau: 'bg-blue-600',
  Rot: 'bg-red-600', 'Gr\u00fcn': 'bg-green-600',
};

export function VehiclesPage() {
  const { t } = useTranslation();
  const [vehicles, setVehicles] = useState<Vehicle[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [saving, setSaving] = useState(false);
  const [form, setForm] = useState({ plate: '', make: '', model: '', color: '' });

  useEffect(() => {
    api.getVehicles().then(res => {
      if (res.success && res.data) setVehicles(res.data);
    }).finally(() => setLoading(false));
  }, []);

  async function handleAdd(e: React.FormEvent) {
    e.preventDefault();
    if (!form.plate.trim()) return;
    setSaving(true);
    const res = await api.createVehicle({
      plate: form.plate.toUpperCase(),
      make: form.make || undefined,
      model: form.model || undefined,
      color: form.color || undefined,
    });
    if (res.success && res.data) {
      setVehicles(prev => [...prev, res.data!]);
      toast.success(t('vehicles.added', 'Fahrzeug hinzugef\u00fcgt'));
      setForm({ plate: '', make: '', model: '', color: '' });
      setShowForm(false);
    } else {
      toast.error(res.error?.message || t('common.error'));
    }
    setSaving(false);
  }

  async function handleDelete(id: string) {
    const res = await api.deleteVehicle(id);
    if (res.success) {
      setVehicles(prev => prev.filter(v => v.id !== id));
      toast.success(t('vehicles.removed', 'Fahrzeug entfernt'));
    }
  }

  const container = stagger;
  const item = fadeUp;

  if (loading) return <VehiclesSkeleton />;

  return (
    <AnimatePresence mode="wait">
    <motion.div key="vehicles-loaded" variants={container} initial="hidden" animate="show" className="space-y-8">
      <motion.div variants={item} className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white">{t('vehicles.title', 'Meine Fahrzeuge')}</h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">{t('vehicles.subtitle', 'Fahrzeuge verwalten')}</p>
        </div>
        <button onClick={() => setShowForm(true)} className="btn btn-primary self-start sm:self-auto">
          <Plus weight="bold" className="w-4 h-4" /> {t('vehicles.add', 'Hinzuf\u00fcgen')}
        </button>
      </motion.div>

      {/* Add vehicle modal */}
      <AnimatePresence>
        {showForm && (
          <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="fixed inset-0 z-50 flex items-center justify-center p-4" role="dialog" aria-modal="true" aria-label={t('vehicles.newVehicle', 'New vehicle')}>
            <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={() => setShowForm(false)} aria-hidden="true" />
            <motion.div initial={{ opacity: 0, scale: 0.95, y: 20 }} animate={{ opacity: 1, scale: 1, y: 0 }} exit={{ opacity: 0, scale: 0.95 }} className="relative w-full max-w-md bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 shadow-2xl">
              <div className="flex items-center justify-between px-6 py-4 border-b border-surface-200 dark:border-surface-800">
                <h2 className="text-lg font-semibold text-surface-900 dark:text-white flex items-center gap-2">
                  <Car weight="fill" className="w-5 h-5 text-primary-600" />
                  {t('vehicles.newVehicle', 'Neues Fahrzeug')}
                </h2>
                <button onClick={() => setShowForm(false)} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800" aria-label={t('common.cancel', 'Close')}>
                  <X weight="bold" className="w-5 h-5 text-surface-500" aria-hidden="true" />
                </button>
              </div>
              <form onSubmit={handleAdd} className="p-6 space-y-4">
                <div>
                  <label htmlFor="vehicle-plate" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('vehicles.plate', 'Kennzeichen')} *</label>
                  <input id="vehicle-plate" type="text" value={form.plate} onChange={e => setForm({ ...form, plate: e.target.value })} className="input w-full" placeholder="M-AB 1234" required autoFocus />
                </div>
                <div className="grid grid-cols-2 gap-3">
                  <div>
                    <label htmlFor="vehicle-make" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('vehicles.make', 'Marke')}</label>
                    <input id="vehicle-make" type="text" value={form.make} onChange={e => setForm({ ...form, make: e.target.value })} className="input w-full" placeholder="BMW" />
                  </div>
                  <div>
                    <label htmlFor="vehicle-model" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('vehicles.model', 'Modell')}</label>
                    <input id="vehicle-model" type="text" value={form.model} onChange={e => setForm({ ...form, model: e.target.value })} className="input w-full" placeholder="3er" />
                  </div>
                </div>
                <div>
                  <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('vehicles.color', 'Farbe')}</label>
                  <div className="flex flex-wrap gap-2 mt-1">
                    {COLOR_OPTIONS.map(c => (
                      <button key={c.value} type="button" onClick={() => setForm({ ...form, color: form.color === c.value ? '' : c.value })}
                        aria-label={c.value} aria-pressed={form.color === c.value}
                        className={`w-8 h-8 rounded-full ${c.bg} transition-all ${form.color === c.value ? 'ring-2 ring-primary-500 ring-offset-2 dark:ring-offset-surface-900 scale-110' : ''}`}
                      />
                    ))}
                  </div>
                </div>
                <div className="flex justify-end gap-3 pt-2">
                  <button type="button" onClick={() => setShowForm(false)} className="btn btn-secondary">{t('common.cancel', 'Abbrechen')}</button>
                  <button type="submit" disabled={saving || !form.plate.trim()} className="btn btn-primary">
                    {saving ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> : t('common.save', 'Speichern')}
                  </button>
                </div>
              </form>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Vehicle list */}
      {vehicles.length === 0 ? (
        <motion.div variants={item} className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-16 text-center">
          <motion.div animate={{ y: [0, -4, 0] }} transition={{ duration: 2, repeat: Infinity, ease: 'easeInOut' }}>
            <Car weight="light" className="w-20 h-20 text-surface-200 dark:text-surface-700 mx-auto" />
          </motion.div>
          <p className="text-surface-500 dark:text-surface-400 mb-4 mt-4">{t('vehicles.noVehicles', 'Noch keine Fahrzeuge angelegt')}</p>
          <motion.button onClick={() => setShowForm(true)} className="btn btn-primary" whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}>
            <Plus weight="bold" className="w-4 h-4" /> {t('vehicles.add', 'Hinzuf\u00fcgen')}
          </motion.button>
        </motion.div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {vehicles.map((v, i) => {
            const colorClass = v.color ? (colorMap[v.color] || 'bg-surface-400') : 'bg-surface-400';
            return (
              <motion.div key={v.id} variants={item} className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-5">
                <div className="flex items-start justify-between">
                  <div className="flex items-center gap-3">
                    <div className={`w-3 h-3 rounded-full flex-shrink-0 ${colorClass}`} />
                    <div>
                      <p className="text-lg font-semibold text-surface-900 dark:text-white font-mono tracking-wider">{v.plate}</p>
                      {(v.make || v.model) && <p className="text-sm text-surface-500 dark:text-surface-400">{[v.make, v.model].filter(Boolean).join(' ')}</p>}
                      {v.color && <p className="text-xs text-surface-400 mt-0.5">{v.color}</p>}
                    </div>
                  </div>
                  <button onClick={() => handleDelete(v.id)} className="p-2 rounded-lg text-surface-400 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors" aria-label={t('vehicles.deleteVehicle', { plate: v.plate })}>
                    <Trash weight="regular" className="w-5 h-5" aria-hidden="true" />
                  </button>
                </div>
                {v.is_default && (
                  <div className="mt-3 pt-3 border-t border-surface-100 dark:border-surface-800">
                    <span className="inline-flex items-center gap-1.5 text-xs font-medium text-surface-500 dark:text-surface-400">
                      <Star weight="fill" className="w-3 h-3 text-primary-500" /> {t('vehicles.isDefault', 'Standardfahrzeug')}
                    </span>
                  </div>
                )}
              </motion.div>
            );
          })}
        </div>
      )}
    </motion.div>
    </AnimatePresence>
  );
}
