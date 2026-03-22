import { useEffect, useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Bell, Queue, Check, X, Question, Clock, ArrowUp } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

interface WaitlistEntry {
  id: string;
  user_id: string;
  lot_id: string;
  created_at: string;
  notified_at: string | null;
  status: 'waiting' | 'offered' | 'accepted' | 'declined' | 'expired';
  offer_expires_at: string | null;
  accepted_booking_id: string | null;
}

interface WaitlistPosition {
  entry: WaitlistEntry;
  position: number;
  total_ahead: number;
  estimated_wait_minutes: number | null;
}

interface Lot {
  id: string;
  name: string;
  total_slots: number;
  available_slots: number;
}

const statusColors: Record<string, string> = {
  waiting: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  offered: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
  accepted: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
  declined: 'bg-surface-100 text-surface-500 dark:bg-surface-800 dark:text-surface-400',
  expired: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
};

export function WaitlistPage() {
  const { t } = useTranslation();
  const [lots, setLots] = useState<Lot[]>([]);
  const [entries, setEntries] = useState<WaitlistPosition[]>([]);
  const [loading, setLoading] = useState(true);
  const [showHelp, setShowHelp] = useState(false);
  const [joining, setJoining] = useState<string | null>(null);

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const lotsRes = await fetch('/api/v1/lots').then(r => r.json());
      if (lotsRes.success) {
        const allLots: Lot[] = lotsRes.data || [];
        setLots(allLots);

        // Load waitlist position for each full lot
        const fullLots = allLots.filter(l => l.available_slots === 0);
        const waitlistPromises = fullLots.map(async (lot: Lot) => {
          try {
            const res = await fetch(`/api/v1/lots/${lot.id}/waitlist`).then(r => r.json());
            if (res.success && res.data.entries.length > 0) {
              return res.data.entries as WaitlistPosition[];
            }
          } catch { /* ignore */ }
          return [] as WaitlistPosition[];
        });
        const results = await Promise.all(waitlistPromises);
        setEntries(results.flat());
      }
    } catch { /* ignore */ }
    setLoading(false);
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  async function handleJoin(lotId: string) {
    setJoining(lotId);
    try {
      const res = await fetch(`/api/v1/lots/${lotId}/waitlist/subscribe`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ priority: 3 }),
      }).then(r => r.json());
      if (res.success) {
        toast.success(t('waitlistExt.joined'));
        loadData();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } catch {
      toast.error(t('common.error'));
    }
    setJoining(null);
  }

  async function handleLeave(lotId: string) {
    try {
      const res = await fetch(`/api/v1/lots/${lotId}/waitlist`, { method: 'DELETE' }).then(r => r.json());
      if (res.success) {
        toast.success(t('waitlistExt.left'));
        loadData();
      }
    } catch {
      toast.error(t('common.error'));
    }
  }

  async function handleAccept(lotId: string, entryId: string) {
    try {
      const res = await fetch(`/api/v1/lots/${lotId}/waitlist/${entryId}/accept`, { method: 'POST' }).then(r => r.json());
      if (res.success) {
        toast.success(t('waitlistExt.accepted'));
        loadData();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } catch {
      toast.error(t('common.error'));
    }
  }

  async function handleDecline(lotId: string, entryId: string) {
    try {
      const res = await fetch(`/api/v1/lots/${lotId}/waitlist/${entryId}/decline`, { method: 'POST' }).then(r => r.json());
      if (res.success) {
        toast.success(t('waitlistExt.declined'));
        loadData();
      }
    } catch {
      toast.error(t('common.error'));
    }
  }

  const fullLots = lots.filter(l => l.available_slots === 0);
  const userEntryLotIds = new Set(entries.map(e => e.entry.lot_id));

  return (
    <div className="space-y-6 p-4 max-w-4xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-surface-50">
            {t('waitlistExt.title')}
          </h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">
            {t('waitlistExt.subtitle')}
          </p>
        </div>
        <button
          onClick={() => setShowHelp(!showHelp)}
          className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800"
          aria-label={t('waitlistExt.helpLabel')}
        >
          <Question size={24} />
        </button>
      </div>

      {/* Help tooltip */}
      {showHelp && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          className="p-4 rounded-xl bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800"
        >
          <p className="text-sm text-blue-700 dark:text-blue-300">
            {t('waitlistExt.help')}
          </p>
        </motion.div>
      )}

      {loading ? (
        <div className="flex justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-2 border-primary-500 border-t-transparent" />
        </div>
      ) : (
        <>
          {/* Active waitlist entries */}
          {entries.length > 0 && (
            <div className="space-y-3">
              <h2 className="text-lg font-semibold text-surface-800 dark:text-surface-200">
                {t('waitlistExt.yourEntries')}
              </h2>
              {entries.map(wp => {
                const lot = lots.find(l => l.id === wp.entry.lot_id);
                return (
                  <motion.div
                    key={wp.entry.id}
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="p-4 rounded-xl bg-white dark:bg-surface-800 border border-surface-200 dark:border-surface-700 shadow-sm"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        <Queue size={20} className="text-primary-500" />
                        <div>
                          <p className="font-medium text-surface-900 dark:text-surface-100">
                            {lot?.name || wp.entry.lot_id}
                          </p>
                          <div className="flex items-center gap-2 mt-1">
                            <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${statusColors[wp.entry.status]}`}>
                              {t(`waitlistExt.status.${wp.entry.status}`)}
                            </span>
                            {wp.entry.status === 'waiting' && (
                              <span className="text-xs text-surface-500 flex items-center gap-1">
                                <ArrowUp size={12} />
                                {t('waitlistExt.position', { pos: wp.position })}
                              </span>
                            )}
                            {wp.estimated_wait_minutes != null && wp.entry.status === 'waiting' && (
                              <span className="text-xs text-surface-500 flex items-center gap-1">
                                <Clock size={12} />
                                {t('waitlistExt.estimatedWait', { minutes: wp.estimated_wait_minutes })}
                              </span>
                            )}
                          </div>
                        </div>
                      </div>
                      <div className="flex gap-2">
                        {wp.entry.status === 'offered' && (
                          <>
                            <button
                              onClick={() => handleAccept(wp.entry.lot_id, wp.entry.id)}
                              className="flex items-center gap-1 px-3 py-1.5 bg-green-500 text-white rounded-lg text-sm hover:bg-green-600"
                            >
                              <Check size={16} /> {t('waitlistExt.accept')}
                            </button>
                            <button
                              onClick={() => handleDecline(wp.entry.lot_id, wp.entry.id)}
                              className="flex items-center gap-1 px-3 py-1.5 bg-surface-200 dark:bg-surface-700 rounded-lg text-sm hover:bg-surface-300 dark:hover:bg-surface-600"
                            >
                              <X size={16} /> {t('waitlistExt.decline')}
                            </button>
                          </>
                        )}
                        {wp.entry.status === 'waiting' && (
                          <button
                            onClick={() => handleLeave(wp.entry.lot_id)}
                            className="px-3 py-1.5 text-sm text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-lg"
                          >
                            {t('waitlistExt.leave')}
                          </button>
                        )}
                      </div>
                    </div>
                  </motion.div>
                );
              })}
            </div>
          )}

          {/* Full lots where user can join */}
          {fullLots.length > 0 && (
            <div className="space-y-3">
              <h2 className="text-lg font-semibold text-surface-800 dark:text-surface-200">
                {t('waitlistExt.fullLots')}
              </h2>
              {fullLots
                .filter(l => !userEntryLotIds.has(l.id))
                .map(lot => (
                  <motion.div
                    key={lot.id}
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    className="p-4 rounded-xl bg-white dark:bg-surface-800 border border-surface-200 dark:border-surface-700 shadow-sm flex items-center justify-between"
                  >
                    <div>
                      <p className="font-medium text-surface-900 dark:text-surface-100">{lot.name}</p>
                      <p className="text-xs text-surface-500 mt-0.5">
                        {t('waitlistExt.lotFull', { total: lot.total_slots })}
                      </p>
                    </div>
                    <button
                      onClick={() => handleJoin(lot.id)}
                      disabled={joining === lot.id}
                      className="flex items-center gap-2 px-4 py-2 bg-primary-500 text-white rounded-lg hover:bg-primary-600 disabled:opacity-50"
                    >
                      <Bell size={16} />
                      {joining === lot.id ? t('waitlistExt.joiningWaitlist') : t('waitlistExt.joinWaitlist')}
                    </button>
                  </motion.div>
                ))}
            </div>
          )}

          {/* Empty state */}
          {fullLots.length === 0 && entries.length === 0 && (
            <div className="text-center py-12 text-surface-400">
              <Queue size={48} className="mx-auto mb-3 opacity-40" />
              <p>{t('waitlistExt.noFullLots')}</p>
            </div>
          )}
        </>
      )}
    </div>
  );
}

export default WaitlistPage;
