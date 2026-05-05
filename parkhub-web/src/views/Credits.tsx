import { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  CoinsIcon, ArrowDownIcon, ArrowUpIcon, ArrowClockwiseIcon,
  TrendUpIcon, SparkleIcon,
} from '@phosphor-icons/react';
import { api, type UserCredits } from '../api/client';
import { useAuth } from '../context/AuthContext';
import { staggerSlow, fadeUp } from '../constants/animations';
import { HeroEyebrow } from '../components/v11/HeroEyebrow';

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

  const container = staggerSlow;
  const item = fadeUp;

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
      {/* v11 SOTA hero — primary tone, page-hero variant. */}
      <motion.section variants={item} className="admin-hero page-hero">
        <div className="admin-hero-left">
          <HeroEyebrow icon={CoinsIcon} label={t('credits.eyebrow', 'BOOKING WALLET')} />
          <h1 className="admin-hero-headline">{t('credits.title')}</h1>
          <p className="admin-hero-sub">{t('credits.subtitle')}</p>
        </div>
      </motion.section>

      {/* Balance display */}
      <motion.div variants={item}>
        <p className="text-sm font-medium text-surface-500 dark:text-surface-400 mb-1">{t('credits.balance')}</p>
        <div className="flex items-baseline gap-2">
          <span className="text-5xl font-bold tracking-normal text-surface-900 dark:text-white">
            {balance}
          </span>
          <span className="text-lg text-surface-400">/ {quota}</span>
        </div>
        <div className="mt-3 h-2 w-full max-w-xs bg-surface-200 dark:bg-surface-700 rounded-full overflow-hidden">
          <div className="h-full bg-primary-600 rounded-full transition-all duration-500" style={{ width: `${percentage}%` }} />
        </div>
        <p className="text-sm text-surface-500 dark:text-surface-400 mt-2">
          {t('credits.creditsPerBooking', { count: 1 })}
        </p>
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
              <SparkleIcon weight="fill" className="w-5 h-5 text-primary-600 dark:text-primary-400" />
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
              <TrendUpIcon weight="fill" className="w-5 h-5 text-orange-600 dark:text-orange-400" />
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
              <ArrowClockwiseIcon weight="fill" className="w-5 h-5 text-emerald-600 dark:text-emerald-400" />
            </div>
          </div>
        </div>
      </motion.div>

      {/* Transaction history */}
      <motion.div variants={item}>
        <h2 className="text-sm font-semibold uppercase tracking-normal text-surface-500 dark:text-surface-400 mb-3">
          {t('credits.history')}
        </h2>

        {!credits?.transactions?.length ? (
          <div className="text-center py-8">
            <CoinsIcon weight="light" className="w-12 h-12 text-surface-200 dark:text-surface-700 mx-auto" />
            <p className="text-surface-500 dark:text-surface-400 mt-3">{t('credits.noTransactions')}</p>
          </div>
        ) : (
          <div className="divide-y divide-surface-100 dark:divide-surface-800">
            {credits.transactions.map(tx => (
              <div key={tx.id} className="flex items-center gap-3 py-3">
                {tx.amount > 0
                  ? <ArrowDownIcon weight="bold" className="w-4 h-4 text-emerald-600 dark:text-emerald-400 flex-shrink-0" />
                  : <ArrowUpIcon weight="bold" className="w-4 h-4 text-red-600 dark:text-red-400 flex-shrink-0" />
                }
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-surface-900 dark:text-white">
                    {t(`credits.${tx.type}`)}
                  </p>
                  {tx.description && (
                    <p className="text-xs text-surface-500 dark:text-surface-400 truncate">{tx.description}</p>
                  )}
                </div>
                <div className="text-right">
                  <span className={`text-sm font-semibold ${tx.amount > 0 ? 'text-emerald-600 dark:text-emerald-400' : 'text-red-600 dark:text-red-400'}`}>
                    {tx.amount > 0 ? '+' : ''}{tx.amount}
                  </span>
                  <p className="text-xs text-surface-500 dark:text-surface-400">
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
