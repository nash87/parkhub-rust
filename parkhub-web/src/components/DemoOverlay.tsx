import { useEffect, useState, useCallback, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { Sparkle, Eye, Timer, ArrowsClockwise, CaretDown, CaretUp, X } from '@phosphor-icons/react';
import { api, type DemoStatus } from '../api/client';

export function DemoOverlay() {
  const { t } = useTranslation();
  const [demo, setDemo] = useState<DemoStatus | null>(null);
  const [enabled, setEnabled] = useState(false);
  const [collapsed, setCollapsed] = useState(false);
  const [localTimer, setLocalTimer] = useState(0);
  const [resetCountdown, setResetCountdown] = useState<number | null>(null);
  const resetTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const isSolo = (demo?.viewers ?? 0) <= 1;

  // Check if demo mode is enabled
  useEffect(() => {
    api.getDemoConfig().then(res => {
      if (res.success && res.data?.demo_mode) {
        setEnabled(true);
      }
    }).catch(() => { /* Demo mode not available */ });
  }, []);

  // Poll demo status
  useEffect(() => {
    if (!enabled) return;

    function fetchStatus() {
      api.getDemoStatus().then(res => {
        if (res.success && res.data) {
          setDemo(res.data);
          setLocalTimer(res.data.timer.remaining);
          if (res.data.reset) {
            window.location.reload();
          }
        }
      }).catch(() => {});
    }

    fetchStatus();
    const interval = setInterval(fetchStatus, 15000);
    return () => clearInterval(interval);
  }, [enabled]);

  // Local countdown
  useEffect(() => {
    if (!enabled || localTimer <= 0) return;
    const interval = setInterval(() => {
      setLocalTimer(t => Math.max(0, t - 1));
    }, 1000);
    return () => clearInterval(interval);
  }, [enabled, localTimer > 0]);

  // Solo reset countdown
  const startResetCountdown = useCallback(() => {
    setResetCountdown(10);
    resetTimerRef.current = setInterval(() => {
      setResetCountdown(prev => {
        if (prev === null) return null;
        if (prev <= 1) {
          clearInterval(resetTimerRef.current!);
          resetTimerRef.current = null;
          api.resetDemo().then(res => {
            if (res.success) {
              window.location.reload();
            }
          });
          return null;
        }
        return prev - 1;
      });
    }, 1000);
  }, []);

  const cancelResetCountdown = useCallback(() => {
    if (resetTimerRef.current) {
      clearInterval(resetTimerRef.current);
      resetTimerRef.current = null;
    }
    setResetCountdown(null);
  }, []);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (resetTimerRef.current) clearInterval(resetTimerRef.current);
    };
  }, []);

  const handleVote = useCallback(() => {
    api.voteDemoReset().then(res => {
      if (res.success) {
        setDemo(prev => prev ? {
          ...prev,
          votes: { ...prev.votes, has_voted: true, current: prev.votes.current + 1 },
        } : prev);
      }
    });
  }, []);

  if (!enabled || !demo) return null;

  const minutes = Math.floor(localTimer / 60);
  const seconds = localTimer % 60;
  const isLow = localTimer < 300;
  const voteProgress = demo.votes.threshold > 0 ? (demo.votes.current / demo.votes.threshold) * 100 : 0;

  return (
    <motion.div
      initial={{ y: -60, opacity: 0 }}
      animate={{ y: 0, opacity: 1 }}
      className="fixed top-3 left-1/2 -translate-x-1/2 z-50"
    >
      <div className="card shadow-lg border-accent-300/50 dark:border-accent-700/50">
        <button
          onClick={() => setCollapsed(!collapsed)}
          className="flex items-center gap-3 px-3.5 py-2 min-w-[180px] cursor-pointer"
        >
          {/* Demo badge */}
          <span className="flex items-center gap-1 badge badge-primary">
            <Sparkle weight="fill" className="w-2.5 h-2.5 animate-pulse" />
            {t('demo.badge')}
          </span>

          {/* Timer */}
          <span className={`font-mono text-xs font-bold ${isLow ? 'text-red-500 animate-pulse' : 'text-surface-700 dark:text-surface-300'}`}>
            <Timer weight="bold" className="w-3 h-3 inline mr-0.5" />
            {String(minutes).padStart(2, '0')}:{String(seconds).padStart(2, '0')}
          </span>

          {/* Viewers */}
          <span className="flex items-center gap-1 text-[10px] text-surface-500">
            <Eye weight="regular" className="w-3 h-3" />
            {demo.viewers}
          </span>

          {collapsed ? <CaretDown weight="bold" className="w-2.5 h-2.5 text-surface-400" /> : <CaretUp weight="bold" className="w-2.5 h-2.5 text-surface-400" />}
        </button>

        <AnimatePresence>
          {!collapsed && (
            <motion.div
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: 'auto', opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              className="overflow-hidden"
            >
              <div className="px-3.5 pb-2.5 pt-1 border-t border-surface-200/50 dark:border-surface-700/50">
                {isSolo ? (
                  resetCountdown !== null ? (
                    <div className="space-y-1.5">
                      <div className="flex items-center justify-between">
                        <span className="text-[10px] font-medium text-surface-500 uppercase tracking-wider">
                          {t('demo.resetIn', { seconds: resetCountdown })}
                        </span>
                        <button
                          onClick={cancelResetCountdown}
                          className="btn btn-sm btn-ghost text-red-500 hover:bg-red-50 dark:hover:bg-red-950 cursor-pointer"
                        >
                          <X weight="bold" className="w-3 h-3" />
                          {t('demo.cancel')}
                        </button>
                      </div>
                      <div className="h-1.5 bg-surface-200 dark:bg-surface-700 overflow-hidden">
                        <motion.div
                          className="h-full bg-accent-500"
                          initial={{ width: '100%' }}
                          animate={{ width: '0%' }}
                          transition={{ duration: 10, ease: 'linear' }}
                        />
                      </div>
                    </div>
                  ) : (
                    <button
                      onClick={startResetCountdown}
                      className="btn btn-sm btn-primary w-full cursor-pointer"
                    >
                      <ArrowsClockwise weight="bold" className="w-3 h-3" />
                      {t('demo.resetNow')}
                    </button>
                  )
                ) : (
                  <>
                    <div className="flex items-center gap-2 mb-1.5">
                      <div className="flex-1 h-1.5 bg-surface-200 dark:bg-surface-700 overflow-hidden">
                        <motion.div
                          className="h-full bg-primary-500"
                          animate={{ width: `${voteProgress}%` }}
                          transition={{ type: 'spring', stiffness: 100 }}
                        />
                      </div>
                      <span className="text-[10px] font-medium text-surface-500 font-mono">
                        {demo.votes.current}/{demo.votes.threshold}
                      </span>
                    </div>

                    <button
                      onClick={handleVote}
                      disabled={demo.votes.has_voted}
                      className="btn btn-sm btn-primary w-full disabled:opacity-50 cursor-pointer"
                    >
                      <ArrowsClockwise weight="bold" className="w-3 h-3" />
                      {demo.votes.has_voted
                        ? t('demo.votesNeeded', { current: demo.votes.current, needed: demo.votes.threshold })
                        : t('demo.voteReset')}
                    </button>
                  </>
                )}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </motion.div>
  );
}
