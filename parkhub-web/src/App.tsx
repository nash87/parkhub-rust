import React, { useEffect, Suspense, useRef, startTransition } from 'react';
import { BrowserRouter, Routes, Route, Navigate, useLocation } from 'react-router-dom';
import { Toaster } from 'react-hot-toast';
import { AuthProvider, useAuth } from './context/AuthContext';
import { ThemeProvider } from './context/ThemeContext';
import { UseCaseProvider } from './context/UseCaseContext';
import { FeaturesProvider } from './context/FeaturesContext';
import { registerRoute, preloadRoutesIdle } from './lib/routePreload';
import './i18n';
import { loadTranslationOverrides } from './i18n';

// Eagerly loaded shell
import { Layout } from './components/Layout';
import { ErrorBoundary } from './components/ErrorBoundary';

// Lazy helper — wraps named exports for React.lazy and auto-registers
// the module loader for hover-intent prefetching.
function lazy<T extends Record<string, React.ComponentType>>(
  loader: () => Promise<T>,
  name: keyof T,
  path?: string,
) {
  if (path) registerRoute(path, loader);
  return React.lazy(() => loader().then(m => ({ default: m[name] as React.ComponentType })));
}

// Auth pages (small, on critical path for unauthenticated users)
const WelcomePage = lazy(() => import('./views/Welcome'), 'WelcomePage', '/welcome');
const LoginPage = lazy(() => import('./views/Login'), 'LoginPage', '/login');
const RegisterPage = lazy(() => import('./views/Register'), 'RegisterPage', '/register');
const ForgotPasswordPage = lazy(() => import('./views/ForgotPassword'), 'ForgotPasswordPage', '/forgot-password');
const UseCaseSelectorPage = lazy(() => import('./views/UseCaseSelector'), 'UseCaseSelectorPage');
const NotFoundPage = lazy(() => import('./views/NotFound'), 'NotFoundPage');
const LobbyDisplayPage = lazy(() => import('./views/LobbyDisplay'), 'LobbyDisplayPage');
const SetupWizardPage = lazy(() => import('./views/SetupWizard'), 'SetupWizardPage');

// Main app pages — paths registered for hover-intent prefetch
const DashboardPage = lazy(() => import('./views/Dashboard'), 'DashboardPage', '/');
const BookPage = lazy(() => import('./views/Book'), 'BookPage', '/book');
const BookingsPage = lazy(() => import('./views/Bookings'), 'BookingsPage', '/bookings');
const CreditsPage = lazy(() => import('./views/Credits'), 'CreditsPage', '/credits');
const VehiclesPage = lazy(() => import('./views/Vehicles'), 'VehiclesPage', '/vehicles');
const AbsencesPage = lazy(() => import('./views/Absences'), 'AbsencesPage', '/absences');
const ProfilePage = lazy(() => import('./views/Profile'), 'ProfilePage', '/profile');
const TeamPage = lazy(() => import('./views/Team'), 'TeamPage', '/team');
const NotificationsPage = lazy(() => import('./views/Notifications'), 'NotificationsPage', '/notifications');
const CalendarPage = lazy(() => import('./views/Calendar'), 'CalendarPage', '/calendar');
const DemoOverlay = lazy(() => import('./components/DemoOverlay'), 'DemoOverlay');
const InstallPrompt = lazy(() => import('./components/InstallPrompt'), 'InstallPrompt');
const SWUpdatePrompt = lazy(() => import('./components/SWUpdatePrompt'), 'SWUpdatePrompt');

