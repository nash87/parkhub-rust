import { useState, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Timer, ArrowsClockwise, Eye, CaretUp, CaretDown, Sparkle } from '@phosphor-icons/react';

const API_BASE = import.meta.env.VITE_API_URL || '';

interface DemoStatus {
  enabled: boolean;
  timer: { remaining: number; duration: number };
  votes: { current: number; threshold: number; has_voted: boolean };
  viewers: number;
}

export function DemoOverlay() {
  const [status, setStatus] = useState<DemoStatus | null>(null);
  const [collapsed, setCollapsed] = useState(false);
  const [voting, setVoting] = useState(false);
  const [localRemaining, setLocalRemaining] = useState<number | null>(null);
  const [demoEnabled, setDemoEnabled] = useState<boolean>(
    import.meta.env.VITE_DEMO_MODE === 'true'
  );

  // Check demo config from backend
  useEffect(() => {
    if (demoEnabled) return;
    fetch(`${API_BASE}/api/v1/demo/config`)
      .then(r => r.json())
      .then(data => { if (data.demo_mode) setDemoEnabled(true); })
      .catch(() => {});
  }, [demoEnabled]);

  // Fetch status
  const fetchStatus = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/api/v1/demo/status`);
      if (!res.ok) return;
      const data: DemoStatus = await res.json();
      setStatus(data);
      setLocalRemaining(data.timer.remaining);
    } catch {
      // silently ignore fetch errors
    }
  }, []);

  // Poll every 15s
  useEffect(() => {
    if (!demoEnabled) return;
    fetchStatus();
    const interval = setInterval(fetchStatus, 15000);
    return () => clearInterval(interval);
  }, [demoEnabled, fetchStatus]);

  // Local countdown
  useEffect(() => {
    if (localRemaining === null) return;
    const interval = setInterval(() => {
      setLocalRemaining(prev => (prev === null || prev <= 0) ? 0 : prev - 1);
    }, 1000);
    return () => clearInterval(interval);
  }, [localRemaining !== null]);

  // Vote
  const handleVote = async () => {
    if (voting || status?.votes.has_voted) return;
    setVoting(true);
    try {
      const res = await fetch(`${API_BASE}/api/v1/demo/vote`, { method: 'POST' });
      const data = await res.json();
      if (data.reset) {
        setStatus(prev => prev ? { ...prev, votes: { ...prev.votes, current: 0 } } : prev);
        setTimeout(() => window.location.reload(), 2000);
      } else {
        setStatus(prev => prev ? {
          ...prev,
          votes: { ...prev.votes, current: data.votes, has_voted: true }
        } : prev);
      }
    } catch {
      // silently ignore vote errors
    } finally {
      setVoting(false);
    }
  };

  if (!demoEnabled || !status) return null;

  const remaining = localRemaining ?? status.timer.remaining;
  const minutes = Math.floor(remaining / 60);
  const seconds = remaining % 60;
  const timerStr = `${String(minutes).padStart(2, '0')}:${String(seconds).padStart(2, '0')}`;
  const isUrgent = remaining < 300;
  const voteProgress = (status.votes.current / status.votes.threshold) * 100;

  return (
    <AnimatePresence>
      <motion.div
        initial={{ y: -80, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        exit={{ y: -80, opacity: 0 }}
        transition={{ type: 'spring', damping: 25, stiffness: 300 }}
        className="fixed top-3 left-1/2 -translate-x-1/2 z-[60]"
      >
        {collapsed ? (
          <motion.button
            layout
            onClick={() => setCollapsed(false)}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-full
                       bg-primary-600/90 backdrop-blur-xl text-white
                       text-xs font-bold shadow-lg shadow-primary-600/25
                       hover:bg-primary-500/90 transition-colors"
          >
            <Sparkle weight="fill" className="w-3.5 h-3.5" />
            DEMO
            <span className="font-mono text-[10px] opacity-80">{timerStr}</span>
            <CaretDown weight="bold" className="w-3 h-3" />
          </motion.button>
        ) : (
          <motion.div
            layout
            className="flex items-center gap-3 px-4 py-2 rounded-2xl
                       bg-white/80 dark:bg-gray-900/80 backdrop-blur-xl
                       border border-gray-200/50 dark:border-gray-700/50
                       shadow-xl shadow-black/10 dark:shadow-black/30
                       text-sm"
          >
            {/* DEMO badge */}
            <div className="flex items-center gap-1 px-2 py-0.5 rounded-lg
                            bg-primary-600 text-white font-bold text-xs tracking-wide">
              <Sparkle weight="fill" className="w-3.5 h-3.5 animate-pulse" />
              DEMO
            </div>

            <div className="w-px h-5 bg-gray-200 dark:bg-gray-700" />

            {/* Timer */}
            <div className={`flex items-center gap-1.5 font-mono font-semibold tabular-nums
                            ${isUrgent ? 'text-red-500 animate-pulse' : 'text-gray-700 dark:text-gray-300'}`}>
              <Timer weight="bold" className="w-4 h-4" />
              {timerStr}
            </div>

            <div className="w-px h-5 bg-gray-200 dark:bg-gray-700" />

            {/* Viewers */}
            <div className="flex items-center gap-1.5 text-gray-500 dark:text-gray-400">
              <Eye weight="bold" className="w-4 h-4" />
              <span className="text-xs font-medium">{status.viewers}</span>
            </div>

            <div className="w-px h-5 bg-gray-200 dark:bg-gray-700" />

            {/* Vote to reset */}
            <button
              onClick={handleVote}
              disabled={voting || status.votes.has_voted}
              className={`flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-xs font-medium
                         transition-all duration-200
                         ${status.votes.has_voted
                           ? 'bg-emerald-100 dark:bg-emerald-900/30 text-emerald-600 dark:text-emerald-400 cursor-default'
                           : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-primary-100 dark:hover:bg-primary-900/30 hover:text-primary-700 dark:hover:text-primary-400'
                         }`}
            >
              <ArrowsClockwise weight="bold" className={`w-3.5 h-3.5 ${voting ? 'animate-spin' : ''}`} />
              <span>{status.votes.has_voted ? 'Voted' : 'Reset'}</span>
              <div className="w-10 h-1.5 rounded-full bg-gray-200 dark:bg-gray-700 overflow-hidden">
                <div
                  className="h-full rounded-full bg-primary-600 transition-all duration-500"
                  style={{ width: `${voteProgress}%` }}
                />
              </div>
              <span className="text-[10px] opacity-60">
                {status.votes.current}/{status.votes.threshold}
              </span>
            </button>

            {/* Collapse */}
            <button
              onClick={() => setCollapsed(true)}
              className="p-0.5 rounded text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
              aria-label="Minimize demo bar"
            >
              <CaretUp weight="bold" className="w-3.5 h-3.5" />
            </button>
          </motion.div>
        )}
      </motion.div>
    </AnimatePresence>
  );
}
