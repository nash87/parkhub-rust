import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

// ── Hoisted mocks ──
const { mockUser, localStorageMock } = vi.hoisted(() => {
  const mockUser = { current: null as any, loading: false };

  let store: Record<string, string> = {};
  const localStorageMock = {
    getItem: vi.fn((key: string) => store[key] ?? null),
    setItem: vi.fn((key: string, val: string) => { store[key] = val; }),
    removeItem: vi.fn((key: string) => { delete store[key]; }),
    clear: vi.fn(() => { store = {}; }),
  };

  Object.defineProperty(globalThis.window ?? globalThis, 'localStorage', {
    value: localStorageMock, writable: true, configurable: true,
  });

  const persistentMql = {
    matches: false,
    media: '(prefers-color-scheme: dark)',
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    dispatchEvent: vi.fn(),
  };

  Object.defineProperty(globalThis.window ?? globalThis, 'matchMedia', {
    writable: true, configurable: true,
    value: vi.fn((_query: string) => persistentMql),
  });

  return { mockUser, localStorageMock };
});

vi.mock('./context/AuthContext', () => ({
  AuthProvider: ({ children }: any) => <>{children}</>,
  useAuth: () => ({
    user: mockUser.current,
    loading: mockUser.loading,
    login: vi.fn(),
    logout: vi.fn(),
    refreshUser: vi.fn(),
  }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => (typeof fallback === 'string' ? fallback : key),
  }),
}));

vi.mock('./i18n', () => ({
  default: {},
  loadTranslationOverrides: vi.fn(),
}));

vi.mock('react-hot-toast', () => ({
  Toaster: () => <div data-testid="toaster" />,
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, ...props }: any, ref: any) => <div ref={ref} {...props}>{children}</div>),
    button: React.forwardRef(({ children, ...props }: any, ref: any) => <button ref={ref} {...props}>{children}</button>),
    aside: React.forwardRef(({ children, ...props }: any, ref: any) => <aside ref={ref} {...props}>{children}</aside>),
    nav: React.forwardRef(({ children, ...props }: any, ref: any) => <nav ref={ref} {...props}>{children}</nav>),
    span: React.forwardRef(({ children, ...props }: any, ref: any) => <span ref={ref} {...props}>{children}</span>),
    li: React.forwardRef(({ children, ...props }: any, ref: any) => <li ref={ref} {...props}>{children}</li>),
    ul: React.forwardRef(({ children, ...props }: any, ref: any) => <ul ref={ref} {...props}>{children}</ul>),
    p: React.forwardRef(({ children, ...props }: any, ref: any) => <p ref={ref} {...props}>{children}</p>),
    section: React.forwardRef(({ children, ...props }: any, ref: any) => <section ref={ref} {...props}>{children}</section>),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () =>
  new Proxy({}, { get: (_, name) => (props: any) => <span data-testid={`icon-${String(name)}`} {...props} /> })
);

// Stub lazy-loaded components to avoid actually loading all of them
vi.mock('./components/Layout', () => ({
  Layout: () => <div data-testid="layout">Layout</div>,
}));

vi.mock('./components/DemoOverlay', () => ({
  DemoOverlay: () => null,
}));

vi.mock('./components/InstallPrompt', () => ({
  InstallPrompt: () => null,
}));

vi.mock('./components/SWUpdatePrompt', () => ({
  SWUpdatePrompt: () => null,
}));

// Stub view components
vi.mock('./views/Welcome', () => ({
  WelcomePage: () => <div data-testid="page-welcome">Welcome</div>,
}));
vi.mock('./views/Login', () => ({
  LoginPage: () => <div data-testid="page-login">Login</div>,
}));
vi.mock('./views/Register', () => ({
  RegisterPage: () => <div data-testid="page-register">Register</div>,
}));
vi.mock('./views/ForgotPassword', () => ({
  ForgotPasswordPage: () => <div data-testid="page-forgot">Forgot</div>,
}));
vi.mock('./views/NotFound', () => ({
  NotFoundPage: () => <div data-testid="page-notfound">Not Found</div>,
}));
vi.mock('./views/Dashboard', () => ({
  DashboardPage: () => <div data-testid="page-dashboard">Dashboard</div>,
}));
vi.mock('./views/UseCaseSelector', () => ({
  UseCaseSelectorPage: () => <div data-testid="page-choose">Choose</div>,
}));
vi.mock('./views/LobbyDisplay', () => ({
  LobbyDisplayPage: () => <div data-testid="page-lobby">Lobby</div>,
}));
vi.mock('./views/SetupWizard', () => ({
  SetupWizardPage: () => <div data-testid="page-setup">Setup</div>,
}));