// Admin pages
const MapViewPage = lazy(() => import('./views/MapView'), 'MapViewPage', '/map');
const AdminPage = lazy(() => import('./views/Admin'), 'AdminPage', '/admin');
const AdminSettingsPage = lazy(() => import('./views/AdminSettings'), 'AdminSettingsPage', '/admin/settings');
const AdminUsersPage = lazy(() => import('./views/AdminUsers'), 'AdminUsersPage', '/admin/users');
const AdminAnnouncementsPage = lazy(() => import('./views/AdminAnnouncements'), 'AdminAnnouncementsPage', '/admin/announcements');
const AdminLotsPage = lazy(() => import('./views/AdminLots'), 'AdminLotsPage', '/admin/lots');
const AdminReportsPage = lazy(() => import('./views/AdminReports'), 'AdminReportsPage', '/admin/reports');
const FavoritesPage = lazy(() => import('./views/Favorites'), 'FavoritesPage', '/favorites');
const TranslationsPage = lazy(() => import('./views/Translations'), 'TranslationsPage', '/translations');
const AdminTranslationsPage = lazy(() => import('./views/AdminTranslations'), 'AdminTranslationsPage', '/admin/translations');
const AdminAnalyticsPage = lazy(() => import('./views/AdminAnalytics'), 'AdminAnalyticsPage', '/admin/analytics');
const AdminRateLimitsPage = lazy(() => import('./views/AdminRateLimits'), 'AdminRateLimitsPage', '/admin/rate-limits');
const AdminTenantsPage = lazy(() => import('./views/AdminTenants'), 'AdminTenantsPage', '/admin/tenants');
const AdminAuditLogPage = lazy(() => import('./views/AdminAuditLog'), 'AdminAuditLogPage', '/admin/audit-log');
const AdminDataManagementPage = lazy(() => import('./views/AdminDataManagement'), 'AdminDataManagementPage', '/admin/data');
const AdminFleetPage = lazy(() => import('./views/AdminFleet'), 'AdminFleetPage', '/admin/fleet');
const AdminAccessiblePage = lazy(() => import('./views/AdminAccessible'), 'AdminAccessiblePage', '/admin/accessible');
const AdminMaintenancePage = lazy(() => import('./views/AdminMaintenance'), 'AdminMaintenancePage', '/admin/maintenance');
const AdminBillingPage = lazy(() => import('./views/AdminBilling'), 'AdminBillingPage', '/admin/billing');
const VisitorsPage = lazy(() => import('./views/Visitors'), 'VisitorsPage', '/visitors');
const AdminVisitorsPage = lazy(() => import('./views/Visitors'), 'AdminVisitorsPage', '/admin/visitors');
const EVChargingPage = lazy(() => import('./views/EVCharging'), 'EVChargingPage', '/ev-charging');
const AdminChargersPage = lazy(() => import('./views/EVCharging'), 'AdminChargersPage', '/admin/chargers');
const ParkingHistoryPage = lazy(() => import('./views/ParkingHistory'), 'ParkingHistoryPage', '/history');
const AbsenceApprovalPage = lazy(() => import('./views/AbsenceApproval'), 'AbsenceApprovalPage', '/absence-approval');
const AdminDashboardPage = lazy(() => import('./views/AdminDashboard'), 'AdminDashboardPage', '/admin/widgets');
const AdminPluginsPage = lazy(() => import('./views/AdminPlugins'), 'AdminPluginsPage', '/admin/plugins');
const AdminCompliancePage = lazy(() => import('./views/AdminCompliance'), 'AdminCompliancePage', '/admin/compliance');
const AdminSSOPage = lazy(() => import('./views/AdminSSO'), 'AdminSSOPage', '/admin/sso');
const AdminWebhooksPage = lazy(() => import('./views/AdminWebhooks'), 'AdminWebhooksPage', '/admin/webhooks');
const AdminRolesPage = lazy(() => import('./views/AdminRoles'), 'AdminRolesPage', '/admin/roles');
const AdminZonesPage = lazy(() => import('./views/AdminZones'), 'AdminZonesPage', '/admin/zones');
const AdminUpdatesPage = lazy(() => import('./views/AdminUpdates'), 'AdminUpdatesPage', '/admin/updates');

// New feature pages
const SwapRequestsPage = lazy(() => import('./views/SwapRequests'), 'SwapRequestsPage', '/swap-requests');
const QRCheckInPage = lazy(() => import('./views/QRCheckIn'), 'QRCheckInPage', '/checkin');
const GuestPassPage = lazy(() => import('./views/GuestPass'), 'GuestPassPage', '/guest-pass');
const OccupancyHeatmapPage = lazy(() => import('./views/OccupancyHeatmap'), 'OccupancyHeatmapPage', '/admin/heatmap');
const TeamLeaderboardPage = lazy(() => import('./views/TeamLeaderboard'), 'TeamLeaderboardPage', '/leaderboard');
const OccupancyPredictionPage = lazy(() => import('./views/OccupancyPrediction'), 'OccupancyPredictionPage', '/predict');

// Scheduled reports page (admin)
const AdminScheduledReportsPage = (() => {
  try { return lazy(() => import('./views/AdminScheduledReports'), 'AdminScheduledReportsPage', '/admin/scheduled-reports'); }
  catch { return () => null; }
})();

function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { user, loading } = useAuth();
  if (loading) return <LoadingSplash />;
  if (!user) {
    const seen = localStorage.getItem('parkhub_welcome_seen');
    return <Navigate to={seen ? '/login' : '/welcome'} replace />;
  }
  return <>{children}</>;
}

function AdminRoute({ children }: { children: React.ReactNode }) {
  const { user } = useAuth();
  if (!user || !['admin', 'superadmin'].includes(user.role)) return <Navigate to="/" replace />;
  return <>{children}</>;
}

