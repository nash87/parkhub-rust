import { useCallback, useEffect, useState, type ComponentType } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { V5ThemeProvider, useV5Theme } from './ThemeProvider';
import { V5ToastProvider } from './Toast';
import { V5SettingsProvider, useV5Settings } from './settings';
import { syncSettingsToServer } from './settings/syncToServer';
import { applyFontVariant } from './fonts/fontVariants';
import { V5Sidebar } from './sidebar';
import { api } from '../api/client';
import { V5TopBar } from './TopBar';
import { V5CommandPalette } from './CommandPalette';
import { V5AssistantPanel } from './AssistantPanel';
import { breadcrumbFor, byId, NAV, SECTION_HEADINGS, type NavSection, type ScreenId } from './nav';
import { V5NamedIcon } from './primitives';
import { DashboardV5 } from './screens/Dashboard';
import { BuchungenV5 } from './screens/Buchungen';
import { BuchenV5 } from './screens/Buchen';
import { FahrzeugeV5 } from './screens/Fahrzeuge';
import { KalenderV5 } from './screens/Kalender';
import { KarteV5 } from './screens/Karte';
import { CreditsV5 } from './screens/Credits';
import { ProfilV5 } from './screens/Profil';
import { TeamV5 } from './screens/Team';
import { RanglisteV5 } from './screens/Rangliste';
import { EVV5 } from './screens/EV';
import { TauschV5 } from './screens/Tausch';
import { EincheckenV5 } from './screens/Einchecken';
import { VorhersagenV5 } from './screens/Vorhersagen';
import { GaestepassV5 } from './screens/Gaestepass';
import { AnalyticsV5 } from './screens/Analytics';
import { NutzerV5 } from './screens/Nutzer';
import { BillingV5 } from './screens/Billing';
import { LobbyV5 } from './screens/Lobby';
import { BenachrichtigungenV5 } from './screens/Benachrichtigungen';
import { EinstellungenV5 } from './screens/Einstellungen';
import { StandorteV5 } from './screens/Standorte';
import { IntegrationsV5 } from './screens/Integrations';
import { ApikeysV5 } from './screens/Apikeys';
import { AuditV5 } from './screens/Audit';
import { PoliciesV5 } from './screens/Policies';
import { startViewTransition } from './viewTransitions';
import { readScreenFromUrl, useSyncScreenToUrl } from './useDeepLink';
import { useKeyboardShortcuts } from './useKeyboardShortcuts';

import './fonts';
import './tokens.css';
import './density/density.css';

/**
 * Registry of all 26 v5 screens. Every `NavItem.id` in `./nav.ts` maps
 * to a concrete component here — the former placeholder fallback was
 * retired in v5.1 once the last screen port landed.
 */
const SCREENS: Record<ScreenId, ComponentType<{ navigate: (id: ScreenId) => void }>> = {
  dashboard: DashboardV5,
  buchungen: BuchungenV5,
  buchen: BuchenV5,
  fahrzeuge: FahrzeugeV5,
  kalender: KalenderV5,
  karte: KarteV5,
  credits: CreditsV5,
  profil: ProfilV5,
  team: TeamV5,
  rangliste: RanglisteV5,
  ev: EVV5,
  tausch: TauschV5,
  einchecken: EincheckenV5,
  vorhersagen: VorhersagenV5,
  gaestepass: GaestepassV5,
  analytics: AnalyticsV5,
  nutzer: NutzerV5,
  billing: BillingV5,
  lobby: LobbyV5,
  benachrichtigungen: BenachrichtigungenV5,
  einstellungen: EinstellungenV5,
  standorte: StandorteV5,
  integrations: IntegrationsV5,
  apikeys: ApikeysV5,
  audit: AuditV5,
  policies: PoliciesV5,
};

const STORAGE_KEY = 'ph-v5-screen';
const DEFAULT_SCREEN: ScreenId = 'dashboard';

/**
 * Query client configured for v5 data: short stale window for admin tables,
 * opportunistic refetch on focus. Individual queries override as needed.
 */
function makeQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 30_000,
        gcTime: 5 * 60_000,
        refetchOnWindowFocus: true,
        retry: 1,
      },
    },
  });
}