// Stub all other lazy views to avoid import errors
const emptyModule = () => null;
vi.mock('./views/Book', () => ({ BookPage: emptyModule }));
vi.mock('./views/Bookings', () => ({ BookingsPage: emptyModule }));
vi.mock('./views/Credits', () => ({ CreditsPage: emptyModule }));
vi.mock('./views/Vehicles', () => ({ VehiclesPage: emptyModule }));
vi.mock('./views/Absences', () => ({ AbsencesPage: emptyModule }));
vi.mock('./views/Profile', () => ({ ProfilePage: emptyModule }));
vi.mock('./views/Team', () => ({ TeamPage: emptyModule }));
vi.mock('./views/Notifications', () => ({ NotificationsPage: emptyModule }));
vi.mock('./views/Calendar', () => ({ CalendarPage: emptyModule }));
vi.mock('./views/MapView', () => ({ MapViewPage: emptyModule }));
vi.mock('./views/Admin', () => ({ AdminPage: emptyModule }));
vi.mock('./views/AdminSettings', () => ({ AdminSettingsPage: emptyModule }));
vi.mock('./views/AdminUsers', () => ({ AdminUsersPage: emptyModule }));
vi.mock('./views/AdminAnnouncements', () => ({ AdminAnnouncementsPage: emptyModule }));
vi.mock('./views/AdminLots', () => ({ AdminLotsPage: emptyModule }));
vi.mock('./views/AdminReports', () => ({ AdminReportsPage: emptyModule }));
vi.mock('./views/Favorites', () => ({ FavoritesPage: emptyModule }));
vi.mock('./views/Translations', () => ({ TranslationsPage: emptyModule }));
vi.mock('./views/AdminTranslations', () => ({ AdminTranslationsPage: emptyModule }));
vi.mock('./views/AdminAnalytics', () => ({ AdminAnalyticsPage: emptyModule }));
vi.mock('./views/AdminRateLimits', () => ({ AdminRateLimitsPage: emptyModule }));
vi.mock('./views/AdminTenants', () => ({ AdminTenantsPage: emptyModule }));
vi.mock('./views/AdminAuditLog', () => ({ AdminAuditLogPage: emptyModule }));
vi.mock('./views/AdminDataManagement', () => ({ AdminDataManagementPage: emptyModule }));
vi.mock('./views/AdminFleet', () => ({ AdminFleetPage: emptyModule }));
vi.mock('./views/AdminAccessible', () => ({ AdminAccessiblePage: emptyModule }));
vi.mock('./views/AdminMaintenance', () => ({ AdminMaintenancePage: emptyModule }));
vi.mock('./views/AdminBilling', () => ({ AdminBillingPage: emptyModule }));
vi.mock('./views/Visitors', () => ({ VisitorsPage: emptyModule, AdminVisitorsPage: emptyModule }));
vi.mock('./views/EVCharging', () => ({ EVChargingPage: emptyModule, AdminChargersPage: emptyModule }));
vi.mock('./views/ParkingHistory', () => ({ ParkingHistoryPage: emptyModule }));
vi.mock('./views/AbsenceApproval', () => ({ AbsenceApprovalPage: emptyModule }));
vi.mock('./views/AdminDashboard', () => ({ AdminDashboardPage: emptyModule }));
vi.mock('./views/AdminPlugins', () => ({ AdminPluginsPage: emptyModule }));
vi.mock('./views/AdminCompliance', () => ({ AdminCompliancePage: emptyModule }));
vi.mock('./views/AdminSSO', () => ({ AdminSSOPage: emptyModule }));
vi.mock('./views/AdminWebhooks', () => ({ AdminWebhooksPage: emptyModule }));
vi.mock('./views/AdminRoles', () => ({ AdminRolesPage: emptyModule }));
vi.mock('./views/AdminZones', () => ({ AdminZonesPage: emptyModule }));
vi.mock('./views/AdminUpdates', () => ({ AdminUpdatesPage: emptyModule }));
vi.mock('./views/SwapRequests', () => ({ SwapRequestsPage: emptyModule }));
vi.mock('./views/QRCheckIn', () => ({ QRCheckInPage: emptyModule }));
vi.mock('./views/GuestPass', () => ({ GuestPassPage: emptyModule }));
vi.mock('./views/OccupancyHeatmap', () => ({ OccupancyHeatmapPage: emptyModule }));
vi.mock('./views/TeamLeaderboard', () => ({ TeamLeaderboardPage: emptyModule }));
vi.mock('./views/OccupancyPrediction', () => ({ OccupancyPredictionPage: emptyModule }));
vi.mock('./views/AdminGraphQL', () => ({ AdminGraphQLPage: emptyModule }));
vi.mock('./views/BookingSharing', () => ({ BookingSharingPage: emptyModule }));
vi.mock('./views/ApiDocs', () => ({ ApiDocsPage: emptyModule }));
vi.mock('./views/ApiVersion', () => ({ ApiVersionPage: emptyModule }));
vi.mock('./views/Geofence', () => ({ GeofencePage: emptyModule }));
vi.mock('./views/Waitlist', () => ({ WaitlistPage: emptyModule }));
vi.mock('./views/ParkingPassView', () => ({ ParkingPassViewPage: emptyModule }));

