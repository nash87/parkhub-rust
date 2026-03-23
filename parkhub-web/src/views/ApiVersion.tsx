import { useEffect, useState, useCallback } from 'react';
import { Code, Info } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';

interface DeprecationNotice {
  endpoint: string;
  method: string;
  severity: 'info' | 'warning' | 'critical';
  message: string;
  sunset_date: string | null;
  replacement: string | null;
}

interface ApiVersionInfo {
  version: string;
  api_prefix: string;
  status: string;
  deprecations: DeprecationNotice[];
  supported_versions: string[];
}

export function ApiVersionBadge() {
  const { t } = useTranslation();
  const [version, setVersion] = useState<string | null>(null);

  const loadVersion = useCallback(async () => {
    try {
      const res = await fetch('/api/v1/version').then(r => r.json());
      if (res.success) {
        setVersion(res.data.version);
      }
    } catch {
      // Silent fail for non-critical display component
    }
  }, []);

  useEffect(() => { loadVersion(); }, [loadVersion]);

  if (!version) return null;

  return (
    <span
      className="inline-flex items-center gap-1 text-xs text-surface-400 dark:text-surface-500"
      data-testid="api-version-badge"
      title={t('apiVersion.tooltip')}
    >
      <Code size={12} />
      API v{version}
    </span>
  );
}

export function ApiVersionAdmin() {
  const { t } = useTranslation();
  const [info, setInfo] = useState<ApiVersionInfo | null>(null);
  const [loading, setLoading] = useState(true);

  const loadInfo = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch('/api/v1/version').then(r => r.json());
      if (res.success) {
        setInfo(res.data);
      }
    } catch {
      // silent
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadInfo(); }, [loadInfo]);

  if (loading || !info) return null;

  return (
    <div className="bg-white dark:bg-surface-800 rounded-xl p-4 shadow-sm border border-surface-200 dark:border-surface-700" data-testid="api-version-admin">
      <div className="flex items-center gap-2 mb-3">
        <Info size={18} className="text-primary-500" />
        <h3 className="font-medium text-surface-900 dark:text-white">
          {t('apiVersion.title')}
        </h3>
      </div>
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-sm">
        <div>
          <p className="text-surface-500">{t('apiVersion.version')}</p>
          <p className="font-mono font-medium text-surface-900 dark:text-white" data-testid="version-value">{info.version}</p>
        </div>
        <div>
          <p className="text-surface-500">{t('apiVersion.prefix')}</p>
          <p className="font-mono font-medium text-surface-900 dark:text-white">{info.api_prefix}</p>
        </div>
        <div>
          <p className="text-surface-500">{t('apiVersion.status')}</p>
          <p className="font-medium text-green-600">{info.status}</p>
        </div>
        <div>
          <p className="text-surface-500">{t('apiVersion.deprecations')}</p>
          <p className="font-medium text-surface-900 dark:text-white">{info.deprecations.length}</p>
        </div>
      </div>
      {info.deprecations.length > 0 && (
        <div className="mt-3 space-y-2" data-testid="deprecation-list">
          {info.deprecations.map((d, i) => (
            <div key={i} className="text-xs bg-amber-50 dark:bg-amber-900/20 rounded-lg p-2 border border-amber-200 dark:border-amber-800">
              <span className="font-mono font-medium">{d.method} {d.endpoint}</span>
              <span className="mx-1">—</span>
              <span>{d.message}</span>
              {d.sunset_date && <span className="ml-1 text-amber-600">(sunset: {d.sunset_date})</span>}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
