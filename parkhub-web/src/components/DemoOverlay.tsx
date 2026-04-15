import { useEffect, useRef, useState, useCallback, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { Sparkle, Eye, Timer, ArrowsClockwise, CaretDown, CaretUp } from '@phosphor-icons/react';
import { api, type DemoStatus } from '../api/client';

// Module-level stable identity for the default reloadPage. Previously this
// was an inline arrow in the component signature, so React created a fresh
// function identity on every render; the useEffect that depends on it
// re-fired every render, clearing the 15s interval and immediately calling
// fetchStatus() again. Combined with the 1-second setTick below, that
// produced 1+ call/sec of /api/v1/demo/status (≈100/sec under E2E) and
// Playwright's networkidle auto-wait never stabilized.
const defaultReloadPage = () => window.location.reload();

function formatRelativeTime(isoString: string): string {
  const diff = Math.floor((Date.now() - new Date(isoString).getTime()) / 1000);
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  const h = Math.floor(diff / 3600);
  const m = Math.floor((diff % 3600) / 60);
  return m > 0 ? `${h}h ${m}m ago` : `${h}h ago`;
}

function formatCountdown(isoString: string): string {
  const diff = Math.max(0, Math.floor((new Date(isoString).getTime() - Date.now()) / 1000));
  if (diff === 0) return 'now';
  const h = Math.floor(diff / 3600);
  const m = Math.floor((diff % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
}

export function DemoOverlay({ reloadPage = defaultReloadPage }: { reloadPage?: () => void } = {}) {
  const { t } = useTranslation();
  const [demo, setDemo] = useState<DemoStatus | null>(null);
  const [enabled, setEnabled] = useState(false);
  const [collapsed, setCollapsed] = useState(() => window.innerWidth < 640);
  const [localTimer, setLocalTimer] = useState(0);
  const [, setTick] = useState(0); // force re-render for relative times

  // Stash reloadPage in a ref so the poll effect only re-fires when
  // `enabled` flips, not on every parent render.
  const reloadPageRef = useRef(reloadPage);
  reloadPageRef.current = reloadPage;

  // Check if demo mode is enabled
  useEffect(() => {
    api.getDemoConfig().then(res => {
      if (res.success && res.data?.demo_mode) {
        setEnabled(true);
      }
    }).catch(() => {});
  }, []);

  // Poll demo status — 15 second interval, re-fires only on enabled changes.
  useEffect(() => {
    if (!enabled) return;

    function fetchStatus() {
      api.getDemoStatus().then(res => {
        if (res.success && res.data) {
          setDemo(res.data);
          setLocalTimer(res.data.timer_seconds);
          if (res.data.reset) {
            reloadPageRef.current();
          }
        }
      }).catch(() => {});
    }

    fetchStatus();
    const interval = setInterval(fetchStatus, 15000);
    return () => clearInterval(interval);
  }, [enabled]);

  // Local countdown + relative time refresh (only when expanded)
  useEffect(() => {
    if (!enabled || collapsed) return;
    const interval = setInterval(() => {
      setLocalTimer(t => Math.max(0, t - 1));
      setTick(t => t + 1);
    }, 1000);
    return () => clearInterval(interval);
  }, [enabled, collapsed]);

  const handleVote = useCallback(() => {
    api.voteDemoReset().then(res => {
      if (res.success) {
        setDemo(prev => prev ? { ...prev, has_voted: true, votes: prev.votes + 1 } : prev);
      }
    });
  }, []);

  const lastReset = useMemo(
    () => demo?.last_reset_at ? formatRelativeTime(demo.last_reset_at) : null,
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [demo?.last_reset_at, localTimer]
  );

  const nextReset = useMemo(
    () => demo?.next_scheduled_reset ? formatCountdown(demo.next_scheduled_reset) : null,
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [demo?.next_scheduled_reset, localTimer]
  );

  if (!enabled || !demo) return null;

  const minutes = Math.floor(localTimer / 60);
  const seconds = localTimer % 60;
  const isLow = localTimer < 300;
  const voteProgress = demo.vote_threshold > 0 ? (demo.votes / demo.vote_threshold) * 100 : 0;

  return (
    <motion.div
      initial={{ y: -80, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      className="fixed top-3 left-1/2 -translate-x-1/2 z-40 max-sm:top-auto max-sm:bottom-20 max-sm:left-auto max-sm:right-3 max-sm:translate-x-0 max-sm:scale-90 max-sm:origin-bottom-right"
    >
      <div className="glass-card shadow-xl">
        <button
          onClick={() => setCollapsed(!collapsed)}
          aria-expanded={!collapsed}
          className="flex items-center gap-3 px-4 py-3 min-w-[200px]"
        >
          {/* Demo badge */}
          <span className="flex items-center gap-1 badge badge-primary">
            <Sparkle weight="fill" className="w-3 h-3 animate-pulse" />
            {t('demo.badge')}
          </span>

          {/* Timer */}
          <span className={`font-mono text-sm font-bold transition-colors duration-300 ${isLow ? 'text-red-500' : 'text-surface-700 dark:text-surface-300'}`}>
            <Timer weight="bold" className="w-3.5 h-3.5 inline mr-1" />
            {String(minutes).padStart(2, '0')}:{String(seconds).padStart(2, '0')}
          </span>

          {/* Viewers */}
          <span className="flex items-center gap-1 text-xs text-surface-600 dark:text-surface-400">
            <Eye weight="regular" className="w-3.5 h-3.5" />
            {demo.viewers}
          </span>

          {collapsed ? <CaretDown weight="bold" className="w-3 h-3 text-surface-400" /> : <CaretUp weight="bold" className="w-3 h-3 text-surface-400" />}
        </button>

        <AnimatePresence>
          {!collapsed && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: 'auto', opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              className="overflow-hidden"
            >
              <div className="px-4 pb-3 pt-1 border-t border-surface-200/50 dark:border-surface-700/50">
                {/* Vote progress */}
                <div className="flex items-center gap-2 mb-2">
                  <div
                    className="flex-1 h-2 bg-surface-200 dark:bg-surface-700 rounded-full overflow-hidden"
                    role="progressbar"
                    aria-valuenow={demo.votes}
                    aria-valuemin={0}
                    aria-valuemax={demo.vote_threshold}
                    aria-label={t('demo.votesNeeded', { current: demo.votes, needed: demo.vote_threshold })}
                  >
                    <motion.div
                      className="h-full bg-primary-500 rounded-full"
                      animate={{ width: `${voteProgress}%` }}
                      transition={{ type: 'spring', stiffness: 100 }}
                    />
                  </div>
                  <span className="text-xs font-medium text-surface-600 dark:text-surface-400">
                    {t('demo.votesNeeded', { current: demo.votes, needed: demo.vote_threshold })}
                  </span>
                </div>

                <button
                  onClick={handleVote}
                  disabled={demo.has_voted || demo.reset_in_progress}
                  className="btn btn-sm btn-primary w-full disabled:opacity-50"
                >
                  <ArrowsClockwise weight="bold" className={`w-3.5 h-3.5 ${demo.reset_in_progress ? 'animate-spin' : ''}`} />
                  {demo.reset_in_progress
                    ? t('demo.resetting', 'Resetting...')
                    : demo.has_voted
                      ? t('demo.votesNeeded', { current: demo.votes, needed: demo.vote_threshold })
                      : t('demo.voteReset')}
                </button>

                {/* Auto-reset info */}
                {(lastReset || nextReset) && (
                  <div className="mt-2 pt-2 border-t border-surface-200/30 dark:border-surface-700/30 text-xs text-surface-600 dark:text-surface-400 space-y-0.5">
                    {lastReset && (
                      <div className="flex justify-between">
                        <span>{t('demo.lastReset', 'Last reset')}</span>
                        <span className="font-mono">{lastReset}</span>
                      </div>
                    )}
                    {nextReset && (
                      <div className="flex justify-between">
                        <span>{t('demo.nextReset', 'Next reset')}</span>
                        <span className="font-mono">{nextReset}</span>
                      </div>
                    )}
                  </div>
                )}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </motion.div>
  );
}