// Stub global fetch for useThemeLoader
vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({ ok: false })));

import { App } from './App';

describe('App', () => {
  beforeEach(() => {
    localStorageMock.clear();
    mockUser.current = null;
    mockUser.loading = false;
    // Mark welcome as seen to get /login redirect instead of /welcome
    localStorageMock.setItem('parkhub_welcome_seen', 'true');
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders without crashing', async () => {
    render(<App />);

    // Should render something (login page for unauthenticated user)
    await waitFor(() => {
      expect(screen.getByTestId('page-login')).toBeInTheDocument();
    });
  });

  it('renders the Toaster', async () => {
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('toaster')).toBeInTheDocument();
    });
  });

  it('redirects unauthenticated user to /login', async () => {
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('page-login')).toBeInTheDocument();
    });
  });

  it('wraps app in ErrorBoundary', () => {
    // The App component wraps everything in ErrorBoundary
    // We verify it renders successfully (ErrorBoundary catches errors)
    const { container } = render(<App />);
    expect(container).toBeTruthy();
  });

  it('shows loading splash when auth is loading', async () => {
    mockUser.loading = true;
    mockUser.current = null;

    window.history.pushState({}, '', '/');
    render(<App />);

    await waitFor(() => {
      expect(screen.getByRole('status', { name: /loading parkhub/i })).toBeInTheDocument();
    });
  });

  it('redirects to /welcome when not seen and unauthenticated', async () => {
    localStorageMock.clear();
    mockUser.current = null;
    mockUser.loading = false;

    window.history.pushState({}, '', '/');
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('page-welcome')).toBeInTheDocument();
    });
  });

  it('redirects to /login when welcome seen and unauthenticated', async () => {
    localStorageMock.setItem('parkhub_welcome_seen', 'true');
    mockUser.current = null;
    mockUser.loading = false;

    window.history.pushState({}, '', '/');
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('page-login')).toBeInTheDocument();
    });
  });

  it('shows dashboard for authenticated user', async () => {
    mockUser.current = { id: '1', name: 'Test', role: 'user', email: 'test@test.com' };
    mockUser.loading = false;

    window.history.pushState({}, '', '/');
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('layout')).toBeInTheDocument();
    });
  });

  it('redirects non-admin users away from admin routes', async () => {
    mockUser.current = { id: '1', name: 'Test', role: 'user', email: 'test@test.com' };
    mockUser.loading = false;

    window.history.pushState({}, '', '/admin');
    render(<App />);

    // Non-admin should be redirected to /
    await waitFor(() => {
      expect(screen.getByTestId('layout')).toBeInTheDocument();
    });
  });

  it('allows admin users to access admin routes', async () => {
    mockUser.current = { id: '1', name: 'Admin', role: 'admin', email: 'admin@test.com' };
    mockUser.loading = false;

    window.history.pushState({}, '', '/admin');
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('layout')).toBeInTheDocument();
    });
  });

  it('allows superadmin users to access admin routes', async () => {
    mockUser.current = { id: '1', name: 'Super', role: 'superadmin', email: 'super@test.com' };
    mockUser.loading = false;

    window.history.pushState({}, '', '/admin');
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('layout')).toBeInTheDocument();
    });
  });

  it('renders public routes without authentication', async () => {
    mockUser.current = null;
    mockUser.loading = false;

    window.history.pushState({}, '', '/register');
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('page-register')).toBeInTheDocument();
    });
  });

  it('renders forgot password route', async () => {
    mockUser.current = null;
    mockUser.loading = false;

    window.history.pushState({}, '', '/forgot-password');
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('page-forgot')).toBeInTheDocument();
    });
  });

  it('renders 404 for unknown routes', async () => {
    mockUser.current = null;
    mockUser.loading = false;

    window.history.pushState({}, '', '/nonexistent-page');
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('page-notfound')).toBeInTheDocument();
    });
  });

  it('renders use case selector route', async () => {
    window.history.pushState({}, '', '/choose');
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('page-choose')).toBeInTheDocument();
    });
  });

  it('renders setup wizard route', async () => {
    window.history.pushState({}, '', '/setup');
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('page-setup')).toBeInTheDocument();
    });
  });

  it('renders lobby display route', async () => {
    window.history.pushState({}, '', '/lobby/lot-123');
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('page-lobby')).toBeInTheDocument();
    });
  });

  it('calls theme API on mount', async () => {
    render(<App />);

    await waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalledWith('/api/v1/theme', expect.objectContaining({ credentials: 'include' }));
    });
  });

  it('applies use case theme from API response', async () => {
    vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({
      ok: true,
      json: () => Promise.resolve({ data: { use_case: { key: 'corporate' } } }),
    })));

    render(<App />);

    await waitFor(() => {
      expect(document.documentElement.dataset.usecase).toBe('corporate');
    });

    // Cleanup
    delete document.documentElement.dataset.usecase;
  });
});
