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

  const container = { hidden: { opacity: 0 }, show: { opacity: 1, transition: { staggerChildren: 0.08 } } };
  const item = { hidden: { opacity: 0, y: 20 }, show: { opacity: 1, y: 0 } };

  if (loading) return (
    <div className="space-y-6">
      <div className="h-10 w-64 skeleton rounded-xl" />
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        {[1,2,3].map(i => <div key={i} className="h-32 skeleton rounded-2xl" />)}
      </div>
      <div className="h-64 skeleton rounded-2xl" />
    </div>
  );

  const balance = credits?.balance ?? user?.credits_balance ?? 0;
  const quota = credits?.monthly_quota ?? user?.credits_monthly_quota ?? 10;
  const used = quota - balance;
  const percentage = quota > 0 ? Math.round((balance / quota) * 100) : 0;

  return (
    <motion.div variants={container} initial="hidden" animate="show" className="space-y-8">
      {/* Header */}
      <motion.div variants={item}>
        <h1 className="text-2xl font-bold text-surface-900 dark:text-white flex items-center gap-3">
          <CoinVertical weight="bold" className="w-7 h-7 text-accent-500" />
          {t('credits.title')}
        </h1>
        <p className="text-surface-500 dark:text-surface-400 mt-1">{t('credits.subtitle')}</p>
      </motion.div>

      {/* Balance display — hero card */}
      <motion.div variants={item} className="card p-8 relative overflow-hidden">
        <div className="absolute top-0 right-0 w-40 h-40 bg-gradient-to-bl from-accent-400/10 to-transparent rounded-bl-full" />
        <div className="relative z-10 flex flex-col sm:flex-row items-start sm:items-center gap-6">
          <div className="flex-1">
            <p className="text-sm font-medium text-surface-500 dark:text-surface-400 mb-1">{t('credits.balance')}</p>
            <div className="flex items-baseline gap-3">
              <span className="text-5xl sm:text-6xl font-black tracking-tight text-accent-600 dark:text-accent-400">
                {balance}
              </span>
              <span className="text-lg text-surface-400">/ {quota}</span>
            </div>
            <p className="text-sm text-surface-500 dark:text-surface-400 mt-2">
              {t('credits.creditsPerBooking', { count: 1 })}
            </p>
          </div>

          {/* Circular progress */}
          <div className="relative w-28 h-28 flex-shrink-0">
            <svg className="w-28 h-28 -rotate-90" viewBox="0 0 120 120">
              <circle cx="60" cy="60" r="52" stroke="currentColor" strokeWidth="8" fill="none"
                className="text-surface-200 dark:text-surface-700" />
              <circle cx="60" cy="60" r="52" stroke="currentColor" strokeWidth="8" fill="none"
                strokeDasharray={`${2 * Math.PI * 52}`}
                strokeDashoffset={`${2 * Math.PI * 52 * (1 - percentage / 100)}`}
                strokeLinecap="round"
                className="text-accent-500 transition-all duration-1000 ease-out" />
            </svg>
            <div className="absolute inset-0 flex items-center justify-center">
              <span className="text-lg font-bold text-surface-900 dark:text-white">{percentage}%</span>
            </div>
          </div>
        </div>
      </motion.div>

      {/* Stat cards */}
      <motion.div variants={item} className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        <div className="stat-card">
          <div className="flex items-start justify-between">
            <div>
              <p className="text-sm font-medium text-surface-500 dark:text-surface-400">{t('credits.monthlyQuota')}</p>
              <p className="mt-2 stat-value text-primary-600 dark:text-primary-400">{quota}</p>
            </div>
            <div className="w-10 h-10 bg-primary-100 dark:bg-primary-900/30 rounded-xl flex items-center justify-center">
              <Gauge weight="bold" className="w-5 h-5 text-primary-600 dark:text-primary-400" />
            </div>
          </div>
        </div>

        <div className="stat-card">
          <div className="flex items-start justify-between">
            <div>
              <p className="text-sm font-medium text-surface-500 dark:text-surface-400">{t('credits.used')}</p>
              <p className="mt-2 stat-value text-orange-600 dark:text-orange-400">{used}</p>
            </div>
            <div className="w-10 h-10 bg-orange-100 dark:bg-orange-900/30 rounded-xl flex items-center justify-center">
              <TrendUp weight="fill" className="w-5 h-5 text-orange-600 dark:text-orange-400" />
            </div>
          </div>
        </div>

        <div className="stat-card">
          <div className="flex items-start justify-between">
            <div>
              <p className="text-sm font-medium text-surface-500 dark:text-surface-400">{t('credits.lastRefill')}</p>
              <p className="mt-2 text-lg font-bold text-surface-900 dark:text-white">
                {credits?.last_refilled ? new Date(credits.last_refilled).toLocaleDateString() : '—'}
              </p>
            </div>
            <div className="w-10 h-10 bg-emerald-100 dark:bg-emerald-900/30 rounded-xl flex items-center justify-center">
              <ArrowClockwise weight="fill" className="w-5 h-5 text-emerald-600 dark:text-emerald-400" />
            </div>
          </div>
        </div>
      </motion.div>

      {/* Transaction history */}
      <motion.div variants={item} className="card p-6">
        <h2 className="text-lg font-semibold text-surface-900 dark:text-white mb-4 flex items-center gap-2">
          <CalendarCheck weight="fill" className="w-5 h-5 text-primary-500" />
          {t('credits.history')}
        </h2>

        {!credits?.transactions?.length ? (
          <div className="text-center py-8">
            <CoinVertical weight="light" className="w-12 h-12 text-surface-200 dark:text-surface-700 mx-auto mb-3" />
            <p className="text-surface-500 dark:text-surface-400">{t('credits.noTransactions')}</p>
          </div>
        ) : (
          <div className="space-y-2">
            {credits.transactions.map(tx => (
              <div key={tx.id} className="flex items-center gap-4 p-3 bg-surface-50 dark:bg-surface-800/50 rounded-xl">
                <div className={`w-9 h-9 rounded-lg flex items-center justify-center ${
                  tx.amount > 0
                    ? 'bg-emerald-100 dark:bg-emerald-900/30'
                    : 'bg-red-100 dark:bg-red-900/30'
                }`}>
                  {tx.amount > 0
                    ? <ArrowDown weight="bold" className="w-4 h-4 text-emerald-600 dark:text-emerald-400" />
                    : <ArrowUp weight="bold" className="w-4 h-4 text-red-600 dark:text-red-400" />
                  }
                </div>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-surface-900 dark:text-white">
                    {t(`credits.${tx.type}`)}
                  </p>
                  {tx.description && (
                    <p className="text-xs text-surface-500 dark:text-surface-400 truncate">{tx.description}</p>
                  )}
                </div>
                <div className="text-right">
                  <span className={`text-sm font-bold ${tx.amount > 0 ? 'text-emerald-600 dark:text-emerald-400' : 'text-red-600 dark:text-red-400'}`}>
                    {tx.amount > 0 ? '+' : ''}{tx.amount}
                  </span>
                  <p className="text-xs text-surface-400">
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
