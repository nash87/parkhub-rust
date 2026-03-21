import { useEffect, useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Star, Trash, MapPin, SpinnerGap } from '@phosphor-icons/react';
import { api, type Favorite, type ParkingLot, type ParkingSlot } from '../api/client';
import { stagger, fadeUp } from '../constants/animations';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

interface EnrichedFavorite extends Favorite {
  lot_name?: string;
  slot_number?: string;
  slot_status?: string;
}

export function FavoritesPage() {
  const { t } = useTranslation();
  const [favorites, setFavorites] = useState<Favorite[]>([]);
  const [lots, setLots] = useState<ParkingLot[]>([]);
  const [slotMap, setSlotMap] = useState<Record<string, ParkingSlot>>({});
  const [loading, setLoading] = useState(true);
  const [removing, setRemoving] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([
      api.getFavorites(),
      api.getLots(),
    ]).then(async ([favRes, lotRes]) => {
      const favs = favRes.success && favRes.data ? favRes.data : [];
      const allLots = lotRes.success && lotRes.data ? lotRes.data : [];
      setFavorites(favs);
      setLots(allLots);

      // Fetch slots for each lot that has favorites
      const lotIds = [...new Set(favs.map(f => f.lot_id))];
      const slotsById: Record<string, ParkingSlot> = {};
      await Promise.all(lotIds.map(async (lotId) => {
        const res = await api.getLotSlots(lotId);
        if (res.success && res.data) {
          for (const s of res.data) slotsById[s.id] = s;
        }
      }));
      setSlotMap(slotsById);
    }).finally(() => setLoading(false));
  }, []);

  const lotMap = useMemo(() => {
    const m: Record<string, ParkingLot> = {};
    for (const l of lots) m[l.id] = l;
    return m;
  }, [lots]);

  const enriched: EnrichedFavorite[] = useMemo(() =>
    favorites.map(f => ({
      ...f,
      lot_name: lotMap[f.lot_id]?.name,
      slot_number: slotMap[f.slot_id]?.slot_number,
      slot_status: slotMap[f.slot_id]?.status,
    })),
  [favorites, lotMap, slotMap]);

  async function handleRemove(slotId: string) {
    setRemoving(slotId);
    const res = await api.removeFavorite(slotId);
    if (res.success) {
      setFavorites(prev => prev.filter(f => f.slot_id !== slotId));
      toast.success(t('favorites.removed'));
    } else {
      toast.error(res.error?.message || t('common.error'));
    }
    setRemoving(null);
  }

  const container = stagger;
  const item = fadeUp;

  if (loading) {
    return (
      <div className="space-y-6">
        <div className="skeleton h-8 w-48 rounded-lg" />
        <div className="skeleton h-4 w-64 rounded" />
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 mt-6">
          {[1, 2, 3].map(i => <div key={i} className="skeleton h-24 rounded-xl" />)}
        </div>
      </div>
    );
  }

  return (
    <AnimatePresence mode="wait">
      <motion.div key="favorites-loaded" variants={container} initial="hidden" animate="show" className="space-y-8">
        <motion.div variants={item} className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div>
            <h1 className="text-2xl font-bold text-surface-900 dark:text-white">{t('favorites.title')}</h1>
            <p className="text-surface-500 dark:text-surface-400 mt-1">{t('favorites.subtitle')}</p>
          </div>
          {favorites.length > 0 && (
            <span className="text-sm text-surface-500 dark:text-surface-400">
              {t('favorites.count', { count: favorites.length })}
            </span>
          )}
        </motion.div>

        {enriched.length === 0 ? (
          <motion.div variants={item} className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-16 text-center">
            <motion.div animate={{ y: [0, -4, 0] }} transition={{ duration: 2, repeat: Infinity, ease: 'easeInOut' }}>
              <Star weight="light" className="w-20 h-20 text-surface-200 dark:text-surface-700 mx-auto" />
            </motion.div>
            <p className="text-surface-500 dark:text-surface-400 mb-2 mt-4">{t('favorites.empty')}</p>
            <p className="text-sm text-surface-400 dark:text-surface-500">{t('favorites.emptyHint')}</p>
          </motion.div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {enriched.map((fav) => {
              const isAvailable = fav.slot_status === 'available';
              return (
                <motion.div
                  key={fav.slot_id}
                  variants={item}
                  layout
                  className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-5"
                >
                  <div className="flex items-start justify-between">
                    <div className="flex items-center gap-3">
                      <div className={`w-10 h-10 rounded-lg flex items-center justify-center ${
                        isAvailable
                          ? 'bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400'
                          : 'bg-surface-100 dark:bg-surface-800 text-surface-400'
                      }`}>
                        <Star weight="fill" className="w-5 h-5" />
                      </div>
                      <div>
                        <p className="font-semibold text-surface-900 dark:text-white">
                          {t('favorites.slot')} {fav.slot_number || '—'}
                        </p>
                        <div className="flex items-center gap-1.5 text-sm text-surface-500 dark:text-surface-400">
                          <MapPin weight="regular" className="w-3.5 h-3.5" />
                          {fav.lot_name || t('favorites.unknownLot')}
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className={`text-xs font-medium px-2 py-0.5 rounded-full ${
                        isAvailable
                          ? 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
                          : 'bg-surface-100 text-surface-500 dark:bg-surface-800 dark:text-surface-400'
                      }`}>
                        {isAvailable ? t('favorites.available') : t('favorites.occupied')}
                      </span>
                      <button
                        onClick={() => handleRemove(fav.slot_id)}
                        disabled={removing === fav.slot_id}
                        className="p-2 rounded-lg text-surface-400 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors"
                        aria-label={t('favorites.remove', { slot: fav.slot_number || fav.slot_id })}
                      >
                        {removing === fav.slot_id
                          ? <SpinnerGap weight="bold" className="w-5 h-5 animate-spin" />
                          : <Trash weight="regular" className="w-5 h-5" aria-hidden="true" />
                        }
                      </button>
                    </div>
                  </div>
                  {fav.created_at && (
                    <p className="text-xs text-surface-400 dark:text-surface-500 mt-3 pt-3 border-t border-surface-100 dark:border-surface-800">
                      {t('favorites.addedOn', { date: new Date(fav.created_at).toLocaleDateString() })}
                    </p>
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
