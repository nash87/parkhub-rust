import { useEffect } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { Toaster } from 'react-hot-toast';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { AuthProvider, useAuth } from './context/AuthContext';
import { useTheme, applyTheme } from './stores/theme';
import { Layout } from './components/Layout';
import { LoginPage } from './pages/Login';
import { RegisterPage } from './pages/Register';
import { DashboardPage } from './pages/Dashboard';
import { BookPage } from './pages/Book';
import { BookingsPage } from './pages/Bookings';
import { VehiclesPage } from './pages/Vehicles';
import { AdminPage } from './pages/Admin';
import { SpinnerGap } from '@phosphor-icons/react';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60, // 1 minute
      retry: 1,
    },
  },
});

function LoadingScreen() {
  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-950">
      <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" />
    </div>
  );
}

function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { isAuthenticated, isLoading } = useAuth();

  if (isLoading) return <LoadingScreen />;
  if (!isAuthenticated) return <Navigate to="/login" replace />;

  return <Layout>{children}</Layout>;
}

function AdminRoute({ children }: { children: React.ReactNode }) {
  const { user, isAuthenticated, isLoading } = useAuth();

  if (isLoading) return <LoadingScreen />;
  if (!isAuthenticated) return <Navigate to="/login" replace />;
  if (user?.role !== 'admin' && user?.role !== 'superadmin') {
    return <Navigate to="/" replace />;
  }

  return <Layout>{children}</Layout>;
}

function ThemeInitializer({ children }: { children: React.ReactNode }) {
  const { isDark } = useTheme();

  useEffect(() => {
    applyTheme(isDark);
  }, [isDark]);

  return <>{children}</>;
}

function AppRoutes() {
  return (
    <Routes>
      {/* Public */}
      <Route path="/login" element={<LoginPage />} />
      <Route path="/register" element={<RegisterPage />} />

      {/* Protected */}
      <Route path="/" element={<ProtectedRoute><DashboardPage /></ProtectedRoute>} />
      <Route path="/book" element={<ProtectedRoute><BookPage /></ProtectedRoute>} />
      <Route path="/bookings" element={<ProtectedRoute><BookingsPage /></ProtectedRoute>} />
      <Route path="/vehicles" element={<ProtectedRoute><VehiclesPage /></ProtectedRoute>} />

      {/* Admin */}
      <Route path="/admin/*" element={<AdminRoute><AdminPage /></AdminRoute>} />

      {/* Catch all */}
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <ThemeInitializer>
          <AuthProvider>
            <AppRoutes />
            <Toaster
              position="top-right"
              toastOptions={{
                duration: 4000,
                style: {
                  background: 'var(--toast-bg, #fff)',
                  color: 'var(--toast-color, #1f2937)',
                  borderRadius: '12px',
                  boxShadow: '0 10px 40px -10px rgba(0, 0, 0, 0.2)',
                },
                success: {
                  iconTheme: {
                    primary: '#22c55e',
                    secondary: '#fff',
                  },
                },
                error: {
                  iconTheme: {
                    primary: '#ef4444',
                    secondary: '#fff',
                  },
                },
              }}
            />
          </AuthProvider>
        </ThemeInitializer>
      </BrowserRouter>
    </QueryClientProvider>
  );
}

export default App;
