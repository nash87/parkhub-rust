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
import { FahrzeugeV5 } from './screens/Fahrzeuge';
import { CreditsV5 } from './screens/Credits';
import { PlaceholderV5 } from './screens/Placeholder';

import './fonts';
import './tokens.css';

/**
 * Registry of fully-ported v5 screens. Screens missing from this map fall
 * back to <PlaceholderV5 />, which links out to the legacy v4 route so
 * navigation never dead-ends during the migration.
 */
const SCREENS: Partial<Record<ScreenId, ComponentType<{ navigate: (id: ScreenId) => void }>>> = {
  dashboard: DashboardV5,
  buchungen: BuchungenV5,
  fahrzeuge: FahrzeugeV5,
  credits: CreditsV5,
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
            {ScreenComponent ? (
              <ScreenComponent navigate={navigate} />
            ) : (
              <PlaceholderV5 id={screen} />
            )}
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
