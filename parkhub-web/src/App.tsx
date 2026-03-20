import React, { useEffect, Suspense } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { Toaster } from 'react-hot-toast';
import { AuthProvider, useAuth } from './context/AuthContext';
import { ThemeProvider } from './context/ThemeContext';
import { UseCaseProvider } from './context/UseCaseContext';
import { FeaturesProvider } from './context/FeaturesContext';
import './i18n';

// Pages
import { WelcomePage } from './views/Welcome';
import { LoginPage } from './views/Login';
import { RegisterPage } from './views/Register';
import { ForgotPasswordPage } from './views/ForgotPassword';
import { NotFoundPage } from './views/NotFound';
import { UseCaseSelectorPage } from './views/UseCaseSelector';
import { DashboardPage } from './views/Dashboard';
import { BookingsPage } from './views/Bookings';
import { CreditsPage } from './views/Credits';
import { AdminPage } from './views/Admin';
const AdminSettingsPage = React.lazy(() => import('./views/AdminSettings').then(m => ({ default: m.AdminSettingsPage })));
const AdminUsersPage = React.lazy(() => import('./views/AdminUsers').then(m => ({ default: m.AdminUsersPage })));
const AdminAnnouncementsPage = React.lazy(() => import('./views/AdminAnnouncements').then(m => ({ default: m.AdminAnnouncementsPage })));
const AdminLotsPage = React.lazy(() => import('./views/AdminLots').then(m => ({ default: m.AdminLotsPage })));
const AdminReportsPage = React.lazy(() => import('./views/AdminReports').then(m => ({ default: m.AdminReportsPage })));
import { VehiclesPage } from './views/Vehicles';
import { AbsencesPage } from './views/Absences';
import { ProfilePage } from './views/Profile';
import { TeamPage } from './views/Team';
import { NotificationsPage } from './views/Notifications';
import { CalendarPage } from './views/Calendar';
import { BookPage } from './views/Book';
import { Layout } from './components/Layout';
import { DemoOverlay } from './components/DemoOverlay';
import { ErrorBoundary } from './components/ErrorBoundary';

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
    <div className="min-h-dvh flex items-center justify-center mesh-gradient">
      <div className="flex flex-col items-center gap-4">
        <div className="w-16 h-16 rounded-xl bg-primary-600 flex items-center justify-center">
          <span className="text-2xl font-black text-white tracking-tight">P</span>
        </div>
        <div className="w-8 h-8 border-2 border-primary-500 border-t-transparent rounded-full animate-spin" />
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

function AppRoutes() {
  return (
    <Routes>
      <Route path="/welcome" element={<WelcomePage />} />
      <Route path="/login" element={<LoginPage />} />
      <Route path="/register" element={<RegisterPage />} />
      <Route path="/forgot-password" element={<ForgotPasswordPage />} />
      <Route path="/choose" element={<UseCaseSelectorPage />} />
      <Route path="/" element={<ProtectedRoute><Layout /></ProtectedRoute>}>
        <Route index element={<DashboardPage />} />
        <Route path="book" element={<BookPage />} />
        <Route path="bookings" element={<BookingsPage />} />
        <Route path="credits" element={<CreditsPage />} />
        <Route path="vehicles" element={<VehiclesPage />} />
        <Route path="absences" element={<AbsencesPage />} />
        <Route path="profile" element={<ProfilePage />} />
        <Route path="team" element={<TeamPage />} />
        <Route path="notifications" element={<NotificationsPage />} />
        <Route path="calendar" element={<CalendarPage />} />
        <Route path="admin" element={<AdminRoute><Suspense fallback={<LoadingSplash />}><AdminPage /></Suspense></AdminRoute>}>
          <Route index element={<Suspense fallback={<LoadingSplash />}><AdminReportsPage /></Suspense>} />
          <Route path="settings" element={<Suspense fallback={<LoadingSplash />}><AdminSettingsPage /></Suspense>} />
          <Route path="users" element={<Suspense fallback={<LoadingSplash />}><AdminUsersPage /></Suspense>} />
          <Route path="lots" element={<Suspense fallback={<LoadingSplash />}><AdminLotsPage /></Suspense>} />
          <Route path="announcements" element={<Suspense fallback={<LoadingSplash />}><AdminAnnouncementsPage /></Suspense>} />
          <Route path="reports" element={<Suspense fallback={<LoadingSplash />}><AdminReportsPage /></Suspense>} />
        </Route>
      </Route>
      <Route path="*" element={<NotFoundPage />} />
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
          <DemoOverlay />
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
