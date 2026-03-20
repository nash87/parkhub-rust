import React, { useEffect, Suspense } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { Toaster } from 'react-hot-toast';
import { AuthProvider, useAuth } from './context/AuthContext';
import { ThemeProvider } from './context/ThemeContext';
import { UseCaseProvider } from './context/UseCaseContext';
import { FeaturesProvider } from './context/FeaturesContext';
import './i18n';

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
const AdminPage = lazy(() => import('./views/Admin'), 'AdminPage');
const AdminSettingsPage = lazy(() => import('./views/AdminSettings'), 'AdminSettingsPage');
const AdminUsersPage = lazy(() => import('./views/AdminUsers'), 'AdminUsersPage');
const AdminAnnouncementsPage = lazy(() => import('./views/AdminAnnouncements'), 'AdminAnnouncementsPage');
const AdminLotsPage = lazy(() => import('./views/AdminLots'), 'AdminLotsPage');
const AdminReportsPage = lazy(() => import('./views/AdminReports'), 'AdminReportsPage');

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

/** Fetch /api/v1/theme on mount and apply use-case CSS theme */
function useThemeLoader() {
  useEffect(() => {
    fetch('/api/v1/theme')
      .then(r => r.json())
      .then(res => {
        const key = res?.data?.use_case?.key;
        if (key) document.documentElement.dataset.usecase = key;
      })
      .catch(() => {});
  }, []);
}

function SuspenseRoute({ children }: { children: React.ReactNode }) {
  return <Suspense fallback={<LoadingSplash />}>{children}</Suspense>;
}

function AppRoutes() {
  return (
    <Routes>
      <Route path="/welcome" element={<SuspenseRoute><WelcomePage /></SuspenseRoute>} />
      <Route path="/login" element={<SuspenseRoute><LoginPage /></SuspenseRoute>} />
      <Route path="/register" element={<SuspenseRoute><RegisterPage /></SuspenseRoute>} />
      <Route path="/forgot-password" element={<SuspenseRoute><ForgotPasswordPage /></SuspenseRoute>} />
      <Route path="/choose" element={<SuspenseRoute><UseCaseSelectorPage /></SuspenseRoute>} />
      <Route path="/" element={<ProtectedRoute><Layout /></ProtectedRoute>}>
        <Route index element={<SuspenseRoute><DashboardPage /></SuspenseRoute>} />
        <Route path="book" element={<SuspenseRoute><BookPage /></SuspenseRoute>} />
        <Route path="bookings" element={<SuspenseRoute><BookingsPage /></SuspenseRoute>} />
        <Route path="credits" element={<SuspenseRoute><CreditsPage /></SuspenseRoute>} />
        <Route path="vehicles" element={<SuspenseRoute><VehiclesPage /></SuspenseRoute>} />
        <Route path="absences" element={<SuspenseRoute><AbsencesPage /></SuspenseRoute>} />
        <Route path="profile" element={<SuspenseRoute><ProfilePage /></SuspenseRoute>} />
        <Route path="team" element={<SuspenseRoute><TeamPage /></SuspenseRoute>} />
        <Route path="notifications" element={<SuspenseRoute><NotificationsPage /></SuspenseRoute>} />
        <Route path="calendar" element={<SuspenseRoute><CalendarPage /></SuspenseRoute>} />
        <Route path="admin" element={<AdminRoute><SuspenseRoute><AdminPage /></SuspenseRoute></AdminRoute>}>
          <Route index element={<SuspenseRoute><AdminReportsPage /></SuspenseRoute>} />
          <Route path="settings" element={<SuspenseRoute><AdminSettingsPage /></SuspenseRoute>} />
          <Route path="users" element={<SuspenseRoute><AdminUsersPage /></SuspenseRoute>} />
          <Route path="lots" element={<SuspenseRoute><AdminLotsPage /></SuspenseRoute>} />
          <Route path="announcements" element={<SuspenseRoute><AdminAnnouncementsPage /></SuspenseRoute>} />
          <Route path="reports" element={<SuspenseRoute><AdminReportsPage /></SuspenseRoute>} />
        </Route>
      </Route>
      <Route path="*" element={<SuspenseRoute><NotFoundPage /></SuspenseRoute>} />
    </Routes>
  );
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
