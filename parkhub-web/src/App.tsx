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
import { DashboardPage } from './views/Dashboard';
import { BookingsPage } from './views/Bookings';
import { CreditsPage } from './views/Credits';
import { UseCaseSelectorPage } from './views/UseCaseSelector';
import { AdminFeaturesPage } from './views/AdminFeatures';
import { Layout } from './components/Layout';
import { DemoOverlay } from './components/DemoOverlay';
import { QuickActionsFab } from './components/QuickActionsFab';

function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { user, loading } = useAuth();
  if (loading) return <LoadingSplash />;
  if (!user) return <Navigate to="/login" replace />;
  return <>{children}</>;
}

function AdminRoute({ children }: { children: React.ReactNode }) {
  const { user } = useAuth();
  if (!user || !['admin', 'superadmin'].includes(user.role)) return <Navigate to="/" replace />;
  return <>{children}</>;
}

function LoadingSplash() {
  return (
    <div className="min-h-dvh flex items-center justify-center parking-grid">
      <div className="flex flex-col items-center gap-4">
        <div className="w-16 h-16 rounded-2xl bg-primary-800 dark:bg-surface-800 flex items-center justify-center shadow-lg border border-primary-700 dark:border-surface-700">
          <span className="text-2xl font-black text-accent-500 tracking-tight font-[Outfit]">P</span>
        </div>
        <div className="w-8 h-8 border-2 border-accent-500 border-t-transparent rounded-full animate-spin" />
      </div>
    </div>
  );
}

function AppRoutes() {
  return (
    <Routes>
      <Route path="/setup" element={<UseCaseSelectorPage />} />
      <Route path="/welcome" element={<WelcomePage />} />
      <Route path="/login" element={<LoginPage />} />
      <Route path="/register" element={<RegisterPage />} />
      <Route path="/" element={<ProtectedRoute><Layout /></ProtectedRoute>}>
        <Route index element={<DashboardPage />} />
        <Route path="bookings" element={<BookingsPage />} />
        <Route path="credits" element={<CreditsPage />} />
        <Route path="admin/features" element={<AdminRoute><AdminFeaturesPage /></AdminRoute>} />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

export function App() {
  return (
    <BrowserRouter>
      <ThemeProvider>
        <UseCaseProvider>
        <FeaturesProvider>
        <AuthProvider>
          <AppRoutes />
          <QuickActionsFab />
          <DemoOverlay />
          <Toaster
            position="bottom-center"
            toastOptions={{
              className: '!bg-surface-800 !text-white !rounded-xl !shadow-lg !text-sm !font-medium',
              duration: 3000,
            }}
          />
        </AuthProvider>
        </FeaturesProvider>
        </UseCaseProvider>
      </ThemeProvider>
    </BrowserRouter>
  );
}
