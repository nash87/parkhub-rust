/**
 * Settings hub — unified entry point for user + workspace preferences.
 *
 * Ported (minimally) from the claude.ai/design v4 handoff bundle
 * (settings.jsx). The design's 858-line page models a full settings
 * app with 12 sections; this first cut ships the **Appearance** panel
 * (the truly new functionality — nav layout + theme swatches + density)
 * and links the rest (Profile, Notifications, Vehicles, Admin) to the
 * existing dedicated pages so no feature is duplicated or lost.
 *
 * Full multi-section unification + workspace-default locks are tracked
 * as T-1842 along with the nav-variants integration.
 */

import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { User, Building, Bell, Car, Keyboard, Shield, Coins, ChartLine, FileText, ArrowRight } from '@phosphor-icons/react';
import {
  SCard,
  SRow,
  SSeg,
  SToggle,
  ThemeSwatches,
  NavLayoutGrid,
  type ThemeSwatch,
} from '../components/ui/SettingsPrimitives';
import { useTheme, type DesignThemeId } from '../context/ThemeContext';
import { useNavLayout } from '../hooks/useNavLayout';
import { useDensity } from '../hooks/useDensity';

type Scope = 'user' | 'workspace';

// Both nav-layout and density persistence live in dedicated hooks
// (useNavLayout, useDensity) so every consumer shares a single source of
// truth with cross-tab + same-tab change events.

/**
 * Theme swatches exposed in the picker. Each `value` matches one of the
 * `[data-design-theme=…]` rules baked into `src/styles/themes.css`, so the
 * selection immediately re-tints the whole app via the existing theme
 * bridge — no additional CSS has to ship with this component.
 */
// Swatches are derived at render time from the single source of truth
// (DESIGN_THEMES via useTheme().designThemes) so every swatch value is
// guaranteed to be a valid DesignThemeId. Previously-hardcoded IDs
// (`residential`/`shared`/`rental`/`personal`) were use-case palette keys,
// not design-theme keys, so clicking those was a silent no-op.

