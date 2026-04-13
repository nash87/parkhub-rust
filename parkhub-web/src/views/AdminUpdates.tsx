import { useEffect, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  ArrowsClockwise, CheckCircle, Warning, CloudArrowDown, Info,
  Clock, Spinner, ArrowRight,
} from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';
import { getInMemoryToken } from '../api/client';

interface VersionInfo {
  version: string;
  build_hash?: string;
  build_date?: string;
  uptime_seconds?: number;
}

interface UpdateCheckResult {
  available: boolean;
  latest_version: string;
  current_version: string;
  release_url: string;
  release_notes: string;
  published_at: string;
}

interface UpdateHistoryEntry {
  id: string;
  from_version: string;
  to_version: string;
  status: 'success' | 'failed' | 'in_progress';
  applied_at: string;
  error?: string;
}

type UpdateChannel = 'stable' | 'beta';
type UpdateStep = 'idle' | 'downloading' | 'installing' | 'restarting' | 'done' | 'error';

function authHeaders(): Record<string, string> {
  const token = getInMemoryToken();
  return {
    'Content-Type': 'application/json',
    Accept: 'application/json',
    'X-Requested-With': 'XMLHttpRequest',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
  };
}

function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function StatusBadge({ status }: { status: string }) {
  const styles: Record<string, string> = {
    success: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400',
    failed: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
    in_progress: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  };
  return (
    <span className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${styles[status] || styles.failed}`} data-testid="status-badge">
      {status === 'success' ? 'Success' : status === 'failed' ? 'Failed' : 'In Progress'}
    </span>
  );
}

export function AdminUpdatesPage() {
  const { t } = useTranslation();
  const [versionInfo, setVersionInfo] = useState<VersionInfo | null>(null);
  const [checkResult, setCheckResult] = useState<UpdateCheckResult | null>(null);
  const [checking, setChecking] = useState(false);
  const [autoUpdate, setAutoUpdate] = useState(false);
  const [channel, setChannel] = useState<UpdateChannel>('stable');
  const [history, setHistory] = useState<UpdateHistoryEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [updateStep, setUpdateStep] = useState<UpdateStep>('idle');

  const fetchVersion = useCallback(async () => {
    try {
      const res = await fetch('/api/v1/system/version', {
        headers: authHeaders(),
        credentials: 'include',
      }).then(r => r.json());
      if (res.success || res.data) setVersionInfo(res.data);
    } catch { /* ignore */ }
  }, []);

  const fetchHistory = useCallback(async () => {
    try {
      const res = await fetch('/api/v1/admin/updates/history', {
        headers: authHeaders(),
        credentials: 'include',
      }).then(r => r.json());
      if (res.success || res.data) setHistory(res.data || []);
    } catch { /* ignore */ }
  }, []);

  const fetchSettings = useCallback(async () => {
    try {
      const res = await fetch('/api/v1/admin/settings', {
        headers: authHeaders(),
        credentials: 'include',
      }).then(r => r.json());
      if (res.data) {
        setAutoUpdate(res.data.auto_update ?? false);
        setChannel(res.data.update_channel ?? 'stable');
      }
    } catch { /* ignore */ }
  }, []);

  useEffect(() => {
    Promise.all([fetchVersion(), fetchHistory(), fetchSettings()])
      .finally(() => setLoading(false));
  }, [fetchVersion, fetchHistory, fetchSettings]);

  async function handleCheckUpdate() {
    setChecking(true);
    setCheckResult(null);
    try {
      const res = await fetch(`/api/v1/admin/updates/check?channel=${channel}`, {
        headers: authHeaders(),
        credentials: 'include',
      }).then(r => r.json());
      if (res.success || res.data) {
        setCheckResult(res.data);
      } else {
        toast.error(res.error?.message || t('common.error', 'Error'));
      }
    } catch {
      toast.error(t('common.error', 'Error'));
    }
    setChecking(false);
  }

  async function handleApplyUpdate() {
    setUpdateStep('downloading');
    try {
      const res = await fetch('/api/v1/admin/updates/apply', {
        method: 'POST',
        headers: authHeaders(),
        credentials: 'include',
        body: JSON.stringify({ version: checkResult?.latest_version }),
      });

      // Simulate step progression for UX
      setUpdateStep('installing');
      const json = await res.json();

      if (json.success || json.data) {
        setUpdateStep('restarting');
        setTimeout(() => {
          setUpdateStep('done');
          toast.success(t('updates.applySuccess', 'Update applied successfully'));
          fetchVersion();
          fetchHistory();
        }, 2000);
      } else {
        setUpdateStep('error');
        toast.error(json.error?.message || t('updates.applyFailed', 'Update failed'));
      }
    } catch {
      setUpdateStep('error');
      toast.error(t('updates.applyFailed', 'Update failed'));
    }
  }

  async function handleToggleAutoUpdate() {
    const newValue = !autoUpdate;
    try {
      const res = await fetch('/api/v1/admin/settings', {
        method: 'POST',
        headers: authHeaders(),
        credentials: 'include',
        body: JSON.stringify({ auto_update: newValue }),
      }).then(r => r.json());
      if (res.success) {
        setAutoUpdate(newValue);
        toast.success(newValue
          ? t('updates.autoEnabled', 'Auto-updates enabled')
          : t('updates.autoDisabled', 'Auto-updates disabled'));
      } else {
        toast.error(res.error?.message || t('common.error', 'Error'));
      }
    } catch {
      toast.error(t('common.error', 'Error'));
    }
  }

  async function handleChannelChange(newChannel: UpdateChannel) {
    setChannel(newChannel);
    try {
      await fetch('/api/v1/admin/settings', {
        method: 'POST',
        headers: authHeaders(),
        credentials: 'include',
        body: JSON.stringify({ update_channel: newChannel }),
      });
    } catch { /* ignore */ }
  }

  const isUpdating = updateStep !== 'idle' && updateStep !== 'done' && updateStep !== 'error';

  if (loading) {
    return (
      <div className="space-y-4">
        {Array.from({ length: 3 }, (_, i) => <div key={i} className="h-24 skeleton rounded-xl" />)}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center gap-3">
        <div className="p-2 bg-primary-100 dark:bg-primary-900/30 rounded-lg">
          <ArrowsClockwise weight="bold" className="w-5 h-5 text-primary-600 dark:text-primary-400" />
        </div>
        <div>
          <h2 className="text-lg font-bold text-surface-900 dark:text-white">
            {t('updates.title', 'System Updates')}
          </h2>
          <p className="text-sm text-surface-500 dark:text-surface-400">
            {t('updates.subtitle', 'Manage application updates and versioning')}
          </p>
        </div>
      </div>

      {/* Current Version Card */}
      <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 shadow-sm p-5" data-testid="version-card">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-semibold text-surface-700 dark:text-surface-300">
            {t('updates.currentVersion', 'Current Version')}
          </h3>
          <Info weight="fill" className="w-4 h-4 text-surface-400" />
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <div>
            <p className="text-xs text-surface-400 mb-1">{t('updates.version', 'Version')}</p>
            <p className="text-xl font-bold text-surface-900 dark:text-white" data-testid="current-version">
              {versionInfo?.version || '—'}
            </p>
          </div>
          <div>
            <p className="text-xs text-surface-400 mb-1">{t('updates.buildInfo', 'Build')}</p>
            <p className="text-sm font-mono text-surface-600 dark:text-surface-300">
              {versionInfo?.build_hash?.slice(0, 8) || '—'}
              {versionInfo?.build_date ? ` (${new Date(versionInfo.build_date).toLocaleDateString()})` : ''}
            </p>
          </div>
          <div>
            <p className="text-xs text-surface-400 mb-1">{t('updates.uptime', 'Uptime')}</p>
            <p className="text-sm font-medium text-surface-600 dark:text-surface-300">
              {versionInfo?.uptime_seconds != null ? formatUptime(versionInfo.uptime_seconds) : '—'}
            </p>
          </div>
        </div>
      </div>

      {/* Check for Updates */}
      <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 shadow-sm p-5 space-y-4" data-testid="update-check-section">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-semibold text-surface-700 dark:text-surface-300">
            {t('updates.checkTitle', 'Check for Updates')}
          </h3>
          <button
            onClick={handleCheckUpdate}
            disabled={checking || isUpdating}
            className="btn btn-primary btn-sm"
            data-testid="check-btn"
          >
            {checking ? (
              <Spinner weight="bold" className="w-4 h-4 animate-spin" />
            ) : (
              <ArrowsClockwise weight="bold" className="w-4 h-4" />
            )}
            {t('updates.checkButton', 'Check for Updates')}
          </button>
        </div>

        <AnimatePresence mode="wait">
          {checkResult && !checkResult.available && (
            <motion.div
              key="up-to-date"
              initial={{ opacity: 0, y: -8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0 }}
              className="flex items-center gap-3 p-4 bg-emerald-50 dark:bg-emerald-900/20 border border-emerald-200 dark:border-emerald-800 rounded-xl"
              data-testid="up-to-date"
            >
              <CheckCircle weight="fill" className="w-6 h-6 text-emerald-600 dark:text-emerald-400" />
              <div>
                <p className="text-sm font-medium text-emerald-800 dark:text-emerald-300">
                  {t('updates.upToDate', "You're up to date!")}
                </p>
                <p className="text-xs text-emerald-600 dark:text-emerald-400">
                  {t('updates.running', 'Running version {{version}}', { version: checkResult.current_version })}
                </p>
              </div>
            </motion.div>
          )}

          {checkResult?.available && (
            <motion.div
              key="update-available"
              initial={{ opacity: 0, y: -8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0 }}
              className="p-4 bg-emerald-50 dark:bg-emerald-900/20 border border-emerald-200 dark:border-emerald-800 rounded-xl space-y-3"
              data-testid="update-available"
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <CloudArrowDown weight="fill" className="w-5 h-5 text-emerald-600 dark:text-emerald-400" />
                  <span className="text-sm font-semibold text-emerald-800 dark:text-emerald-300">
                    {t('updates.newVersion', 'New version available')}
                  </span>
                </div>
                <div className="flex items-center gap-1.5 text-sm text-surface-600 dark:text-surface-300">
                  <span className="font-mono">{checkResult.current_version}</span>
                  <ArrowRight weight="bold" className="w-3.5 h-3.5" />
                  <span className="font-mono font-bold text-emerald-700 dark:text-emerald-400">{checkResult.latest_version}</span>
                </div>
              </div>

              {checkResult.release_notes && (
                <div className="text-sm text-surface-600 dark:text-surface-400 bg-white/60 dark:bg-surface-800/60 rounded-lg p-3">
                  <p className="font-medium text-surface-700 dark:text-surface-300 mb-1">
                    {t('updates.releaseNotes', 'Release Notes')}
                  </p>
                  <p className="whitespace-pre-wrap">{checkResult.release_notes}</p>
                </div>
              )}

              <div className="flex items-center gap-2">
                <button
                  onClick={handleApplyUpdate}
                  disabled={isUpdating}
                  className="btn btn-primary btn-sm"
                  data-testid="apply-btn"
                >
                  <CloudArrowDown weight="bold" className="w-4 h-4" />
                  {t('updates.applyButton', 'Update Now')}
                </button>
                {checkResult.release_url && (
                  <a
                    href={checkResult.release_url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-xs text-primary-600 dark:text-primary-400 hover:underline"
                  >
                    {t('updates.viewRelease', 'View release')}
                  </a>
                )}
                {checkResult.published_at && (
                  <span className="text-xs text-surface-400 ml-auto">
                    {new Date(checkResult.published_at).toLocaleDateString()}
                  </span>
                )}
              </div>
            </motion.div>
          )}
        </AnimatePresence>

        {/* Update progress */}
        {isUpdating && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="p-4 bg-primary-50 dark:bg-primary-900/20 border border-primary-200 dark:border-primary-800 rounded-xl"
            data-testid="update-progress"
          >
            <div className="flex items-center gap-3">
              <Spinner weight="bold" className="w-5 h-5 text-primary-600 animate-spin" />
              <div>
                <p className="text-sm font-medium text-primary-800 dark:text-primary-300">
                  {updateStep === 'downloading' && t('updates.stepDownloading', 'Downloading update...')}
                  {updateStep === 'installing' && t('updates.stepInstalling', 'Installing update...')}
                  {updateStep === 'restarting' && t('updates.stepRestarting', 'Restarting service...')}
                </p>
              </div>
            </div>
            <div className="mt-3 flex gap-1">
              {['downloading', 'installing', 'restarting'].map((step, i) => {
                const steps = ['downloading', 'installing', 'restarting'];
                const currentIdx = steps.indexOf(updateStep);
                return (
                  <div
                    key={step}
                    className={`h-1.5 flex-1 rounded-full transition-colors ${
                      i <= currentIdx ? 'bg-primary-500' : 'bg-surface-200 dark:bg-surface-700'
                    }`}
                  />
                );
              })}
            </div>
          </motion.div>
        )}

        {updateStep === 'done' && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="flex items-center gap-3 p-4 bg-emerald-50 dark:bg-emerald-900/20 border border-emerald-200 dark:border-emerald-800 rounded-xl"
            data-testid="update-success"
          >
            <CheckCircle weight="fill" className="w-6 h-6 text-emerald-600" />
            <p className="text-sm font-medium text-emerald-800 dark:text-emerald-300">
              {t('updates.applySuccess', 'Update applied successfully!')}
            </p>
          </motion.div>
        )}

        {updateStep === 'error' && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="flex items-center gap-3 p-4 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-xl"
            data-testid="update-error"
          >
            <Warning weight="fill" className="w-6 h-6 text-red-600" />
            <p className="text-sm font-medium text-red-800 dark:text-red-300">
              {t('updates.applyFailed', 'Update failed. Please try again or apply manually.')}
            </p>
          </motion.div>
        )}
      </div>

      {/* Auto-Update Toggle & Channel Selector */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        {/* Auto-Update */}
        <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 shadow-sm p-5" data-testid="auto-update-card">
          <div className="flex items-center justify-between">
            <div className="flex-1 mr-4">
              <h3 className="text-sm font-semibold text-surface-700 dark:text-surface-300">
                {t('updates.autoUpdateLabel', 'Enable automatic updates')}
              </h3>
              <p className="text-xs text-surface-400 mt-1">
                {t('updates.autoUpdateDesc', 'Automatically install minor and patch updates. Major versions require manual approval.')}
              </p>
            </div>
            <button
              onClick={handleToggleAutoUpdate}
              className="relative flex-shrink-0"
              role="switch"
              aria-checked={autoUpdate}
              data-testid="auto-update-toggle"
            >
              <div className={`w-11 h-6 rounded-full transition-colors ${autoUpdate ? 'bg-primary-500' : 'bg-surface-300 dark:bg-surface-600'}`}>
                <div className={`absolute top-0.5 w-5 h-5 rounded-full bg-white shadow-sm transition-transform ${autoUpdate ? 'translate-x-[22px]' : 'translate-x-0.5'}`} />
              </div>
            </button>
          </div>
        </div>

        {/* Channel Selector */}
        <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 shadow-sm p-5" data-testid="channel-card">
          <h3 className="text-sm font-semibold text-surface-700 dark:text-surface-300 mb-3">
            {t('updates.channelTitle', 'Update Channel')}
          </h3>
          <div className="flex gap-2">
            {(['stable', 'beta'] as const).map(ch => (
              <button
                key={ch}
                onClick={() => handleChannelChange(ch)}
                className={`flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                  channel === ch
                    ? 'bg-primary-50 dark:bg-primary-950/30 text-primary-700 dark:text-primary-300 border border-primary-200 dark:border-primary-800'
                    : 'bg-surface-50 dark:bg-surface-800 text-surface-600 dark:text-surface-400 border border-surface-200 dark:border-surface-700 hover:bg-surface-100 dark:hover:bg-surface-700'
                }`}
                data-testid={`channel-${ch}`}
              >
                {ch === 'stable'
                  ? t('updates.channelStable', 'Stable')
                  : t('updates.channelBeta', 'Beta')}
              </button>
            ))}
          </div>
          <p className="text-xs text-surface-400 mt-2">
            {channel === 'stable'
              ? t('updates.channelStableDesc', 'Recommended. Receives tested, production-ready updates.')
              : t('updates.channelBetaDesc', 'Early access to new features. May contain bugs.')}
          </p>
        </div>
      </div>

      {/* Update History */}
      <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 shadow-sm" data-testid="update-history">
        <div className="px-5 py-4 border-b border-surface-100 dark:border-surface-800">
          <h3 className="text-sm font-semibold text-surface-700 dark:text-surface-300">
            {t('updates.historyTitle', 'Update History')}
          </h3>
        </div>
        {history.length === 0 ? (
          <div className="p-8 text-center">
            <Clock weight="thin" className="w-12 h-12 mx-auto text-surface-300 dark:text-surface-600 mb-3" />
            <p className="text-sm text-surface-500 dark:text-surface-400">
              {t('updates.noHistory', 'No update history yet')}
            </p>
          </div>
        ) : (
          <div className="divide-y divide-surface-100 dark:divide-surface-800">
            {history.map(entry => (
              <div key={entry.id} className="px-5 py-3 flex items-center justify-between" data-testid="history-row">
                <div className="flex items-center gap-3 min-w-0">
                  <div className="min-w-0">
                    <p className="text-sm font-medium text-surface-900 dark:text-white">
                      {entry.from_version} <ArrowRight weight="bold" className="inline w-3 h-3 mx-1" /> {entry.to_version}
                    </p>
                    <p className="text-xs text-surface-400">
                      {new Date(entry.applied_at).toLocaleString()}
                    </p>
                  </div>
                </div>
                <StatusBadge status={entry.status} />
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
