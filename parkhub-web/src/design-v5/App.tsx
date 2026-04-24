import { useCallback, useEffect, useState, type ComponentType } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { V5ThemeProvider } from './ThemeProvider';
import { V5ToastProvider } from './Toast';
import { V5Sidebar } from './Sidebar';
import { V5TopBar } from './TopBar';
import { V5CommandPalette } from './CommandPalette';
import { V5AIPanel } from './AIPanel';
import { breadcrumbFor, byId, type ScreenId } from './nav';
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

import './fonts';
import './tokens.css';

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

function V5Shell() {
  const [screen, setScreen] = useState<ScreenId>(() => {
    if (typeof window === 'undefined') return DEFAULT_SCREEN;
    const stored = window.localStorage.getItem(STORAGE_KEY) as ScreenId | null;
    return stored && byId.has(stored) ? stored : DEFAULT_SCREEN;
  });
  const [cmdOpen, setCmdOpen] = useState(false);
  const [aiOpen, setAiOpen] = useState(false);

  useEffect(() => {
    window.localStorage.setItem(STORAGE_KEY, screen);
  }, [screen]);

  // Keyboard shortcuts: ⌘K / Ctrl+K for palette, ? for AI panel.
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setCmdOpen((o) => !o);
        return;
      }
      // Don't steal ? while the user is typing in a form
      if (e.key === '?' && !(e.target instanceof HTMLInputElement) && !(e.target instanceof HTMLTextAreaElement) && !(e.target instanceof HTMLSelectElement)) {
        setAiOpen((o) => !o);
      }
    };
    window.addEventListener('keydown', h);
    return () => window.removeEventListener('keydown', h);
  }, []);

  const navigate = useCallback((id: ScreenId) => {
    setScreen(id);
    setCmdOpen(false);
  }, []);

  const meta = byId.get(screen);
  const ScreenComponent = SCREENS[screen];

  return (
    <div
      style={{
        width: '100%',
        height: '100dvh',
        display: 'flex',
        background: 'var(--v5-bg)',
        color: 'var(--v5-txt)',
        overflow: 'hidden',
        fontFamily: "'Inter Variable', 'Inter', system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif",
        fontFeatureSettings: '"cv11", "ss01"',
      }}
    >
      <V5Sidebar active={screen} onNavigate={navigate} />
      <div
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
          onOpenCommand={() => setCmdOpen(true)}
          onToggleAI={() => setAiOpen((o) => !o)}
          aiOpen={aiOpen}
        />
        <div style={{ flex: 1, display: 'flex', overflow: 'hidden' }}>
          <main
            key={screen}
            className="v5-ani"
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
          <V5AIPanel open={aiOpen} />
        </div>
      </div>
      <V5CommandPalette
        open={cmdOpen}
        onClose={() => setCmdOpen(false)}
        onNavigate={navigate}
      />
    </div>
  );
}

export function V5App() {
  // Lazy client creation keeps SSR & test environments from booting a query
  // store they'll never use. One store per app instance is intentional.
  const [queryClient] = useState(makeQueryClient);

  return (
    <QueryClientProvider client={queryClient}>
      <V5ThemeProvider>
        <V5ToastProvider>
          <V5Shell />
        </V5ToastProvider>
      </V5ThemeProvider>
    </QueryClientProvider>
  );
}
