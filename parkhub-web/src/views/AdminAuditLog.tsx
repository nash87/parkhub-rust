import { useEffect, useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { ClockCounterClockwise, DownloadSimple, FunnelSimple, MagnifyingGlass, FileCsv, FileDoc, FileJs, Question, CircleNotch } from '@phosphor-icons/react';
import { api, type AuditLogEntry } from '../api/client';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

const ACTION_TYPES = [
  'LoginSuccess', 'LoginFailed', 'Logout',
  'BookingCreated', 'BookingCancelled',
  'UserCreated', 'UserDeleted',
  'LotCreated', 'SettingsChanged', 'ConfigChanged',
  'PasswordChanged', 'TwoFactorEnabled',
  'ApiKeyCreated', 'PaymentCompleted',
];

type ActionCategory = 'auth' | 'create' | 'update' | 'delete' | 'other';

function getActionCategory(eventType: string): ActionCategory {
  const lower = eventType.toLowerCase();
  if (lower.includes('login') || lower.includes('logout') || lower.includes('password') || lower.includes('twofactor') || lower.includes('apikey') || lower.includes('token')) return 'auth';
  if (lower.includes('created') || lower.includes('added') || lower.includes('enabled')) return 'create';
  if (lower.includes('updated') || lower.includes('changed') || lower.includes('settings') || lower.includes('config')) return 'update';
  if (lower.includes('deleted') || lower.includes('cancelled') || lower.includes('removed') || lower.includes('revoked') || lower.includes('disabled')) return 'delete';
  return 'other';
}

const categoryColors: Record<ActionCategory, string> = {
  create: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400',
  update: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  delete: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  auth: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
  other: 'bg-surface-100 text-surface-700 dark:bg-surface-800 dark:text-surface-300',
};

function formatEventType(eventType: string): string {
  return eventType.replace(/([A-Z])/g, ' $1').trim();
}

export function AdminAuditLogPage() {
  const { t } = useTranslation();
  const [entries, setEntries] = useState<AuditLogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [total, setTotal] = useState(0);
  const [actionFilter, setActionFilter] = useState('');
  const [userFilter, setUserFilter] = useState('');
  const [dateFrom, setDateFrom] = useState('');
  const [dateTo, setDateTo] = useState('');

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const res = await api.getAuditLog({
        page,
        per_page: 25,
        action: actionFilter || undefined,
        user: userFilter || undefined,
        from: dateFrom || undefined,
        to: dateTo || undefined,
      });
      if (res.success && res.data) {
        setEntries(res.data.entries);
        setTotalPages(res.data.total_pages);
        setTotal(res.data.total);
      }
    } catch {
      toast.error(t('common.error'));
    } finally {
      setLoading(false);
    }
  }, [page, actionFilter, userFilter, dateFrom, dateTo, t]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  const [showExportDialog, setShowExportDialog] = useState(false);
  const [exportFormat, setExportFormat] = useState<'csv' | 'json' | 'pdf'>('csv');
  const [exporting, setExporting] = useState(false);

  function handleExport() {
    const url = api.exportAuditLog({
      action: actionFilter || undefined,
      user: userFilter || undefined,
      from: dateFrom || undefined,
      to: dateTo || undefined,
    });
    window.open(url, '_blank');
  }

  async function handleEnhancedExport() {
    setExporting(true);
    try {
      const params = new URLSearchParams();
      params.set('format', exportFormat);
      if (actionFilter) params.set('action', actionFilter);
      if (userFilter) params.set('user_id', userFilter);
      if (dateFrom) params.set('from', dateFrom);
      if (dateTo) params.set('to', dateTo);

      const res = await fetch(`/api/v1/admin/audit-log/export/enhanced?${params}`).then(r => r.json());
      if (res.success && res.data?.download_url) {
        window.open(res.data.download_url, '_blank');
        toast.success(t('auditLog.exportStarted', 'Export started'));
        setShowExportDialog(false);
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } catch {
      toast.error(t('common.error'));
    } finally {
      setExporting(false);
    }
  }

  function handleFilterApply() {
    setPage(1);
    loadData();
  }

  if (loading && entries.length === 0) return (
    <div className="space-y-4">
      <div className="h-8 w-48 skeleton rounded-lg" />
      <div className="h-12 skeleton rounded-xl" />
      <div className="space-y-2">
        {[1, 2, 3, 4, 5].map(i => <div key={i} className="h-16 skeleton rounded-xl" />)}
      </div>
    </div>
  );

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <ClockCounterClockwise weight="duotone" className="w-6 h-6 text-primary-500" />
          <div>
            <h2 className="text-xl font-bold text-surface-900 dark:text-white">{t('auditLog.title', 'Audit Log')}</h2>
            <p className="text-sm text-surface-500">{t('auditLog.totalEntries', '{{count}} entries', { count: total })}</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={handleExport}
            className="flex items-center gap-2 px-4 py-2 bg-surface-100 dark:bg-surface-700 text-surface-700 dark:text-surface-300 rounded-xl text-sm font-medium hover:bg-surface-200 dark:hover:bg-surface-600 transition-colors"
            data-testid="export-csv-btn"
          >
            <FileCsv weight="bold" className="w-4 h-4" />
            {t('auditLog.exportCsv', 'CSV')}
          </button>
          <button
            onClick={() => setShowExportDialog(d => !d)}
            className="flex items-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-xl text-sm font-medium hover:bg-primary-700 transition-colors"
            data-testid="export-enhanced-btn"
          >
            <DownloadSimple weight="bold" className="w-4 h-4" />
            {t('auditLog.advancedExport', 'Advanced Export')}
          </button>
        </div>
      </div>

      {/* Enhanced Export Dialog */}
      {showExportDialog && (
        <div className="bg-white dark:bg-surface-800 rounded-2xl border border-surface-200 dark:border-surface-700 p-5 space-y-4" data-testid="export-dialog">
          <h3 className="font-semibold text-surface-900 dark:text-white flex items-center gap-2">
            <DownloadSimple className="w-5 h-5 text-primary-500" />
            {t('auditLog.advancedExport', 'Advanced Export')}
          </h3>
          <p className="text-sm text-surface-500 dark:text-surface-400">
            {t('auditLog.exportHelp', 'Export audit log in your preferred format. Current filters will be applied. Download link expires in 5 minutes.')}
          </p>
          <div className="flex gap-3">
            {(['csv', 'json', 'pdf'] as const).map(fmt => (
              <button
                key={fmt}
                onClick={() => setExportFormat(fmt)}
                className={`flex items-center gap-2 px-4 py-3 rounded-xl border transition-colors ${
                  exportFormat === fmt
                    ? 'bg-primary-50 dark:bg-primary-900/20 border-primary-300 dark:border-primary-700 text-primary-700 dark:text-primary-300'
                    : 'bg-surface-50 dark:bg-surface-900 border-surface-200 dark:border-surface-700 text-surface-600 dark:text-surface-400'
                }`}
                data-testid={`format-${fmt}`}
              >
                {fmt === 'csv' && <FileCsv className="w-5 h-5" />}
                {fmt === 'json' && <FileJs className="w-5 h-5" />}
                {fmt === 'pdf' && <FileDoc className="w-5 h-5" />}
                <span className="font-medium uppercase">{fmt}</span>
              </button>
            ))}
          </div>
          <div className="flex gap-2 pt-2">
            <button
              onClick={handleEnhancedExport}
              disabled={exporting}
              className="flex items-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-xl text-sm font-medium hover:bg-primary-700 transition-colors disabled:opacity-50"
              data-testid="export-download-btn"
            >
              {exporting ? <CircleNotch className="w-4 h-4 animate-spin" /> : <DownloadSimple className="w-4 h-4" />}
              {exporting ? t('auditLog.exporting', 'Exporting...') : t('auditLog.download', 'Download')}
            </button>
            <button
              onClick={() => setShowExportDialog(false)}
              className="px-4 py-2 bg-surface-100 dark:bg-surface-700 text-surface-700 dark:text-surface-300 rounded-xl text-sm font-medium hover:bg-surface-200 dark:hover:bg-surface-600 transition-colors"
            >
              {t('common.cancel', 'Cancel')}
            </button>
          </div>
        </div>
      )}

      {/* Filters */}
      <div className="glass-card p-4 rounded-2xl space-y-3" data-testid="audit-filters">
        <div className="flex items-center gap-2 text-sm font-medium text-surface-600 dark:text-surface-300">
          <FunnelSimple weight="bold" className="w-4 h-4" />
          {t('auditLog.filters', 'Filters')}
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
          <select
            value={actionFilter}
            onChange={e => { setActionFilter(e.target.value); setPage(1); }}
            className="input-field text-sm"
            aria-label={t('auditLog.filterAction', 'Filter by action')}
            data-testid="filter-action"
          >
            <option value="">{t('auditLog.allActions', 'All Actions')}</option>
            {ACTION_TYPES.map(a => (
              <option key={a} value={a}>{formatEventType(a)}</option>
            ))}
          </select>

          <div className="relative">
            <MagnifyingGlass className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
            <input
              type="text"
              value={userFilter}
              onChange={e => setUserFilter(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && handleFilterApply()}
              placeholder={t('auditLog.searchUser', 'Search user...')}
              className="input-field pl-9 text-sm"
              data-testid="filter-user"
            />
          </div>

          <input
            type="date"
            value={dateFrom}
            onChange={e => { setDateFrom(e.target.value); setPage(1); }}
            className="input-field text-sm"
            aria-label={t('auditLog.dateFrom', 'From date')}
            data-testid="filter-from"
          />

          <input
            type="date"
            value={dateTo}
            onChange={e => { setDateTo(e.target.value); setPage(1); }}
            className="input-field text-sm"
            aria-label={t('auditLog.dateTo', 'To date')}
            data-testid="filter-to"
          />
        </div>
      </div>

      {/* Table */}
      <div className="glass-card rounded-2xl overflow-hidden" data-testid="audit-table">
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-surface-200 dark:border-surface-700 text-left">
                <th className="px-4 py-3 font-medium text-surface-500">{t('auditLog.colTime', 'Time')}</th>
                <th className="px-4 py-3 font-medium text-surface-500">{t('auditLog.colAction', 'Action')}</th>
                <th className="px-4 py-3 font-medium text-surface-500">{t('auditLog.colUser', 'User')}</th>
                <th className="px-4 py-3 font-medium text-surface-500">{t('auditLog.colTarget', 'Target')}</th>
                <th className="px-4 py-3 font-medium text-surface-500">{t('auditLog.colIp', 'IP')}</th>
                <th className="px-4 py-3 font-medium text-surface-500">{t('auditLog.colDetails', 'Details')}</th>
              </tr>
            </thead>
            <tbody>
              {entries.length === 0 ? (
                <tr>
                  <td colSpan={6} className="px-4 py-8 text-center text-surface-400">
                    {t('auditLog.empty', 'No audit entries found')}
                  </td>
                </tr>
              ) : entries.map(entry => {
                const cat = getActionCategory(entry.event_type);
                return (
                  <tr key={entry.id} className="border-b border-surface-100 dark:border-surface-800 hover:bg-surface-50 dark:hover:bg-surface-800/50 transition-colors" data-testid="audit-row">
                    <td className="px-4 py-3 text-surface-600 dark:text-surface-300 whitespace-nowrap">
                      {new Date(entry.timestamp).toLocaleString()}
                    </td>
                    <td className="px-4 py-3">
                      <span className={`inline-flex px-2 py-0.5 rounded-full text-xs font-medium ${categoryColors[cat]}`}>
                        {formatEventType(entry.event_type)}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-surface-700 dark:text-surface-200">
                      {entry.username || '-'}
                    </td>
                    <td className="px-4 py-3 text-surface-500">
                      {entry.target_type ? `${entry.target_type}${entry.target_id ? `:${entry.target_id}` : ''}` : '-'}
                    </td>
                    <td className="px-4 py-3 text-surface-500 font-mono text-xs">
                      {entry.ip_address || '-'}
                    </td>
                    <td className="px-4 py-3 text-surface-500 max-w-xs truncate" title={entry.details || ''}>
                      {entry.details || '-'}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-between" data-testid="audit-pagination">
          <span className="text-sm text-surface-500">
            {t('auditLog.pageInfo', 'Page {{page}} of {{total}}', { page, total: totalPages })}
          </span>
          <div className="flex gap-2">
            <button
              onClick={() => setPage(p => Math.max(1, p - 1))}
              disabled={page <= 1}
              className="px-3 py-1.5 rounded-lg text-sm font-medium bg-surface-100 dark:bg-surface-800 disabled:opacity-40 hover:bg-surface-200 dark:hover:bg-surface-700 transition-colors"
            >
              {t('common.back', 'Previous')}
            </button>
            <button
              onClick={() => setPage(p => Math.min(totalPages, p + 1))}
              disabled={page >= totalPages}
              className="px-3 py-1.5 rounded-lg text-sm font-medium bg-surface-100 dark:bg-surface-800 disabled:opacity-40 hover:bg-surface-200 dark:hover:bg-surface-700 transition-colors"
            >
              {t('common.next', 'Next')}
            </button>
          </div>
        </div>
      )}
    </motion.div>
  );
}
