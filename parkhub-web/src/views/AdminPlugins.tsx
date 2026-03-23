import { useEffect, useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Puzzle, ToggleLeft, ToggleRight, Gear, Question, Lightning } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

interface PluginRoute {
  path: string;
  method: string;
  description: string;
}

interface PluginInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  status: 'enabled' | 'disabled';
  subscribed_events: string[];
  routes: PluginRoute[];
  config: Record<string, any>;
}

interface PluginListResponse {
  plugins: PluginInfo[];
  total: number;
  enabled: number;
}

export function AdminPluginsPage() {
  const { t } = useTranslation();
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [total, setTotal] = useState(0);
  const [enabled, setEnabled] = useState(0);
  const [loading, setLoading] = useState(true);
  const [showHelp, setShowHelp] = useState(false);
  const [configDialog, setConfigDialog] = useState<string | null>(null);
  const [configValues, setConfigValues] = useState<Record<string, any>>({});
  const [savingConfig, setSavingConfig] = useState(false);

  const loadPlugins = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch('/api/v1/admin/plugins').then(r => r.json());
      if (res.success) {
        const data: PluginListResponse = res.data;
        setPlugins(data.plugins);
        setTotal(data.total);
        setEnabled(data.enabled);
      }
    } catch {
      toast.error(t('common.error'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => { loadPlugins(); }, [loadPlugins]);

  const handleToggle = async (id: string) => {
    try {
      const res = await fetch(`/api/v1/admin/plugins/${id}/toggle`, { method: 'PUT' }).then(r => r.json());
      if (res.success) {
        toast.success(t('plugins.toggled'));
        loadPlugins();
      }
    } catch {
      toast.error(t('common.error'));
    }
  };

  const openConfig = async (id: string) => {
    try {
      const res = await fetch(`/api/v1/admin/plugins/${id}/config`).then(r => r.json());
      if (res.success) {
        setConfigValues(res.data);
        setConfigDialog(id);
      }
    } catch {
      toast.error(t('common.error'));
    }
  };

  const saveConfig = async () => {
    if (!configDialog) return;
    setSavingConfig(true);
    try {
      const res = await fetch(`/api/v1/admin/plugins/${configDialog}/config`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ config: configValues }),
      }).then(r => r.json());
      if (res.success) {
        toast.success(t('plugins.configSaved'));
        setConfigDialog(null);
        loadPlugins();
      }
    } catch {
      toast.error(t('common.error'));
    } finally {
      setSavingConfig(false);
    }
  };

  const eventColors: Record<string, string> = {
    booking_created: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
    booking_cancelled: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
    user_registered: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
    lot_full: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white">
            {t('plugins.title')}
          </h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">
            {t('plugins.subtitle')}
          </p>
        </div>
        <button
          onClick={() => setShowHelp(!showHelp)}
          className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700"
          aria-label={t('plugins.helpLabel')}
          data-testid="plugins-help-btn"
        >
          <Question size={20} />
        </button>
      </div>

      {/* Help tooltip */}
      {showHelp && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4"
          data-testid="plugins-help"
        >
          <p className="text-sm text-blue-700 dark:text-blue-300">
            {t('plugins.help')}
          </p>
        </motion.div>
      )}

      {/* Stats */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4" data-testid="plugins-stats">
        <div className="bg-white dark:bg-surface-800 rounded-xl p-4 shadow-sm border border-surface-200 dark:border-surface-700">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-lg bg-primary-100 dark:bg-primary-900/30">
              <Puzzle size={20} className="text-primary-600 dark:text-primary-400" />
            </div>
            <div>
              <p className="text-sm text-surface-500 dark:text-surface-400">{t('plugins.totalPlugins')}</p>
              <p className="text-xl font-bold text-surface-900 dark:text-white">{total}</p>
            </div>
          </div>
        </div>
        <div className="bg-white dark:bg-surface-800 rounded-xl p-4 shadow-sm border border-surface-200 dark:border-surface-700">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-lg bg-green-100 dark:bg-green-900/30">
              <Lightning size={20} className="text-green-600 dark:text-green-400" />
            </div>
            <div>
              <p className="text-sm text-surface-500 dark:text-surface-400">{t('plugins.enabledPlugins')}</p>
              <p className="text-xl font-bold text-surface-900 dark:text-white">{enabled}</p>
            </div>
          </div>
        </div>
        <div className="bg-white dark:bg-surface-800 rounded-xl p-4 shadow-sm border border-surface-200 dark:border-surface-700">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-lg bg-surface-100 dark:bg-surface-700">
              <Gear size={20} className="text-surface-600 dark:text-surface-400" />
            </div>
            <div>
              <p className="text-sm text-surface-500 dark:text-surface-400">{t('plugins.disabledPlugins')}</p>
              <p className="text-xl font-bold text-surface-900 dark:text-white">{total - enabled}</p>
            </div>
          </div>
        </div>
      </div>

      {/* Plugin grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4" data-testid="plugins-grid">
        {plugins.map(plugin => (
          <motion.div
            key={plugin.id}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="bg-white dark:bg-surface-800 rounded-xl p-5 shadow-sm border border-surface-200 dark:border-surface-700"
            data-testid="plugin-card"
          >
            <div className="flex items-start justify-between">
              <div className="flex-1">
                <div className="flex items-center gap-2">
                  <h3 className="text-lg font-semibold text-surface-900 dark:text-white">
                    {plugin.name}
                  </h3>
                  <span className="text-xs text-surface-400">v{plugin.version}</span>
                </div>
                <p className="text-sm text-surface-500 dark:text-surface-400 mt-1">
                  {plugin.description}
                </p>
                <p className="text-xs text-surface-400 mt-1">
                  {t('plugins.by')} {plugin.author}
                </p>
              </div>
              <button
                onClick={() => handleToggle(plugin.id)}
                className="ml-3 flex-shrink-0"
                aria-label={t(plugin.status === 'enabled' ? 'plugins.disable' : 'plugins.enable')}
                data-testid={`toggle-${plugin.id}`}
              >
                {plugin.status === 'enabled' ? (
                  <ToggleRight size={32} weight="fill" className="text-green-500" />
                ) : (
                  <ToggleLeft size={32} className="text-surface-300 dark:text-surface-600" />
                )}
              </button>
            </div>

            {/* Events */}
            <div className="mt-3 flex flex-wrap gap-1">
              {plugin.subscribed_events.map(event => (
                <span
                  key={event}
                  className={`text-xs px-2 py-0.5 rounded-full ${eventColors[event] || 'bg-surface-100 text-surface-600'}`}
                >
                  {event.replace(/_/g, ' ')}
                </span>
              ))}
            </div>

            {/* Actions */}
            <div className="mt-4 flex items-center gap-2">
              <button
                onClick={() => openConfig(plugin.id)}
                className="text-sm px-3 py-1.5 rounded-lg bg-surface-100 dark:bg-surface-700 hover:bg-surface-200 dark:hover:bg-surface-600 text-surface-700 dark:text-surface-300"
                data-testid={`config-${plugin.id}`}
              >
                <Gear size={14} className="inline mr-1" />
                {t('plugins.configure')}
              </button>
              {plugin.routes.length > 0 && (
                <span className="text-xs text-surface-400">
                  {plugin.routes.length} {t('plugins.routes')}
                </span>
              )}
            </div>
          </motion.div>
        ))}
      </div>

      {plugins.length === 0 && (
        <div className="text-center py-12 text-surface-500 dark:text-surface-400" data-testid="plugins-empty">
          {t('plugins.empty')}
        </div>
      )}

      {/* Config dialog */}
      {configDialog && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" data-testid="config-dialog">
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            className="bg-white dark:bg-surface-800 rounded-xl p-6 max-w-md w-full mx-4 shadow-xl"
          >
            <h2 className="text-lg font-semibold text-surface-900 dark:text-white mb-4">
              {t('plugins.configTitle')}
            </h2>
            <div className="space-y-3 max-h-80 overflow-y-auto">
              {Object.entries(configValues).map(([key, value]) => (
                <div key={key}>
                  <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                    {key.replace(/_/g, ' ')}
                  </label>
                  {typeof value === 'boolean' ? (
                    <button
                      onClick={() => setConfigValues({ ...configValues, [key]: !value })}
                      className="flex items-center"
                      data-testid={`config-field-${key}`}
                    >
                      {value ? (
                        <ToggleRight size={28} weight="fill" className="text-green-500" />
                      ) : (
                        <ToggleLeft size={28} className="text-surface-300" />
                      )}
                    </button>
                  ) : (
                    <input
                      type="text"
                      value={String(value)}
                      onChange={(e) => setConfigValues({ ...configValues, [key]: e.target.value })}
                      className="w-full px-3 py-2 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-surface-900 dark:text-white"
                      data-testid={`config-field-${key}`}
                    />
                  )}
                </div>
              ))}
            </div>
            <div className="flex justify-end gap-2 mt-4">
              <button
                onClick={() => setConfigDialog(null)}
                className="px-4 py-2 rounded-lg bg-surface-100 dark:bg-surface-700 text-surface-700 dark:text-surface-300"
              >
                {t('common.cancel')}
              </button>
              <button
                onClick={saveConfig}
                disabled={savingConfig}
                className="px-4 py-2 rounded-lg bg-primary-500 text-white hover:bg-primary-600 disabled:opacity-50"
                data-testid="save-config-btn"
              >
                {savingConfig ? t('common.saving') : t('common.save')}
              </button>
            </div>
          </motion.div>
        </div>
      )}
    </div>
  );
}
