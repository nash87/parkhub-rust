import React, { useEffect, Suspense } from 'react';
import { BrowserRouter, Routes, Route, Navigate, useLocation } from 'react-router-dom';
import { Toaster } from 'react-hot-toast';
import { AnimatePresence } from 'framer-motion';
import { AuthProvider, useAuth } from './context/AuthContext';
import { ThemeProvider } from './context/ThemeContext';
import { UseCaseProvider } from './context/UseCaseContext';
import { FeaturesProvider } from './context/FeaturesContext';
import './i18n';
import { loadTranslationOverrides } from './i18n';

// Eagerly loaded shell
import { Layout } from './components/Layout';
import { ErrorBoundary } from './components/ErrorBoundary';

// Lazy helper — wraps named exports for React.lazy
const lazy = <T extends Record<string, React.ComponentType>>(
  loader: () => Promise<T>,
  name: keyof T,
) => React.lazy(() => loader().then(m => ({ default: m[name] as React.ComponentType })));

// Auth pages (small, on critical path for unauthenticated users)
const WelcomePage = lazy(() => import('./views/Welcome'), 'WelcomePage');
const LoginPage = lazy(() => import('./views/Login'), 'LoginPage');
const RegisterPage = lazy(() => import('./views/Register'), 'RegisterPage');
const ForgotPasswordPage = lazy(() => import('./views/ForgotPassword'), 'ForgotPasswordPage');
const UseCaseSelectorPage = lazy(() => import('./views/UseCaseSelector'), 'UseCaseSelectorPage');
const NotFoundPage = lazy(() => import('./views/NotFound'), 'NotFoundPage');
const LobbyDisplayPage = lazy(() => import('./views/LobbyDisplay'), 'LobbyDisplayPage');
const SetupWizardPage = lazy(() => import('./views/SetupWizard'), 'SetupWizardPage');

// Main app pages
const DashboardPage = lazy(() => import('./views/Dashboard'), 'DashboardPage');
const BookPage = lazy(() => import('./views/Book'), 'BookPage');
const BookingsPage = lazy(() => import('./views/Bookings'), 'BookingsPage');
const CreditsPage = lazy(() => import('./views/Credits'), 'CreditsPage');
const VehiclesPage = lazy(() => import('./views/Vehicles'), 'VehiclesPage');
const AbsencesPage = lazy(() => import('./views/Absences'), 'AbsencesPage');
const ProfilePage = lazy(() => import('./views/Profile'), 'ProfilePage');
const TeamPage = lazy(() => import('./views/Team'), 'TeamPage');
const NotificationsPage = lazy(() => import('./views/Notifications'), 'NotificationsPage');
const CalendarPage = lazy(() => import('./views/Calendar'), 'CalendarPage');
const DemoOverlay = lazy(() => import('./components/DemoOverlay'), 'DemoOverlay');
const InstallPrompt = lazy(() => import('./components/InstallPrompt'), 'InstallPrompt');

// Admin pages
const MapViewPage = lazy(() => import('./views/MapView'), 'MapViewPage');
const AdminPage = lazy(() => import('./views/Admin'), 'AdminPage');
const AdminSettingsPage = lazy(() => import('./views/AdminSettings'), 'AdminSettingsPage');
const AdminUsersPage = lazy(() => import('./views/AdminUsers'), 'AdminUsersPage');
const AdminAnnouncementsPage = lazy(() => import('./views/AdminAnnouncements'), 'AdminAnnouncementsPage');
const AdminLotsPage = lazy(() => import('./views/AdminLots'), 'AdminLotsPage');
const AdminReportsPage = lazy(() => import('./views/AdminReports'), 'AdminReportsPage');
const FavoritesPage = lazy(() => import('./views/Favorites'), 'FavoritesPage');
const TranslationsPage = lazy(() => import('./views/Translations'), 'TranslationsPage');
const AdminTranslationsPage = lazy(() => import('./views/AdminTranslations'), 'AdminTranslationsPage');
const AdminAnalyticsPage = lazy(() => import('./views/AdminAnalytics'), 'AdminAnalyticsPage');
const AdminRateLimitsPage = lazy(() => import('./views/AdminRateLimits'), 'AdminRateLimitsPage');
const AdminTenantsPage = lazy(() => import('./views/AdminTenants'), 'AdminTenantsPage');
const AdminAuditLogPage = lazy(() => import('./views/AdminAuditLog'), 'AdminAuditLogPage');
const AdminDataManagementPage = lazy(() => import('./views/AdminDataManagement'), 'AdminDataManagementPage');
const AdminFleetPage = lazy(() => import('./views/AdminFleet'), 'AdminFleetPage');
const AdminAccessiblePage = lazy(() => import('./views/AdminAccessible'), 'AdminAccessiblePage');
const AdminMaintenancePage = lazy(() => import('./views/AdminMaintenance'), 'AdminMaintenancePage');
const AdminBillingPage = lazy(() => import('./views/AdminBilling'), 'AdminBillingPage');
const VisitorsPage = lazy(() => import('./views/Visitors'), 'VisitorsPage');
const AdminVisitorsPage = lazy(() => import('./views/Visitors'), 'AdminVisitorsPage');
const EVChargingPage = lazy(() => import('./views/EVCharging'), 'EVChargingPage');
const AdminChargersPage = lazy(() => import('./views/EVCharging'), 'AdminChargersPage');
const ParkingHistoryPage = lazy(() => import('./views/ParkingHistory'), 'ParkingHistoryPage');
const AbsenceApprovalPage = lazy(() => import('./views/AbsenceApproval'), 'AbsenceApprovalPage');
const AdminDashboardPage = lazy(() => import('./views/AdminDashboard'), 'AdminDashboardPage');
const AdminPluginsPage = lazy(() => import('./views/AdminPlugins'), 'AdminPluginsPage');
const AdminCompliancePage = lazy(() => import('./views/AdminCompliance'), 'AdminCompliancePage');
const AdminSSOPage = lazy(() => import('./views/AdminSSO'), 'AdminSSOPage');

function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { user, loading } = useAuth();
  if (loading) return <LoadingSplash />;
  if (!user) {
    // First-time visitors see the welcome/language screen
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

/** Fetch /api/v1/theme on mount and apply use-case CSS theme + load translation overrides */
function useThemeLoader() {
  useEffect(() => {
    fetch('/api/v1/theme')
      .then(r => r.json())
      .then(res => {
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

function AnimatedRoutes() {
  const location = useLocation();

  return (
    <AnimatePresence mode="wait">
      <Routes location={location} key={location.pathname}>
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
          </Route>
        </Route>
        <Route path="*" element={<SuspenseRoute><NotFoundPage /></SuspenseRoute>} />
      </Routes>
    </AnimatePresence>
  );
}

function AppRoutes() {
  return <AnimatedRoutes />;
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