function LoadingSplash() {
  return (
    <div className="min-h-dvh flex items-center justify-center mesh-gradient" role="status" aria-label="Loading ParkHub">
      <div className="flex flex-col items-center gap-4">
        <div className="w-16 h-16 rounded-xl bg-primary-600 flex items-center justify-center">
          <span className="text-2xl font-black text-white tracking-tight">P</span>
        </div>
        <div className="w-8 h-8 border-2 border-primary-500 border-t-transparent rounded-full animate-spin" aria-hidden="true" />
      </div>
    </div>
  );
}

function useThemeLoader() {
  useEffect(() => {
    fetch('/api/v1/theme', { credentials: 'include' })
      .then(r => { if (!r.ok) return null; return r.json(); })
      .then(res => {
        if (!res) return;
        const key = res?.data?.use_case?.key;
        if (key) document.documentElement.dataset.usecase = key;
      })
      .catch(() => {});
    loadTranslationOverrides();
  }, []);
}

function SuspenseRoute({ children }: { children: React.ReactNode }) {
  return <Suspense fallback={<LoadingSplash />}>{children}</Suspense>;
}

function ViewTransitionRoutes() {
  const location = useLocation();
  const prevPath = useRef(location.pathname);

  useEffect(() => {
    if (prevPath.current === location.pathname) return;
    prevPath.current = location.pathname;

    if ('startViewTransition' in document) {
      (document as any).startViewTransition(() => {
        startTransition(() => {});
      });
    }
  }, [location.pathname]);

  return (
    <Routes location={location}>
      <Route path="/welcome" element={<SuspenseRoute><WelcomePage /></SuspenseRoute>} />
      <Route path="/login" element={<SuspenseRoute><LoginPage /></SuspenseRoute>} />
      <Route path="/register" element={<SuspenseRoute><RegisterPage /></SuspenseRoute>} />
      <Route path="/forgot-password" element={<SuspenseRoute><ForgotPasswordPage /></SuspenseRoute>} />
      <Route path="/choose" element={<SuspenseRoute><UseCaseSelectorPage /></SuspenseRoute>} />
      <Route path="/lobby/:lotId" element={<SuspenseRoute><LobbyDisplayPage /></SuspenseRoute>} />
      <Route path="/setup" element={<SuspenseRoute><SetupWizardPage /></SuspenseRoute>} />
      <Route path="/" element={<ProtectedRoute><Layout /></ProtectedRoute>}>
        <Route index element={<SuspenseRoute><DashboardPage /></SuspenseRoute>} />
        <Route path="book" element={<SuspenseRoute><BookPage /></SuspenseRoute>} />
        <Route path="bookings" element={<SuspenseRoute><BookingsPage /></SuspenseRoute>} />
        <Route path="credits" element={<SuspenseRoute><CreditsPage /></SuspenseRoute>} />
        <Route path="vehicles" element={<SuspenseRoute><VehiclesPage /></SuspenseRoute>} />
        <Route path="favorites" element={<SuspenseRoute><FavoritesPage /></SuspenseRoute>} />
        <Route path="absences" element={<SuspenseRoute><AbsencesPage /></SuspenseRoute>} />
        <Route path="profile" element={<SuspenseRoute><ProfilePage /></SuspenseRoute>} />
        <Route path="team" element={<SuspenseRoute><TeamPage /></SuspenseRoute>} />
        <Route path="notifications" element={<SuspenseRoute><NotificationsPage /></SuspenseRoute>} />
        <Route path="calendar" element={<SuspenseRoute><CalendarPage /></SuspenseRoute>} />
        <Route path="visitors" element={<SuspenseRoute><VisitorsPage /></SuspenseRoute>} />
        <Route path="ev-charging" element={<SuspenseRoute><EVChargingPage /></SuspenseRoute>} />
        <Route path="history" element={<SuspenseRoute><ParkingHistoryPage /></SuspenseRoute>} />
        <Route path="absence-approval" element={<SuspenseRoute><AbsenceApprovalPage /></SuspenseRoute>} />
        <Route path="map" element={<SuspenseRoute><MapViewPage /></SuspenseRoute>} />
        <Route path="swap-requests" element={<SuspenseRoute><SwapRequestsPage /></SuspenseRoute>} />
        <Route path="checkin" element={<SuspenseRoute><QRCheckInPage /></SuspenseRoute>} />
        <Route path="guest-pass" element={<SuspenseRoute><GuestPassPage /></SuspenseRoute>} />
        <Route path="leaderboard" element={<SuspenseRoute><TeamLeaderboardPage /></SuspenseRoute>} />
        <Route path="predict" element={<SuspenseRoute><OccupancyPredictionPage /></SuspenseRoute>} />
        <Route path="translations" element={<SuspenseRoute><TranslationsPage /></SuspenseRoute>} />
        <Route path="admin" element={<AdminRoute><SuspenseRoute><AdminPage /></SuspenseRoute></AdminRoute>}>
          <Route index element={<SuspenseRoute><AdminReportsPage /></SuspenseRoute>} />
          <Route path="settings" element={<SuspenseRoute><AdminSettingsPage /></SuspenseRoute>} />
          <Route path="users" element={<SuspenseRoute><AdminUsersPage /></SuspenseRoute>} />
          <Route path="lots" element={<SuspenseRoute><AdminLotsPage /></SuspenseRoute>} />
          <Route path="announcements" element={<SuspenseRoute><AdminAnnouncementsPage /></SuspenseRoute>} />
          <Route path="reports" element={<SuspenseRoute><AdminReportsPage /></SuspenseRoute>} />
          <Route path="translations" element={<SuspenseRoute><AdminTranslationsPage /></SuspenseRoute>} />
          <Route path="analytics" element={<SuspenseRoute><AdminAnalyticsPage /></SuspenseRoute>} />
          <Route path="rate-limits" element={<SuspenseRoute><AdminRateLimitsPage /></SuspenseRoute>} />
          <Route path="tenants" element={<SuspenseRoute><AdminTenantsPage /></SuspenseRoute>} />
          <Route path="audit-log" element={<SuspenseRoute><AdminAuditLogPage /></SuspenseRoute>} />
          <Route path="data" element={<SuspenseRoute><AdminDataManagementPage /></SuspenseRoute>} />
          <Route path="fleet" element={<SuspenseRoute><AdminFleetPage /></SuspenseRoute>} />
          <Route path="accessible" element={<SuspenseRoute><AdminAccessiblePage /></SuspenseRoute>} />
          <Route path="maintenance" element={<SuspenseRoute><AdminMaintenancePage /></SuspenseRoute>} />
          <Route path="billing" element={<SuspenseRoute><AdminBillingPage /></SuspenseRoute>} />
          <Route path="visitors" element={<SuspenseRoute><AdminVisitorsPage /></SuspenseRoute>} />
          <Route path="chargers" element={<SuspenseRoute><AdminChargersPage /></SuspenseRoute>} />
          <Route path="widgets" element={<SuspenseRoute><AdminDashboardPage /></SuspenseRoute>} />
          <Route path="plugins" element={<SuspenseRoute><AdminPluginsPage /></SuspenseRoute>} />
          <Route path="compliance" element={<SuspenseRoute><AdminCompliancePage /></SuspenseRoute>} />
          <Route path="sso" element={<SuspenseRoute><AdminSSOPage /></SuspenseRoute>} />
          <Route path="webhooks" element={<SuspenseRoute><AdminWebhooksPage /></SuspenseRoute>} />
          <Route path="roles" element={<SuspenseRoute><AdminRolesPage /></SuspenseRoute>} />
          <Route path="zones" element={<SuspenseRoute><AdminZonesPage /></SuspenseRoute>} />
          <Route path="updates" element={<SuspenseRoute><AdminUpdatesPage /></SuspenseRoute>} />
          <Route path="heatmap" element={<SuspenseRoute><OccupancyHeatmapPage /></SuspenseRoute>} />
          <Route path="scheduled-reports" element={<SuspenseRoute><AdminScheduledReportsPage /></SuspenseRoute>} />
        </Route>
      </Route>
      <Route path="*" element={<SuspenseRoute><NotFoundPage /></SuspenseRoute>} />
    </Routes>
  );
}

