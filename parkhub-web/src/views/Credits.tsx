import { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  CoinVertical, ArrowDown, ArrowUp, ArrowClockwise, CalendarCheck,
  TrendUp, Gauge,
} from '@phosphor-icons/react';
import { api, type UserCredits } from '../api/client';
import { useAuth } from '../context/AuthContext';

export function CreditsPage() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const [credits, setCredits] = useState<UserCredits | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.getUserCredits().then(res => {
      if (res.success && res.data) setCredits(res.data);
    }).finally(() => setLoading(false));
  }, []);

  const container = { hidden: { opacity: 0 }, show: { opacity: 1, transition: { staggerChildren: 0.06 } } };
  const item = { hidden: { opacity: 0, y: 12 }, show: { opacity: 1, y: 0, transition: { ease: [0.22, 1, 0.36, 1] as const } } };

  if (loading) return (
    <div className="space-y-5">
      <div className="h-8 w-64 skeleton" />
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
        {[1,2,3].map(i => <div key={i} className="h-28 skeleton" />)}
      </div>
      <div className="h-56 skeleton" />
    </div>
  );

  const balance = credits?.balance ?? user?.credits_balance ?? 0;
  const quota = credits?.monthly_quota ?? user?.credits_monthly_quota ?? 10;
  const used = quota - balance;
  const percentage = quota > 0 ? Math.round((balance / quota) * 100) : 0;

  return (
    <motion.div variants={container} initial="hidden" animate="show" className="space-y-6">
      {/* Header */}
      <motion.div variants={item}>
        <p className="text-xs font-semibold text-accent-600 dark:text-accent-400 uppercase tracking-widest mb-1">
          {t('nav.credits')}
        </p>
        <h1 className="text-2xl font-bold text-surface-900 dark:text-white tracking-tight flex items-center gap-2">
          {t('credits.title')}
        </h1>
        <p className="text-surface-500 dark:text-surface-400 mt-0.5 text-sm">{t('credits.subtitle')}</p>
      </motion.div>

      {/* Balance display — hero card */}
      <motion.div variants={item} className="card p-6 relative overflow-hidden">
        <div className="absolute top-0 right-0 w-32 h-32 bg-gradient-to-bl from-accent-400/8 to-transparent" />
        <div className="absolute bottom-0 left-0 right-0 h-[2px] bg-gradient-to-r from-transparent via-accent-500/40 to-transparent" />
        <div className="relative z-10 flex flex-col sm:flex-row items-start sm:items-center gap-6">
          <div className="flex-1">
            <p className="text-[11px] font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-widest mb-2">{t('credits.balance')}</p>
            <div className="flex items-baseline gap-2">
              <span className="text-5xl sm:text-6xl font-black tracking-tighter text-accent-600 dark:text-accent-400">
                {balance}
              </span>
              <span className="text-lg text-surface-400 font-light">/ {quota}</span>
            </div>
            <p className="text-xs text-surface-500 dark:text-surface-400 mt-2">
              {t('credits.creditsPerBooking', { count: 1 })}
            </p>
          </div>

          {/* Circular progress */}
          <div className="relative w-24 h-24 flex-shrink-0">
            <svg className="w-24 h-24 -rotate-90" viewBox="0 0 120 120">
              <circle cx="60" cy="60" r="52" stroke="currentColor" strokeWidth="6" fill="none"
                className="text-surface-200 dark:text-surface-700" />
              <circle cx="60" cy="60" r="52" stroke="currentColor" strokeWidth="6" fill="none"
                strokeDasharray={`${2 * Math.PI * 52}`}
                strokeDashoffset={`${2 * Math.PI * 52 * (1 - percentage / 100)}`}
                strokeLinecap="butt"
                className="text-accent-500 transition-all duration-1000 ease-out" />
            </svg>
            <div className="absolute inset-0 flex items-center justify-center">
              <span className="text-base font-bold text-surface-900 dark:text-white font-[Outfit]">{percentage}%</span>
            </div>
          </div>
        </div>
      </motion.div>

      {/* Stat cards */}
      <motion.div variants={item} className="grid grid-cols-1 sm:grid-cols-3 gap-3">
        <div className="stat-card">
          <div className="flex items-start justify-between">
            <div>
              <p className="text-[11px] font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider mb-2">{t('credits.monthlyQuota')}</p>
              <p className="stat-value text-primary-700 dark:text-primary-400">{quota}</p>
            </div>
            <div className="w-9 h-9 bg-primary-100 dark:bg-primary-900/20 flex items-center justify-center">
              <Gauge weight="bold" className="w-4 h-4 text-primary-600 dark:text-primary-400" />
            </div>
          </div>
        </div>

        <div className="stat-card">
          <div className="flex items-start justify-between">
            <div>
              <p className="text-[11px] font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider mb-2">{t('credits.used')}</p>
              <p className="stat-value text-accent-600 dark:text-accent-400">{used}</p>
            </div>
            <div className="w-9 h-9 bg-accent-100 dark:bg-accent-900/20 flex items-center justify-center">
              <TrendUp weight="fill" className="w-4 h-4 text-accent-600 dark:text-accent-400" />
            </div>
          </div>
        </div>

        <div className="stat-card">
          <div className="flex items-start justify-between">
            <div>
              <p className="text-[11px] font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider mb-2">{t('credits.lastRefill')}</p>
              <p className="text-base font-bold text-surface-900 dark:text-white font-[Outfit]">
                {credits?.last_refilled ? new Date(credits.last_refilled).toLocaleDateString() : '—'}
              </p>
            </div>
            <div className="w-9 h-9 bg-emerald-50 dark:bg-emerald-900/20 flex items-center justify-center">
              <ArrowClockwise weight="fill" className="w-4 h-4 text-emerald-600 dark:text-emerald-400" />
            </div>
          </div>
        </div>
      </motion.div>

      {/* Transaction history */}
      <motion.div variants={item} className="card p-5">
        <h2 className="text-sm font-semibold text-surface-900 dark:text-white mb-4 flex items-center gap-2 uppercase tracking-wide">
          <CalendarCheck weight="fill" className="w-4 h-4 text-primary-500" />
          {t('credits.history')}
        </h2>

        {!credits?.transactions?.length ? (
          <div className="text-center py-8">
            <CoinVertical weight="light" className="w-12 h-12 text-surface-200 dark:text-surface-700 mx-auto mb-3" />
            <p className="text-surface-500 dark:text-surface-400 text-sm">{t('credits.noTransactions')}</p>
          </div>
        ) : (
          <div className="space-y-1.5">
            {credits.transactions.map(tx => (
              <div key={tx.id} className="flex items-center gap-3 p-2.5 bg-surface-50 dark:bg-surface-800/40 rounded-md hover:bg-surface-100 dark:hover:bg-surface-800/60 transition-colors">
                <div className={`w-8 h-8 flex items-center justify-center ${
                  tx.amount > 0
                    ? 'bg-emerald-100 dark:bg-emerald-900/20'
                    : 'bg-red-100 dark:bg-red-900/20'
                }`}>
                  {tx.amount > 0
                    ? <ArrowDown weight="bold" className="w-3.5 h-3.5 text-emerald-600 dark:text-emerald-400" />
                    : <ArrowUp weight="bold" className="w-3.5 h-3.5 text-red-600 dark:text-red-400" />
                  }
                </div>
                <div className="flex-1 min-w-0">
                  <p className="text-xs font-medium text-surface-900 dark:text-white">
                    {t(`credits.${tx.type}`)}
                  </p>
                  {tx.description && (
                    <p className="text-[11px] text-surface-500 dark:text-surface-400 truncate">{tx.description}</p>
                  )}
                </div>
                <div className="text-right">
                  <span className={`text-xs font-bold font-mono ${tx.amount > 0 ? 'text-emerald-600 dark:text-emerald-400' : 'text-red-600 dark:text-red-400'}`}>
                    {tx.amount > 0 ? '+' : ''}{tx.amount}
                  </span>
                  <p className="text-[10px] text-surface-400 font-mono">
                    {new Date(tx.created_at).toLocaleDateString()}
                  </p>
                </div>
              </div>
            ))}
          </div>
        )}
      </motion.div>
    </motion.div>
  );
}