export function SettingsPage() {
  const { t } = useTranslation();
  const { designTheme, setDesignTheme, setTheme, resolved, designThemes } = useTheme();

  const [scope, setScope] = useState<Scope>('user');
  const [navLayout, setNavLayoutState] = useNavLayout();
  const [density, setDensityState] = useDensity();

  const themeSwatches = useMemo<ThemeSwatch[]>(
    () =>
      designThemes.map((t) => ({
        value: t.id,
        label: t.name,
        color: `linear-gradient(135deg, ${t.previewColors.light[2]}, ${t.previewColors.light[3]})`,
      })),
    [designThemes],
  );

  const userSections = [
    { id: 'profile', label: t('settings.profile', 'Profile'), icon: User, to: '/profile' as const },
    { id: 'notifications', label: t('settings.notifications', 'Notifications'), icon: Bell, to: '/notifications' as const },
    { id: 'vehicles', label: t('settings.vehicles', 'Vehicles'), icon: Car, to: '/vehicles' as const },
    { id: 'shortcuts', label: t('settings.shortcuts', 'Keyboard shortcuts'), icon: Keyboard, to: null, hint: t('settings.pressShortcut', 'Press ⌘/') },
  ];

  const workspaceSections = [
    { id: 'org', label: t('settings.org', 'Organization'), icon: Building, to: '/admin/settings' as const },
    { id: 'policies', label: t('settings.bookingRules', 'Booking rules'), icon: FileText, to: '/admin/settings' as const },
    { id: 'sso', label: t('settings.sso', 'SSO & roles'), icon: Shield, to: '/admin/sso' as const },
    { id: 'billing', label: t('settings.billing', 'Billing'), icon: Coins, to: '/admin/billing' as const },
    { id: 'audit', label: t('settings.auditLog', 'Audit log'), icon: ChartLine, to: '/admin/audit-log' as const },
  ];

  return (
    <div className="max-w-4xl mx-auto p-4 sm:p-6 pb-20">
      <header className="mb-5">
        <h1
          className="text-2xl sm:text-3xl font-bold text-surface-900 dark:text-white"
          style={{ letterSpacing: '-0.02em' }}
        >
          {t('settings.title', 'Settings')}
        </h1>
        <p className="text-sm text-surface-500 dark:text-surface-400 mt-1">
          {t('settings.subtitle', 'Control how ParkHub looks, feels, and behaves — for you and your workspace.')}
        </p>
      </header>

      {/* Scope switcher */}
      <div
        role="tablist"
        aria-label={t('settings.scopeLabel', 'Settings scope')}
        className="inline-flex gap-0.5 p-0.5 rounded-lg bg-surface-100 dark:bg-surface-800 border border-surface-200 dark:border-surface-700 mb-4"
      >
        {(
          [
            { k: 'user' as const, label: t('settings.personal', 'Personal'), icon: User },
            { k: 'workspace' as const, label: t('settings.workspace', 'Workspace'), icon: Building },
          ]
        ).map((s) => {
          const active = scope === s.k;
          return (
            <button
              key={s.k}
              type="button"
              role="tab"
              aria-selected={active}
              onClick={() => setScope(s.k)}
              className={`inline-flex items-center gap-1.5 px-3 py-1.5 text-sm font-semibold rounded-md transition-colors ${
                active
                  ? 'bg-white dark:bg-surface-700 text-surface-900 dark:text-white shadow-sm'
                  : 'text-surface-500 dark:text-surface-400 hover:text-surface-900 dark:hover:text-white'
              }`}
            >
              <s.icon weight="bold" className="w-3.5 h-3.5" />
              {s.label}
              {s.k === 'workspace' && (
                <span className="text-[9px] font-bold uppercase px-1 rounded bg-primary-500/15 text-primary-600 dark:text-primary-400 tracking-wider">
                  Admin
                </span>
              )}
            </button>
          );
        })}
      </div>

      {/* Appearance — the one section that's genuinely new functionality.
          Both scopes show it; other sections link to existing pages. */}
      <SCard
        title={t('settings.appearance', 'Appearance')}
        subtitle={t('settings.appearanceSubtitle', 'Theme, density, and navigation layout')}
      >
        <SRow
          title={t('settings.theme', 'Theme')}
          description={t('settings.themeDesc', 'Pick a palette — the whole app retints via the shared design-token bridge.')}
        >
          <ThemeSwatches
            value={designTheme}
            onChange={(v) => setDesignTheme(v as DesignThemeId)}
            options={themeSwatches}
          />
        </SRow>

        <SRow
          title={t('settings.dark', 'Dark mode')}
          description={t('settings.darkDesc', 'Also togglable via ⌘⇧D.')}
        >
          {/* Drive dark mode through ThemeContext so the preference persists
              in localStorage (parkhub_theme) and stays in sync with every
              other theme-aware control — ThemeSwitcher, PWA meta-theme-color,
              etc. `resolved` returns the effective light/dark even when
              theme === 'system'. */}
          <SToggle
            value={resolved === 'dark'}
            onChange={(v) => setTheme(v ? 'dark' : 'light')}
            label={t('settings.dark', 'Dark mode')}
          />
        </SRow>

        <SRow
          title={t('settings.density', 'Density')}
          description={t('settings.densityDesc', 'Default spacing between rows and cards.')}
        >
          <SSeg
            value={density}
            onChange={setDensityState}
            options={[
              { value: 'compact', label: t('settings.compact', 'Compact') },
              { value: 'cozy', label: t('settings.cozy', 'Cozy') },
              { value: 'comfortable', label: t('settings.comfortable', 'Comfortable') },
            ]}
          />
        </SRow>

        <SRow
          title={t('settings.navLayout', 'Navigation layout')}
          description={t('settings.navLayoutDesc', 'Switch instantly — your choice persists across reloads and syncs to open tabs.')}
        >
          <NavLayoutGrid value={navLayout} onChange={setNavLayoutState} />
        </SRow>
      </SCard>

      {/* Section-index: existing functionality that lives elsewhere */}
      <SCard
        title={scope === 'user' ? t('settings.moreYou', 'More personal settings') : t('settings.moreWorkspace', 'More workspace settings')}
        subtitle={t('settings.linksSubtitle', 'These live on their existing pages for now. Unifying them into this hub is tracked as T-1842.')}
      >
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 -my-2">
          {(scope === 'user' ? userSections : workspaceSections).map((s) => {
            const content = (
              <>
                <s.icon weight="duotone" className="w-5 h-5 text-primary-500 shrink-0" />
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium text-surface-900 dark:text-white truncate">
                    {s.label}
                  </p>
                  {'hint' in s && s.hint && (
                    <p className="text-[11px] text-surface-500 dark:text-surface-400 truncate">
                      {s.hint}
                    </p>
                  )}
                </div>
                {s.to && <ArrowRight weight="bold" className="w-3.5 h-3.5 text-surface-400 shrink-0" />}
              </>
            );
            return s.to ? (
              <Link
                key={s.id}
                to={s.to}
                className="flex items-center gap-3 p-3 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800/60 transition-colors"
              >
                {content}
              </Link>
            ) : (
              <div
                key={s.id}
                className="flex items-center gap-3 p-3 rounded-lg opacity-75"
              >
                {content}
              </div>
            );
          })}
        </div>
      </SCard>
    </div>
  );
}
