import { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { ShieldCheck, Warning } from '@phosphor-icons/react';
import { api, type RateLimitGroup, type RateLimitHistoryBin } from '../api/client';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

const groupColors: Record<string, string> = {
  auth: 'bg-red-500',
  api: 'bg-blue-500',
  public: 'bg-emerald-500',
  webhook: 'bg-amber-500',
};

export function AdminRateLimitsPage() {
  const { t } = useTranslation();
  const [groups, setGroups] = useState<RateLimitGroup[]>([]);
  const [totalBlocked, setTotalBlocked] = useState(0);
  const [historyBins, setHistoryBins] = useState<RateLimitHistoryBin[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadData();
  }, []);

  async function loadData() {
    setLoading(true);
    try {
      const [statsRes, historyRes] = await Promise.all([
        api.getRateLimitStats(),
        api.getRateLimitHistory(),
      ]);
      if (statsRes.success && statsRes.data) {
        setGroups(statsRes.data.groups);
        setTotalBlocked(statsRes.data.total_blocked_last_hour);
      }
      if (historyRes.success && historyRes.data) {
        setHistoryBins(historyRes.data.bins);
      }
    } catch {
      toast.error(t('common.error'));
    } finally {
      setLoading(false);
    }
  }

  const maxBinCount = Math.max(1, ...historyBins.map(b => b.count));

  if (loading) return (
    <div className="space-y-4">
      <div className="h-8 w-48 skeleton rounded-lg" />
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {[1, 2, 3, 4].map(i => <div key={i} className="h-32 skeleton rounded-2xl" />)}
      </div>
      <div className="h-48 skeleton rounded-2xl" />
    </div>
  );

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-bold text-surface-900 dark:text-white">
          {t('rateLimits.title', 'Rate Limits')}
        </h2>
        <div className="flex items-center gap-2 text-sm">
          {totalBlocked > 0 ? (
            <span className="flex items-center gap-1 text-amber-600 dark:text-amber-400">
              <Warning weight="bold" className="w-4 h-4" />
              {t('rateLimits.blockedTotal', '{{count}} blocked (last hour)', { count: totalBlocked })}
            </span>
          ) : (
            <span className="flex items-center gap-1 text-emerald-600 dark:text-emerald-400">
              <ShieldCheck weight="bold" className="w-4 h-4" />
              {t('rateLimits.allClear', 'No blocked requests')}
            </span>
          )}
        </div>
      </div>

      {/* Rate limit group cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {groups.map(group => {
          const pct = group.limit_per_minute > 0
            ? Math.min(100, Math.round((group.current_count / group.limit_per_minute) * 100))
            : 0;
          const barColor = groupColors[group.group] || 'bg-surface-500';
          return (
            <div key={group.group} className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-4" data-testid={`rate-group-${group.group}`}>
              <div className="flex items-center justify-between mb-2">
                <h3 className="text-sm font-semibold text-surface-900 dark:text-white capitalize">{group.group}</h3>
                <span className="text-xs text-surface-500 dark:text-surface-400">
                  {group.limit_per_minute}/{t('rateLimits.perMinute', 'min')}
                </span>
              </div>
              <p className="text-xs text-surface-500 dark:text-surface-400 mb-3">{group.description}</p>
              {/* Progress bar */}
              <div className="w-full h-2 rounded-full bg-surface-100 dark:bg-surface-800 mb-2">
                <div className={`h-full rounded-full ${barColor} transition-all`} style={{ width: `${pct}%` }} />
              </div>
              <div className="flex items-center justify-between text-xs text-surface-500 dark:text-surface-400">
                <span>{group.current_count} / {group.limit_per_minute}</span>
                {group.blocked_last_hour > 0 && (
                  <span className="text-amber-600 dark:text-amber-400">
                    {group.blocked_last_hour} {t('rateLimits.blocked', 'blocked')}
                  </span>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* Blocked requests chart (24h) */}
      <div className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-4">
        <h3 className="text-sm font-semibold text-surface-900 dark:text-white mb-4">
          {t('rateLimits.blockedHistory', 'Blocked Requests (24h)')}
        </h3>
        <div className="flex items-end gap-1 h-32" data-testid="blocked-chart">
          {historyBins.map((bin, idx) => {
            const heightPct = maxBinCount > 0 ? (bin.count / maxBinCount) * 100 : 0;
            return (
              <div key={idx} className="flex-1 flex flex-col items-center justify-end h-full" title={`${bin.hour}: ${bin.count}`}>
                <div
                  className={`w-full rounded-t ${bin.count > 0 ? 'bg-amber-500' : 'bg-surface-100 dark:bg-surface-800'}`}
                  style={{ height: `${Math.max(heightPct, 2)}%` }}
                />
              </div>
            );
          })}
        </div>
        <div className="flex justify-between mt-2 text-[10px] text-surface-400">
          <span>{historyBins[0]?.hour.slice(11, 16) || ''}</span>
          <span>{t('rateLimits.now', 'now')}</span>
        </div>
      </div>
    </motion.div>
  );
}

export default AdminRateLimitsPage;
