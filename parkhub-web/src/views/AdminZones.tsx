import { useEffect, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { MapTrifold, Pencil, Question, Tag } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

type PricingTier = 'economy' | 'standard' | 'premium' | 'vip';

interface ZoneWithPricing {
  id: string;
  lot_id: string;
  name: string;
  description: string | null;
  color: string | null;
  tier: PricingTier;
  tier_display: string;
  tier_color: string;
  pricing_multiplier: number;
  max_capacity: number | null;
}

const TIER_STYLES: Record<PricingTier, { bg: string; text: string; border: string }> = {
  economy: { bg: 'bg-green-100 dark:bg-green-900/30', text: 'text-green-700 dark:text-green-400', border: 'border-green-300 dark:border-green-700' },
  standard: { bg: 'bg-blue-100 dark:bg-blue-900/30', text: 'text-blue-700 dark:text-blue-400', border: 'border-blue-300 dark:border-blue-700' },
  premium: { bg: 'bg-amber-100 dark:bg-amber-900/30', text: 'text-amber-700 dark:text-amber-400', border: 'border-amber-300 dark:border-amber-700' },
  vip: { bg: 'bg-purple-100 dark:bg-purple-900/30', text: 'text-purple-700 dark:text-purple-400', border: 'border-purple-300 dark:border-purple-700' },
};

export function AdminZonesPage() {
  const { t } = useTranslation();
  const [zones, setZones] = useState<ZoneWithPricing[]>([]);
  const [loading, setLoading] = useState(true);
  const [showHelp, setShowHelp] = useState(false);
  const [editZoneId, setEditZoneId] = useState<string | null>(null);
  const [editTier, setEditTier] = useState<PricingTier>('standard');
  const [editMultiplier, setEditMultiplier] = useState('1.0');
  const [editCapacity, setEditCapacity] = useState('');

  const loadZones = useCallback(async () => {
    setLoading(true);
    try {
      // Load zones from all lots
      const lotsRes = await fetch('/api/v1/lots').then(r => r.json());
      if (lotsRes.success && lotsRes.data) {
        const allZones: ZoneWithPricing[] = [];
        for (const lot of lotsRes.data) {
          const zRes = await fetch(`/api/v1/lots/${lot.id}/zones/pricing`).then(r => r.json());
          if (zRes.success && zRes.data) {
            allZones.push(...zRes.data);
          }
        }
        setZones(allZones);
      }
    } catch {
      toast.error(t('common.error'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => { loadZones(); }, [loadZones]);

  function startEdit(zone: ZoneWithPricing) {
    setEditZoneId(zone.id);
    setEditTier(zone.tier);
    setEditMultiplier(zone.pricing_multiplier.toString());
    setEditCapacity(zone.max_capacity?.toString() || '');
  }

  async function handleSavePricing() {
    if (!editZoneId) return;
    try {
      const body: any = { tier: editTier };
      const mult = parseFloat(editMultiplier);
      if (!isNaN(mult) && mult > 0) body.pricing_multiplier = mult;
      const cap = parseInt(editCapacity, 10);
      if (!isNaN(cap) && cap > 0) body.max_capacity = cap;

      const res = await fetch(`/api/v1/admin/zones/${editZoneId}/pricing`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      }).then(r => r.json());

      if (res.success) {
        toast.success(t('parkingZones.pricingUpdated'));
        setEditZoneId(null);
        loadZones();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } catch {
      toast.error(t('common.error'));
    }
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-bold text-surface-900 dark:text-white flex items-center gap-2">
            <MapTrifold weight="duotone" className="w-6 h-6 text-primary-500" />
            {t('parkingZones.title')}
          </h2>
          <p className="text-sm text-surface-500 dark:text-surface-400 mt-1">
            {t('parkingZones.subtitle')}
          </p>
        </div>
        <button
          onClick={() => setShowHelp(h => !h)}
          className="p-2 rounded-lg text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-800"
          title={t('parkingZones.helpLabel')}
        >
          <Question className="w-5 h-5" />
        </button>
      </div>

      {/* Help */}
      <AnimatePresence>
        {showHelp && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-xl p-4 text-sm text-blue-700 dark:text-blue-300"
          >
            {t('parkingZones.help')}
          </motion.div>
        )}
      </AnimatePresence>

      {/* Zone cards */}
      {loading ? (
        <div className="text-center py-12 text-surface-400">{t('common.loading')}</div>
      ) : zones.length === 0 ? (
        <div className="text-center py-12 text-surface-400">{t('parkingZones.empty')}</div>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {zones.map(zone => {
            const style = TIER_STYLES[zone.tier] || TIER_STYLES.standard;
            return (
              <motion.div
                key={zone.id}
                layout
                className="bg-white dark:bg-surface-800 rounded-2xl border border-surface-200 dark:border-surface-700 p-5 space-y-3"
              >
                <div className="flex items-start justify-between">
                  <div>
                    <h3 className="font-semibold text-surface-900 dark:text-white">{zone.name}</h3>
                    {zone.description && (
                      <p className="text-xs text-surface-500 mt-0.5">{zone.description}</p>
                    )}
                  </div>
                  <button
                    onClick={() => startEdit(zone)}
                    className="p-2 rounded-lg text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-700"
                    title={t('parkingZones.editPricing')}
                  >
                    <Pencil className="w-4 h-4" />
                  </button>
                </div>

                {/* Tier badge */}
                <div className="flex items-center gap-2">
                  <span className={`px-3 py-1 rounded-full text-xs font-bold uppercase ${style.bg} ${style.text}`}>
                    <Tag weight="bold" className="w-3 h-3 inline mr-1" />
                    {zone.tier_display}
                  </span>
                  <span className="text-xs text-surface-500">
                    {zone.pricing_multiplier}x
                  </span>
                </div>

                {/* Capacity bar */}
                {zone.max_capacity && (
                  <div>
                    <div className="flex justify-between text-xs text-surface-500 mb-1">
                      <span>{t('parkingZones.capacity')}</span>
                      <span>{zone.max_capacity}</span>
                    </div>
                    <div className="h-2 bg-surface-100 dark:bg-surface-700 rounded-full overflow-hidden">
                      <div
                        className="h-full rounded-full transition-all"
                        style={{ width: '60%', backgroundColor: zone.tier_color }}
                      />
                    </div>
                  </div>
                )}

                {/* Edit form inline */}
                {editZoneId === zone.id && (
                  <div className="mt-3 pt-3 border-t border-surface-200 dark:border-surface-700 space-y-3">
                    <div>
                      <label className="block text-xs font-medium text-surface-600 dark:text-surface-400 mb-1">
                        {t('parkingZones.tier')}
                      </label>
                      <div className="flex gap-1">
                        {(['economy', 'standard', 'premium', 'vip'] as const).map(tier => {
                          const ts = TIER_STYLES[tier];
                          return (
                            <button
                              key={tier}
                              onClick={() => setEditTier(tier)}
                              className={`px-2 py-1 rounded-lg text-xs font-medium border transition-colors ${
                                editTier === tier
                                  ? `${ts.bg} ${ts.text} ${ts.border}`
                                  : 'bg-surface-50 dark:bg-surface-900 border-surface-200 dark:border-surface-700 text-surface-500'
                              }`}
                            >
                              {tier.toUpperCase()}
                            </button>
                          );
                        })}
                      </div>
                    </div>
                    <div className="grid grid-cols-2 gap-2">
                      <div>
                        <label className="block text-xs font-medium text-surface-600 dark:text-surface-400 mb-1">
                          {t('parkingZones.multiplier')}
                        </label>
                        <input
                          type="number"
                          step="0.1"
                          min="0.1"
                          value={editMultiplier}
                          onChange={e => setEditMultiplier(e.target.value)}
                          className="w-full px-2 py-1 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-sm"
                        />
                      </div>
                      <div>
                        <label className="block text-xs font-medium text-surface-600 dark:text-surface-400 mb-1">
                          {t('parkingZones.maxCapacity')}
                        </label>
                        <input
                          type="number"
                          min="1"
                          value={editCapacity}
                          onChange={e => setEditCapacity(e.target.value)}
                          placeholder={t('parkingZones.optional')}
                          className="w-full px-2 py-1 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-sm"
                        />
                      </div>
                    </div>
                    <div className="flex gap-2">
                      <button
                        onClick={handleSavePricing}
                        className="px-3 py-1.5 bg-primary-600 text-white rounded-lg text-xs font-medium hover:bg-primary-700"
                      >
                        {t('parkingZones.save')}
                      </button>
                      <button
                        onClick={() => setEditZoneId(null)}
                        className="px-3 py-1.5 bg-surface-100 dark:bg-surface-700 text-surface-600 dark:text-surface-400 rounded-lg text-xs"
                      >
                        {t('common.cancel')}
                      </button>
                    </div>
                  </div>
                )}
              </motion.div>
            );
          })}
        </div>
      )}
    </div>
  );
}