function AppRoutes() {
  useEffect(() => {
    preloadRoutesIdle(['/', '/bookings', '/book', '/profile', '/admin']);
  }, []);

  return <ViewTransitionRoutes />;
}

function ThemeLoader({ children }: { children: React.ReactNode }) {
  useThemeLoader();
  return <>{children}</>;
}

export function App() {
  return (
    <ErrorBoundary>
    <BrowserRouter>
      <ThemeProvider>
        <ThemeLoader>
        <UseCaseProvider>
        <FeaturesProvider>
        <AuthProvider>
          <AppRoutes />
          <Suspense fallback={null}><DemoOverlay /></Suspense>
          <Suspense fallback={null}><InstallPrompt /></Suspense>
          <Suspense fallback={null}><SWUpdatePrompt /></Suspense>
          <Toaster
            position="bottom-center"
            toastOptions={{
              className: '!bg-surface-800 !text-white !rounded-xl !shadow-lg !text-sm !font-medium',
              duration: 3000,
              ariaProps: { role: 'status', 'aria-live': 'polite' },
            }}
          />
        </AuthProvider>
        </FeaturesProvider>
        </UseCaseProvider>
        </ThemeLoader>
      </ThemeProvider>
    </BrowserRouter>
    </ErrorBoundary>
  );
}
