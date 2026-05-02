import { useEffect, useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { CurrencyDollar, ChartBar, DownloadSimple, Question, Buildings } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

interface CostCenterRow {
  cost_center: string;
  department: string;
  user_count: number;
  total_bookings: number;
  total_credits_used: number;
  total_amount: number;
  currency: string;
}

interface DeptRow {
  department: string;
  user_count: number;
  total_bookings: number;
  total_credits_used: number;
  total_amount: number;
  currency: string;
}

export function AdminBillingPage() {
  const { t } = useTranslation();
  const [ccData, setCcData] = useState<CostCenterRow[]>([]);
  const [deptData, setDeptData] = useState<DeptRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [tab, setTab] = useState<'cost-center' | 'department'>('cost-center');
  const [showHelp, setShowHelp] = useState(false);

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const [ccRes, deptRes] = await Promise.all([
        fetch('/api/v1/admin/billing/by-cost-center').then(r => r.json()),
        fetch('/api/v1/admin/billing/by-department').then(r => r.json()),
      ]);
      if (ccRes.success) setCcData(ccRes.data || []);
      if (deptRes.success) setDeptData(deptRes.data || []);
    } catch { /* ignore */ }
    setLoading(false);
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  async function handleExport() {
    try {
      const res = await fetch('/api/v1/admin/billing/export');
      const blob = await res.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `billing-export-${new Date().toISOString().slice(0, 10)}.csv`;
      a.click();
      URL.revokeObjectURL(url);
      toast.success(t('billing.exported', 'CSV exported'));
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
  }

  const totalAmount = ccData.reduce((sum, r) => sum + r.total_amount, 0);
  const totalBookings = ccData.reduce((sum, r) => sum + r.total_bookings, 0);
  const totalUsers = ccData.reduce((sum, r) => sum + r.user_count, 0);

  if (loading) {
    return (
      <div className="space-y-4">
        {Array.from({ length: 3 }, (_, i) => <div key={i} className="h-24 skeleton rounded-xl" />)}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* v11 SOTA hero — replaces plain h2/subtitle. Pairs with the
          .admin-hero shipped on the Admin shell. Emerald tone matches
          the Cost Center Billing identity. */}
      <section className="admin-hero admin-hero--emerald">
        <div className="admin-hero-left">
          <div className="admin-hero-eyebrow">
            <span className="admin-hero-dot" aria-hidden="true"></span>
            <CurrencyDollar weight="bold" className="w-3.5 h-3.5" />
            {t('billing.eyebrow', 'COST CENTER BILLING')}
          </div>
          <h1 className="admin-hero-headline">{t('billing.title', 'Cost Center Billing')}</h1>
          <p className="admin-hero-sub">
            {t('billing.subtitle', 'Billing breakdown by cost center and department')}
          </p>
        </div>
        <div className="admin-hero-actions">
          <button
            onClick={() => setShowHelp(!showHelp)}
            className="admin-hero-iconbtn"
            aria-label={t('billing.help_toggle', 'Toggle help')}
          >
            <Question weight="bold" className="w-4 h-4" />
          </button>
          <button onClick={handleExport} className="admin-hero-action" data-testid="export-btn">
            <DownloadSimple weight="bold" className="w-4 h-4" />
            {t('billing.export', 'CSV Export')}
          </button>
        </div>
      </section>

      {/* Help */}
      {showHelp && (
        <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} className="bg-emerald-50 dark:bg-emerald-900/20 border border-emerald-200 dark:border-emerald-800 rounded-xl p-4">
          <p className="text-sm text-emerald-800 dark:text-emerald-300">
            {t('billing.help', 'This module provides billing analytics by cost center and department. Track parking spending, credit usage, and generate CSV exports for finance teams. Assign cost centers and departments in user profiles.')}
          </p>
        </motion.div>
      )}

      {/* v11 SOTA summary meters — emerald/blue/purple tones for visual rhythm. */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4" data-testid="billing-summary">
        <SummaryCard tone="success" label={t('billing.totalSpending', 'Total Spending')} value={`EUR ${totalAmount.toFixed(2)}`} icon={<CurrencyDollar weight="bold" className="w-3.5 h-3.5" />} />
        <SummaryCard tone="info" label={t('billing.totalBookings', 'Total Bookings')} value={totalBookings} icon={<ChartBar weight="bold" className="w-3.5 h-3.5" />} />
        <SummaryCard tone="accent" label={t('billing.totalUsers', 'Total Users')} value={totalUsers} icon={<Buildings weight="bold" className="w-3.5 h-3.5" />} />
      </div>

      {/* Tab switcher */}
      <div className="flex gap-1 bg-surface-100 dark:bg-surface-800 rounded-lg p-1" data-testid="billing-tabs">
        <button
          onClick={() => setTab('cost-center')}
          className={`flex-1 px-3 py-2 rounded-md text-sm font-medium transition-colors ${tab === 'cost-center' ? 'bg-white dark:bg-surface-700 text-surface-900 dark:text-white shadow-sm' : 'text-surface-500 dark:text-surface-400'}`}
        >
          {t('billing.byCostCenter', 'By Cost Center')}
        </button>
        <button
          onClick={() => setTab('department')}
          className={`flex-1 px-3 py-2 rounded-md text-sm font-medium transition-colors ${tab === 'department' ? 'bg-white dark:bg-surface-700 text-surface-900 dark:text-white shadow-sm' : 'text-surface-500 dark:text-surface-400'}`}
        >
          {t('billing.byDepartment', 'By Department')}
        </button>
      </div>

      {/* Table */}
      <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 overflow-hidden" data-testid="billing-table">
        <table className="w-full">
          <thead>
            <tr className="border-b border-surface-100 dark:border-surface-800 bg-surface-50 dark:bg-surface-900">
              <th className="text-left px-4 py-3 text-xs font-semibold text-surface-500 uppercase">{tab === 'cost-center' ? t('billing.costCenter', 'Cost Center') : t('billing.department', 'Department')}</th>
              {tab === 'cost-center' && <th className="text-left px-4 py-3 text-xs font-semibold text-surface-500 uppercase">{t('billing.department', 'Department')}</th>}
              <th className="text-right px-4 py-3 text-xs font-semibold text-surface-500 uppercase">{t('billing.users', 'Users')}</th>
              <th className="text-right px-4 py-3 text-xs font-semibold text-surface-500 uppercase">{t('billing.bookings', 'Bookings')}</th>
              <th className="text-right px-4 py-3 text-xs font-semibold text-surface-500 uppercase">{t('billing.credits', 'Credits')}</th>
              <th className="text-right px-4 py-3 text-xs font-semibold text-surface-500 uppercase">{t('billing.amount', 'Amount')}</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-surface-100 dark:divide-surface-800">
            {tab === 'cost-center' ? (
              ccData.length === 0 ? (
                <tr><td colSpan={6} className="px-4 py-8 text-center text-sm text-surface-500">{t('billing.empty', 'No billing data')}</td></tr>
              ) : ccData.map((r, i) => (
                <tr key={i} data-testid="billing-row" className="hover:bg-surface-50 dark:hover:bg-surface-800/50">
                  <td className="px-4 py-3 text-sm font-medium text-surface-900 dark:text-white">{r.cost_center || '-'}</td>
                  <td className="px-4 py-3 text-sm text-surface-600 dark:text-surface-400">{r.department || '-'}</td>
                  <td className="px-4 py-3 text-sm text-right text-surface-600 dark:text-surface-400">{r.user_count}</td>
                  <td className="px-4 py-3 text-sm text-right text-surface-600 dark:text-surface-400">{r.total_bookings}</td>
                  <td className="px-4 py-3 text-sm text-right text-surface-600 dark:text-surface-400">{r.total_credits_used}</td>
                  <td className="px-4 py-3 text-sm text-right font-semibold text-surface-900 dark:text-white">{r.currency} {r.total_amount.toFixed(2)}</td>
                </tr>
              ))
            ) : (
              deptData.length === 0 ? (
                <tr><td colSpan={5} className="px-4 py-8 text-center text-sm text-surface-500">{t('billing.empty', 'No billing data')}</td></tr>
              ) : deptData.map((r, i) => (
                <tr key={i} data-testid="billing-row" className="hover:bg-surface-50 dark:hover:bg-surface-800/50">
                  <td className="px-4 py-3 text-sm font-medium text-surface-900 dark:text-white">{r.department || '-'}</td>
                  <td className="px-4 py-3 text-sm text-right text-surface-600 dark:text-surface-400">{r.user_count}</td>
                  <td className="px-4 py-3 text-sm text-right text-surface-600 dark:text-surface-400">{r.total_bookings}</td>
                  <td className="px-4 py-3 text-sm text-right text-surface-600 dark:text-surface-400">{r.total_credits_used}</td>
                  <td className="px-4 py-3 text-sm text-right font-semibold text-surface-900 dark:text-white">{r.currency} {r.total_amount.toFixed(2)}</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}

function SummaryCard({ label, value, icon, tone = 'primary' }: {
  label: string;
  value: string | number;
  icon: React.ReactNode;
  tone?: 'primary' | 'accent' | 'info' | 'success' | 'warn' | 'danger';
}) {
  // v11 SOTA meter from PR #490 — same pattern as AdminReports stat cards.
  return (
    <div className={`v11-meter v11-meter--${tone}`}>
      <div className="v11-meter-eyebrow">
        {icon}
        {label}
      </div>
      <div className="v11-meter-value">{value}</div>
    </div>
  );
}
