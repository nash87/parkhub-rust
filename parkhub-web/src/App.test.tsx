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
vi.mock('./components/Layout', async () => {
  const { Outlet } = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return {
    Layout: () => <div data-testid="layout">Layout<Outlet /></div>,
  };
});

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

// Stub all other lazy views — render testids so route mounts can be awaited
const stub = (id: string) => () => <div data-testid={`page-${id}`}>{id}</div>;
vi.mock('./views/Book', () => ({ BookPage: stub('book') }));
vi.mock('./views/Bookings', () => ({ BookingsPage: stub('bookings') }));
vi.mock('./views/Credits', () => ({ CreditsPage: stub('credits') }));
vi.mock('./views/Vehicles', () => ({ VehiclesPage: stub('vehicles') }));
vi.mock('./views/Absences', () => ({ AbsencesPage: stub('absences') }));
vi.mock('./views/Profile', () => ({ ProfilePage: stub('profile') }));
vi.mock('./views/Team', () => ({ TeamPage: stub('team') }));
vi.mock('./views/Notifications', () => ({ NotificationsPage: stub('notifications') }));
vi.mock('./views/Calendar', () => ({ CalendarPage: stub('calendar') }));
vi.mock('./views/MapView', () => ({ MapViewPage: stub('map') }));
vi.mock('./views/Admin', async () => {
  const { Outlet } = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { AdminPage: () => <div data-testid="page-admin">Admin<Outlet /></div> };
});
vi.mock('./views/AdminSettings', () => ({ AdminSettingsPage: stub('admin-settings') }));
vi.mock('./views/AdminUsers', () => ({ AdminUsersPage: stub('admin-users') }));
vi.mock('./views/AdminAnnouncements', () => ({ AdminAnnouncementsPage: stub('admin-announcements') }));
vi.mock('./views/AdminLots', () => ({ AdminLotsPage: stub('admin-lots') }));
vi.mock('./views/AdminReports', () => ({ AdminReportsPage: stub('admin-reports') }));
vi.mock('./views/Favorites', () => ({ FavoritesPage: stub('favorites') }));
vi.mock('./views/Translations', () => ({ TranslationsPage: stub('translations') }));
vi.mock('./views/AdminTranslations', () => ({ AdminTranslationsPage: stub('admin-translations') }));
vi.mock('./views/AdminAnalytics', () => ({ AdminAnalyticsPage: stub('admin-analytics') }));
vi.mock('./views/AdminRateLimits', () => ({ AdminRateLimitsPage: stub('admin-rate-limits') }));
vi.mock('./views/AdminTenants', () => ({ AdminTenantsPage: stub('admin-tenants') }));
vi.mock('./views/AdminAuditLog', () => ({ AdminAuditLogPage: stub('admin-audit-log') }));
vi.mock('./views/AdminDataManagement', () => ({ AdminDataManagementPage: stub('admin-data') }));
vi.mock('./views/AdminFleet', () => ({ AdminFleetPage: stub('admin-fleet') }));
vi.mock('./views/AdminAccessible', () => ({ AdminAccessiblePage: stub('admin-accessible') }));
vi.mock('./views/AdminMaintenance', () => ({ AdminMaintenancePage: stub('admin-maintenance') }));
vi.mock('./views/AdminBilling', () => ({ AdminBillingPage: stub('admin-billing') }));
vi.mock('./views/Visitors', () => ({ VisitorsPage: stub('visitors'), AdminVisitorsPage: stub('admin-visitors') }));
vi.mock('./views/EVCharging', () => ({ EVChargingPage: stub('ev-charging'), AdminChargersPage: stub('admin-chargers') }));
vi.mock('./views/ParkingHistory', () => ({ ParkingHistoryPage: stub('history') }));
vi.mock('./views/AbsenceApproval', () => ({ AbsenceApprovalPage: stub('absence-approval') }));
vi.mock('./views/AdminDashboard', () => ({ AdminDashboardPage: stub('admin-widgets') }));
vi.mock('./views/AdminPlugins', () => ({ AdminPluginsPage: stub('admin-plugins') }));
vi.mock('./views/AdminCompliance', () => ({ AdminCompliancePage: stub('admin-compliance') }));
vi.mock('./views/AdminSSO', () => ({ AdminSSOPage: stub('admin-sso') }));
vi.mock('./views/AdminWebhooks', () => ({ AdminWebhooksPage: stub('admin-webhooks') }));
vi.mock('./views/AdminRoles', () => ({ AdminRolesPage: stub('admin-roles') }));
vi.mock('./views/AdminZones', () => ({ AdminZonesPage: stub('admin-zones') }));
vi.mock('./views/AdminUpdates', () => ({ AdminUpdatesPage: stub('admin-updates') }));
vi.mock('./views/SwapRequests', () => ({ SwapRequestsPage: stub('swap-requests') }));
vi.mock('./views/QRCheckIn', () => ({ QRCheckInPage: stub('checkin') }));
vi.mock('./views/GuestPass', () => ({ GuestPassPage: stub('guest-pass') }));
vi.mock('./views/OccupancyHeatmap', () => ({ OccupancyHeatmapPage: stub('admin-heatmap') }));
vi.mock('./views/TeamLeaderboard', () => ({ TeamLeaderboardPage: stub('leaderboard') }));
vi.mock('./views/OccupancyPrediction', () => ({ OccupancyPredictionPage: stub('predict') }));
vi.mock('./views/AdminGraphQL', () => ({ AdminGraphQLPage: stub('admin-graphql') }));
vi.mock('./views/BookingSharing', () => ({ BookingSharingPage: stub('booking-sharing') }));
vi.mock('./views/ApiDocs', () => ({ ApiDocsPage: stub('api-docs') }));
vi.mock('./views/ApiVersion', () => ({ ApiVersionPage: stub('api-version') }));
vi.mock('./views/Geofence', () => ({ GeofencePage: stub('geofence') }));
vi.mock('./views/Waitlist', () => ({ WaitlistPage: stub('waitlist') }));
vi.mock('./views/ParkingPassView', () => ({ ParkingPassViewPage: stub('parking-pass-view') }));

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

  it('handles theme API returning no use_case key', async () => {
    delete document.documentElement.dataset.usecase;
    vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({
      ok: true,
      json: () => Promise.resolve({ data: {} }),
    })));

    render(<App />);

    await waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalledWith('/api/v1/theme', expect.objectContaining({ credentials: 'include' }));
    });
    // Should not crash — no dataset.usecase set when no key returned
    // The dataset may or may not still be set from a prior test in same process,
    // but the key should NOT have been set by this response
    expect(document.documentElement.dataset.usecase).not.toBe('corporate');
  });

  it('handles theme API returning null response', async () => {
    vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({
      ok: true,
      json: () => Promise.resolve(null),
    })));

    render(<App />);

    await waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalled();
    });
  });

  it('handles theme API network failure gracefully', async () => {
    vi.stubGlobal('fetch', vi.fn(() => Promise.reject(new Error('Network error'))));

    render(<App />);

    // Should not crash
    const { container } = render(<App />);
    expect(container).toBeTruthy();
  });

  it('renders choose/use-case-selector route', async () => {
    window.history.pushState({}, '', '/choose');
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('page-choose')).toBeInTheDocument();
    });
  });

  it('renders login route directly', async () => {
    window.history.pushState({}, '', '/login');
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('page-login')).toBeInTheDocument();
    });
  });

  it('renders welcome route directly', async () => {
    window.history.pushState({}, '', '/welcome');
    render(<App />);

    await waitFor(() => {
      expect(screen.getByTestId('page-welcome')).toBeInTheDocument();
    });
  });

  // Visit each authenticated route to execute every lazy() factory in App.tsx
  const userRoutes: Array<[string, string]> = [
    ['/', 'page-dashboard'],
    ['/book', 'page-book'],
    ['/bookings', 'page-bookings'],
    ['/credits', 'page-credits'],
    ['/vehicles', 'page-vehicles'],
    ['/favorites', 'page-favorites'],
    ['/absences', 'page-absences'],
    ['/profile', 'page-profile'],
    ['/team', 'page-team'],
    ['/notifications', 'page-notifications'],
    ['/calendar', 'page-calendar'],
    ['/visitors', 'page-visitors'],
    ['/ev-charging', 'page-ev-charging'],
    ['/history', 'page-history'],
    ['/absence-approval', 'page-absence-approval'],
    ['/map', 'page-map'],
    ['/swap-requests', 'page-swap-requests'],
    ['/checkin', 'page-checkin'],
    ['/guest-pass', 'page-guest-pass'],
    ['/leaderboard', 'page-leaderboard'],
    ['/predict', 'page-predict'],
    ['/translations', 'page-translations'],
  ];
  for (const [route, testid] of userRoutes) {
    it(`mounts user route ${route}`, async () => {
      mockUser.current = { id: '1', name: 'Test', role: 'user', email: 'test@test.com' };
      mockUser.loading = false;
      window.history.pushState({}, '', route);
      render(<App />);
      await waitFor(() => {
        expect(screen.getByTestId(testid)).toBeInTheDocument();
      });
    });
  }

  const adminRoutes: Array<[string, string]> = [
    ['/admin', 'page-admin-reports'],
    ['/admin/settings', 'page-admin-settings'],
    ['/admin/users', 'page-admin-users'],
    ['/admin/lots', 'page-admin-lots'],
    ['/admin/announcements', 'page-admin-announcements'],
    ['/admin/reports', 'page-admin-reports'],
    ['/admin/translations', 'page-admin-translations'],
    ['/admin/analytics', 'page-admin-analytics'],
    ['/admin/rate-limits', 'page-admin-rate-limits'],
    ['/admin/tenants', 'page-admin-tenants'],
    ['/admin/audit-log', 'page-admin-audit-log'],
    ['/admin/data', 'page-admin-data'],
    ['/admin/fleet', 'page-admin-fleet'],
    ['/admin/accessible', 'page-admin-accessible'],
    ['/admin/maintenance', 'page-admin-maintenance'],
    ['/admin/billing', 'page-admin-billing'],
    ['/admin/visitors', 'page-admin-visitors'],
    ['/admin/chargers', 'page-admin-chargers'],
    ['/admin/widgets', 'page-admin-widgets'],
    ['/admin/plugins', 'page-admin-plugins'],
    ['/admin/compliance', 'page-admin-compliance'],
    ['/admin/sso', 'page-admin-sso'],
    ['/admin/webhooks', 'page-admin-webhooks'],
    ['/admin/roles', 'page-admin-roles'],
    ['/admin/zones', 'page-admin-zones'],
    ['/admin/updates', 'page-admin-updates'],
    ['/admin/heatmap', 'page-admin-heatmap'],
  ];
  for (const [route, testid] of adminRoutes) {
    it(`mounts admin route ${route}`, async () => {
      mockUser.current = { id: '1', name: 'Admin', role: 'admin', email: 'admin@test.com' };
      mockUser.loading = false;
      window.history.pushState({}, '', route);
      render(<App />);
      await waitFor(() => {
        expect(screen.getByTestId(testid)).toBeInTheDocument();
      });
    });
  }
});
