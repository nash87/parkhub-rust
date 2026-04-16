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
 * Entirely local: one server call per page load. No analytics, no
 * third-party scripts, no AI inference.
 */

import { useEffect, useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';

interface ModuleInfo {
  name: string;
  category: string;
  description: string;
  enabled: boolean;
  runtime_toggleable: boolean;
  runtime_enabled?: boolean;
  config_keys: string[];
  ui_route: string | null;
  depends_on: string[];
  version: string;
}

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

function StatusDot({ enabled }: { enabled: boolean }) {
  return (
    <span
      aria-label={enabled ? 'enabled' : 'disabled'}
      className={`inline-block h-2.5 w-2.5 rounded-full ${enabled ? 'bg-emerald-500' : 'bg-neutral-500'}`}
    />
  );
}

function ModuleCard({ m, t }: { m: ModuleInfo; t: (k: string, d?: string) => string }) {
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
        <StatusDot enabled={enabled} />
        <h3 className="font-semibold capitalize">{m.name.replace(/-/g, ' ')}</h3>
        {m.runtime_toggleable && (
          <span className="ml-auto rounded-full bg-amber-500/10 px-2 py-0.5 text-[10px] font-medium uppercase text-amber-600 dark:text-amber-400">
            {t('admin.modules.runtimeToggleable', 'runtime')}
          </span>
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
        <span className="ml-auto text-surface-400">v{m.version}</span>
      </div>
    </div>
  );
}

export function AdminModulesPage() {
  const { t } = useTranslation();
  const [modules, setModules] = useState<ModuleInfo[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState<string | 'all'>('all');
  const [hideDisabled, setHideDisabled] = useState(false);

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
                <ModuleCard key={m.name} m={m} t={t} />
              ))}
            </div>
          </section>
        ))
      )}
    </div>
  );
}
