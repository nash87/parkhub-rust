/**
 * Admin Modules Dashboard
 *
 * Single page that answers: "what does this deployment of ParkHub
 * actually do, and where do I configure each piece?"
 *
 * Fetches `/api/v1/modules/info` (server-rendered ModuleInfo registry),
 * groups entries by category, supports free-text search + category
 * filter, and surfaces each module's config-keys + dependencies +
 * deep-link to its own admin page.
 *
 * T-1720 v2: admins see an inline runtime enable/disable switch for
 * every module where `runtime_toggleable` is true. The switch is
 * optimistic: the card flips immediately, then on a non-OK response
 * the state reverts and a toast surfaces the error. Non-admins keep
 * the v1 read-only view.
 */

import { useEffect, useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';
import { type ModuleInfo } from '../api/client';
import { useAuth } from '../context/AuthContext';
import { useModuleToggle } from '../hooks/useModuleToggle';
import { ConfigEditorModal } from '../components/ConfigEditorModal';

const CATEGORY_ORDER = [
  'core',
  'booking',
  'vehicle',
  'payment',
  'admin',
  'analytics',
  'integration',
  'notification',
  'compliance',
  'enterprise',
  'experimental',
] as const;

function categoryLabel(t: (k: string, d?: string) => string, cat: string): string {
  return t(`admin.modules.category.${cat}`, cat.charAt(0).toUpperCase() + cat.slice(1));
}

/** Status pill reflects runtime_enabled (green), runtime off (amber), or compile-time off (gray). */
function StatusDot({ m }: { m: ModuleInfo }) {
  const runtime = m.runtime_enabled ?? m.enabled;
  let cls: string;
  let label: string;
  if (!m.enabled) {
    // Compile-time disabled — nothing admins can do at runtime.
    cls = 'bg-neutral-500';
    label = 'compile-time off';
  } else if (runtime) {
    cls = 'bg-emerald-500';
    label = 'enabled';
  } else {
    // Compiled in but runtime-toggled off.
    cls = 'bg-amber-500';
    label = 'runtime off';
  }
  return (
    <span aria-label={label} className={`inline-block h-2.5 w-2.5 rounded-full ${cls}`} />
  );
}

interface ModuleToggleSwitchProps {
  m: ModuleInfo;
  onLocalChange: (next: boolean) => void;
  onRevert: () => void;
  t: (k: string, d?: string, o?: Record<string, unknown>) => string;
}

/**
 * Accessible switch rendered inside each admin module card. Handles the
 * optimistic flip + toast + revert itself via the useModuleToggle hook.
 */
function ModuleToggleSwitch({ m, onLocalChange, onRevert, t }: ModuleToggleSwitchProps) {
  const { inFlight, toggle } = useModuleToggle(m.name);
  const currentRuntime = m.runtime_enabled ?? m.enabled;

  const disabledForCompileTime = !m.enabled;
  const notToggleable = !m.runtime_toggleable;
  const disabled = inFlight || disabledForCompileTime || notToggleable;

  async function onClick() {
    if (disabled) return;
    const next = !currentRuntime;
    // Optimistic: flip the card immediately, server confirms or we revert.
    onLocalChange(next);
    const result = await toggle(next);
    if (result.ok) {
      toast.success(
        t('admin.modules.toggle.success', 'Module {{name}} toggled', { name: m.name }),
      );
    } else {
      onRevert();
      toast.error(
        t('admin.modules.toggle.error', 'Could not toggle {{name}}', { name: m.name }),
      );
    }
  }

  const ariaLabel = notToggleable
    ? t('admin.modules.toggle.notRuntimeToggleable', 'Not runtime toggleable')
    : currentRuntime
      ? t('admin.modules.toggle.disable', 'Disable')
      : t('admin.modules.toggle.enable', 'Enable');

  const title = notToggleable
    ? t('admin.modules.toggle.notRuntimeToggleable', 'Not runtime toggleable')
    : ariaLabel;

  return (
    <button
      type="button"
      role="switch"
      aria-checked={currentRuntime}
      aria-label={ariaLabel}
      title={title}
      disabled={disabled}
      onClick={onClick}
      data-testid={`module-toggle-${m.name}`}
      className={`relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 dark:focus:ring-offset-surface-900 ${
        currentRuntime
          ? 'bg-primary-600'
          : 'bg-surface-300 dark:bg-surface-600'
      } ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
    >
      <span
        className={`inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform shadow-sm ${
          currentRuntime ? 'translate-x-5' : 'translate-x-0.5'
        }`}
      />
    </button>
  );
}

interface ModuleCardProps {
  m: ModuleInfo;
  t: (k: string, d?: string, o?: Record<string, unknown>) => string;
  isAdmin: boolean;
  onModuleChange: (name: string, runtimeEnabled: boolean) => void;
  onOpenConfig: (name: string) => void;
}

function ModuleCard({ m, t, isAdmin, onModuleChange, onOpenConfig }: ModuleCardProps) {
  const enabled = m.runtime_enabled ?? m.enabled;
  return (
    <div
      className={`rounded-xl border p-4 transition-colors ${
        enabled
          ? 'border-surface-200/50 dark:border-surface-700/50 bg-white/80 dark:bg-surface-900/60'
          : 'border-surface-200/30 dark:border-surface-800/40 bg-surface-50 dark:bg-surface-950 opacity-70'
      }`}
      data-testid={`module-card-${m.name}`}
    >
      <div className="flex items-center gap-2 mb-1">
        <StatusDot m={m} />
        <h3 className="font-semibold capitalize">{m.name.replace(/-/g, ' ')}</h3>
        {m.runtime_toggleable && (
          <span className="rounded-full bg-amber-500/10 px-2 py-0.5 text-[10px] font-medium uppercase text-amber-600 dark:text-amber-400">
            {t('admin.modules.runtimeToggleable', 'runtime')}
          </span>
        )}
        {isAdmin && (
          <div className="ml-auto">
            <ModuleToggleSwitch
              m={m}
              onLocalChange={(next) => onModuleChange(m.name, next)}
              onRevert={() => onModuleChange(m.name, enabled)}
              t={t}
            />
          </div>
        )}
      </div>
      <p className="text-sm text-surface-600 dark:text-surface-400 mb-3">{m.description}</p>
      <div className="flex flex-wrap gap-1 mb-2">
        {m.config_keys.map((k) => (
          <code
            key={k}
            className="rounded bg-surface-100 dark:bg-surface-800 px-1.5 py-0.5 text-[11px] text-surface-700 dark:text-surface-300"
            title={t('admin.modules.configKey', 'Config key (admin settings)')}
          >
            {k}
          </code>
        ))}
      </div>
      {m.depends_on.length > 0 && (
        <div className="mb-2 text-[11px] text-surface-500">
          {t('admin.modules.dependsOn', 'depends on')}: {m.depends_on.join(', ')}
        </div>
      )}
      <div className="flex items-center gap-2 text-xs">
        {m.ui_route ? (
          <Link
            to={m.ui_route}
            className="text-primary-600 dark:text-primary-400 hover:underline"
          >
            {t('admin.modules.openUi', 'Open UI')} →
          </Link>
        ) : (
          <span className="text-surface-400">{t('admin.modules.noUi', 'No UI surface')}</span>
        )}
        {isAdmin && m.config_schema != null && (
          <button
            type="button"
            onClick={() => onOpenConfig(m.name)}
            className="rounded border border-surface-200 dark:border-surface-700 px-2 py-0.5 text-[11px] text-surface-700 dark:text-surface-300 hover:bg-surface-100 dark:hover:bg-surface-800"
            data-testid={`module-config-${m.name}`}
            aria-label={t('admin.modules.config.open', 'Configure')}
          >
            {t('admin.modules.config.open', 'Configure')}
          </button>
        )}
        <span className="ml-auto text-surface-400">v{m.version}</span>
      </div>
    </div>
  );
}

export function AdminModulesPage() {
  const { t } = useTranslation();
  const { user } = useAuth();
  const isAdmin = user?.role === 'admin' || user?.role === 'superadmin';

  const [modules, setModules] = useState<ModuleInfo[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState<string | 'all'>('all');
  const [hideDisabled, setHideDisabled] = useState(false);
  const [configOpenFor, setConfigOpenFor] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    fetch('/api/v1/modules/info', { credentials: 'include' })
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(`HTTP ${r.status}`))))
      .then((j) => {
        if (!active) return;
        const data: ModuleInfo[] = j?.data ?? j?.module_info ?? [];
        setModules(data);
      })
      .catch((e) => active && setErr(String(e)));
    return () => {
      active = false;
    };
  }, []);

  /** Optimistic local update. Caller is responsible for reverting on failure. */
  function handleModuleChange(name: string, runtimeEnabled: boolean) {
    setModules((prev) =>
      prev?.map((m) => (m.name === name ? { ...m, runtime_enabled: runtimeEnabled } : m)) ?? prev,
    );
  }

  const filtered = useMemo(() => {
    if (!modules) return [];
    const q = query.trim().toLowerCase();
    return modules.filter((m) => {
      if (filter !== 'all' && m.category !== filter) return false;
      if (hideDisabled && !(m.runtime_enabled ?? m.enabled)) return false;
      if (!q) return true;
      return (
        m.name.toLowerCase().includes(q) ||
        m.description.toLowerCase().includes(q) ||
        m.config_keys.some((k) => k.toLowerCase().includes(q))
      );
    });
  }, [modules, query, filter, hideDisabled]);

  const grouped = useMemo(() => {
    const by = new Map<string, ModuleInfo[]>();
    for (const m of filtered) {
      if (!by.has(m.category)) by.set(m.category, []);
      by.get(m.category)!.push(m);
    }
    return CATEGORY_ORDER.map((c) => ({ cat: c, mods: by.get(c) ?? [] })).filter(
      (g) => g.mods.length > 0,
    );
  }, [filtered]);

  const total = modules?.length ?? 0;
  const enabledCount = modules?.filter((m) => m.runtime_enabled ?? m.enabled).length ?? 0;

  if (err) {
    return (
      <div className="p-6 text-red-500" role="alert">
        {t('admin.modules.loadError', 'Failed to load modules')}: {err}
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6" data-testid="modules-dashboard">
      <header className="flex items-center gap-4 flex-wrap">
        <div>
          <h1 className="text-2xl font-bold">{t('admin.modules.title', 'Modules')}</h1>
          <p className="text-sm text-surface-600 dark:text-surface-400">
            {t('admin.modules.subtitle', 'Feature modules compiled into this deployment.')}
          </p>
        </div>
        <div className="ml-auto flex items-center gap-2 text-sm">
          <span className="rounded-full bg-emerald-500/10 px-3 py-1 text-emerald-600 dark:text-emerald-400">
            {enabledCount}/{total} {t('admin.modules.active', 'active')}
          </span>
        </div>
      </header>

      <div className="flex flex-wrap gap-3 items-center">
        <input
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder={t('admin.modules.searchPlaceholder', 'Search by name, description, or config key…')}
          className="flex-1 min-w-[220px] rounded-lg border border-surface-200 dark:border-surface-700 bg-transparent px-3 py-2 text-sm outline-none focus:border-primary-400"
          data-testid="modules-search"
        />
        <select
          value={filter}
          onChange={(e) => setFilter(e.target.value as typeof filter)}
          className="rounded-lg border border-surface-200 dark:border-surface-700 bg-transparent px-3 py-2 text-sm"
          aria-label={t('admin.modules.categoryFilter', 'Filter by category')}
        >
          <option value="all">{t('admin.modules.allCategories', 'All categories')}</option>
          {CATEGORY_ORDER.map((c) => (
            <option key={c} value={c}>
              {categoryLabel(t, c)}
            </option>
          ))}
        </select>
        <label className="flex items-center gap-2 text-sm cursor-pointer">
          <input
            type="checkbox"
            checked={hideDisabled}
            onChange={(e) => setHideDisabled(e.target.checked)}
            className="rounded"
          />
          {t('admin.modules.hideDisabled', 'Hide disabled')}
        </label>
      </div>

      {modules === null ? (
        <div className="text-sm text-surface-500">{t('loading', 'Loading…')}</div>
      ) : grouped.length === 0 ? (
        <div className="text-sm text-surface-500">
          {t('admin.modules.noMatches', 'No modules match your filters.')}
        </div>
      ) : (
        grouped.map(({ cat, mods }) => (
          <section key={cat} aria-labelledby={`cat-${cat}`}>
            <h2
              id={`cat-${cat}`}
              className="text-lg font-semibold mb-3 text-surface-700 dark:text-surface-300"
            >
              {categoryLabel(t, cat)}{' '}
              <span className="text-xs text-surface-500 font-normal">({mods.length})</span>
            </h2>
            <div className="grid gap-3 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3">
              {mods.map((m) => (
                <ModuleCard
                  key={m.name}
                  m={m}
                  t={t}
                  isAdmin={isAdmin}
                  onModuleChange={handleModuleChange}
                  onOpenConfig={(name) => setConfigOpenFor(name)}
                />
              ))}
            </div>
          </section>
        ))
      )}

      {configOpenFor && (
        <ConfigEditorModal
          moduleName={configOpenFor}
          isOpen
          onClose={() => setConfigOpenFor(null)}
        />
      )}
    </div>
  );
}