/**
 * Bridge: keep V5ThemeProvider mode in sync with the settings store,
 * lazy-load font CSS as the user picks variants, and hydrate settings
 * from the server on mount. The settings store is canonical; the legacy
 * ThemeProvider remains so existing components (Sidebar, screens) keep
 * their `useV5Theme()` hook contract.
 */
function SettingsBridge() {
  const { settings, hydrateRemote } = useV5Settings();
  const { mode, setMode } = useV5Theme();

  useEffect(() => {
    if (settings.appearance.mode !== mode) setMode(settings.appearance.mode);
  }, [settings.appearance.mode, mode, setMode]);

  useEffect(() => {
    void applyFontVariant(settings.appearance.font);
  }, [settings.appearance.font]);

  // One-shot hydration on mount — silently fail if endpoint is missing.
  useEffect(() => {
    let cancelled = false;
    void api
      .getSettings()
      .then((res) => {
        if (cancelled || !res.success || !res.data) return;
        hydrateRemote(res.data);
      })
      .catch(() => {
        /* endpoint may not be deployed yet */
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return null;
}

export function V5MobileNav({
  open,
  active,
  onClose,
  onNavigate,
}: {
  open: boolean;
  active: ScreenId;
  onClose: () => void;
  onNavigate: (id: ScreenId) => void;
}) {
  const sections: NavSection[] = ['main', 'fleet', 'admin'];

  if (!open) return null;

  return (
    <div
      className="v5-mobile-nav"
      role="dialog"
      aria-modal="true"
      aria-label="Navigation"
    >
      <button
        type="button"
        className="v5-mobile-nav__scrim"
        aria-label="Navigation schließen"
        onClick={onClose}
      />
      <nav className="v5-mobile-nav__sheet" aria-label="Mobile Hauptnavigation">
        <div className="v5-mobile-nav__header">
          <div className="v5-mobile-nav__brand">
            <span className="v5-mobile-nav__brand-mark">P</span>
            <span>ParkHub</span>
          </div>
          <button
            type="button"
            className="v5-mobile-nav__close"
            aria-label="Navigation schließen"
            onClick={onClose}
          >
            <V5NamedIcon name="x" size={16} color="var(--v5-txt)" />
          </button>
        </div>

        <div className="v5-mobile-nav__body">
          {sections.map((section) => (
            <div className="v5-mobile-nav__section" key={section}>
              <div className="v5-mobile-nav__section-label">
                {SECTION_HEADINGS[section]}
              </div>
              {NAV.filter((item) => item.section === section).map((item) => {
                const isActive = item.id === active;
                return (
                  <button
                    key={item.id}
                    type="button"
                    className="v5-mobile-nav__item"
                    aria-current={isActive ? 'page' : undefined}
                    data-active={isActive ? 'true' : 'false'}
                    onClick={() => onNavigate(item.id as ScreenId)}
                  >
                    <V5NamedIcon
                      name={item.icon}
                      size={16}
                      color={isActive ? 'var(--v5-acc)' : 'var(--v5-mut)'}
                    />
                    <span>{item.label}</span>
                  </button>
                );
              })}
            </div>
          ))}
        </div>
      </nav>
    </div>
  );
}

function V5Shell() {
  // URL is source of truth; localStorage is a back-compat cache for users
  // who load `/v5` bare. Priority: URL → localStorage → default.
  const { settings } = useV5Settings();
  const [screen, setScreen] = useState<ScreenId>(() => {
    if (typeof window === 'undefined') return DEFAULT_SCREEN;
    const stored = window.localStorage.getItem(STORAGE_KEY) as ScreenId | null;
    const storedFallback: ScreenId =
      stored && byId.has(stored) ? stored : DEFAULT_SCREEN;
    return readScreenFromUrl(storedFallback);
  });
  const [cmdOpen, setCmdOpen] = useState(false);
  const [assistantOpen, setAssistantOpen] = useState(false);
  const [mobileNavOpen, setMobileNavOpen] = useState(false);

  useEffect(() => {
    window.localStorage.setItem(STORAGE_KEY, screen);
  }, [screen]);

  // Keep browser URL + localStorage in lockstep with the in-memory screen,
  // and restore screen state on back/forward — gated by the deepLinking feature toggle
  // so disabling it stops history pushes and popstate handling entirely.
  useSyncScreenToUrl(screen, setScreen, settings.features.deepLinking);

  // Legacy shortcuts kept in a raw listener so Ctrl+K can intercept
  // browser default. The new per-screen shortcut hook covers single-key
  // bindings (n, /, Escape, ?) with input-focus safeguards.
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setCmdOpen((o) => !o);
      }
    };
    window.addEventListener('keydown', h);
    return () => window.removeEventListener('keydown', h);
  }, []);

  // Global single-key shortcuts (opt-in per screen via future hook calls).
  useKeyboardShortcuts({
    '?': () => setAssistantOpen((o) => !o),
    '/': (e) => {
      e.preventDefault();
      setCmdOpen(true);
    },
    Escape: () => {
      setCmdOpen(false);
      setAssistantOpen(false);
      setMobileNavOpen(false);
    },
  });

  const navigate = useCallback((id: ScreenId) => {
    // Wrap the state update in a View Transition so the browser
    // cross-fades the old and new screen. Gracefully no-ops on
    // Safari <18 / Firefox and under prefers-reduced-motion.
    startViewTransition(() => {
      setScreen(id);
      setCmdOpen(false);
      setMobileNavOpen(false);
    });
  }, []);

  const meta = byId.get(screen);
  const ScreenComponent = SCREENS[screen] ?? SCREENS[DEFAULT_SCREEN]!;

  return (
    <div
      className="v5-shell-root"
      style={{
        width: '100%',
        height: '100dvh',
        display: 'flex',
        background: 'var(--v5-bg)',
        color: 'var(--v5-txt)',
        overflow: 'hidden',
        fontFamily: "var(--v5-font-family, 'Inter Variable', 'Inter', system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif)",
        fontFeatureSettings: '"cv11", "ss01"',
      }}
    >
      <div className="v5-shell-sidebar">
        <V5Sidebar active={screen} onNavigate={navigate} />
      </div>
      <div
        className="v5-shell-content"
        style={{
          flex: 1,
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
          minWidth: 0,
        }}
      >
        <V5TopBar
          title={meta?.label ?? ''}
          breadcrumb={breadcrumbFor(screen)}
          onOpenNavigation={() => setMobileNavOpen(true)}
          onOpenCommand={() => setCmdOpen(true)}
          onToggleAssistant={() => setAssistantOpen((o) => !o)}
          assistantOpen={assistantOpen}
        />
        <div className="v5-shell-stage" style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
          <main
            key={screen}
            // Feature: viewTransitions — apply fade-up animation only when on.
            className={`v5-shell-main ${settings.features.viewTransitions ? 'v5-ani' : ''}`.trim()}
            style={{
              flex: 1,
              overflow: 'hidden',
              display: 'flex',
              flexDirection: 'column',
              minWidth: 0,
            }}
          >
            <ScreenComponent navigate={navigate} />
          </main>
          <V5AssistantPanel open={assistantOpen} />
        </div>
      </div>
      <V5CommandPalette
        open={cmdOpen}
        onClose={() => setCmdOpen(false)}
        onNavigate={navigate}
      />
      <V5MobileNav
        open={mobileNavOpen}
        active={screen}
        onClose={() => setMobileNavOpen(false)}
        onNavigate={navigate}
      />
    </div>
  );
}

// Settings sync helper lives in `./settings/syncToServer` so it can be
// unit-tested independently — see `syncToServer.test.ts`. It throws when
// the API resolves with `success: false` so the provider correctly flips
// to `error` rather than masking 401/404/413 as `saved`.

export function V5App() {
  // Lazy client creation keeps SSR & test environments from booting a query
  // store they'll never use. One store per app instance is intentional.
  const [queryClient] = useState(makeQueryClient);

  return (
    <QueryClientProvider client={queryClient}>
      <V5ThemeProvider>
        <V5SettingsProvider syncToServer={syncSettingsToServer}>
          <V5ToastProvider>
            <SettingsBridge />
            <V5Shell />
          </V5ToastProvider>
        </V5SettingsProvider>
      </V5ThemeProvider>
    </QueryClientProvider>
  );
}
