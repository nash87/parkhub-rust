import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { api, type User, setInMemoryToken } from '../api/client';

interface AuthState {
  user: User | null;
  loading: boolean;
  login: (username: string, password: string) => Promise<{ success: boolean; error?: string }>;
  logout: () => void;
  refreshUser: () => Promise<void>;
}

const AuthContext = createContext<AuthState | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // On page load, try to authenticate via the httpOnly cookie.
    // The cookie is sent automatically with credentials: 'include',
    // so we just call /api/v1/users/me and see if it works.
    api.me().then(res => {
      if (res.success && res.data) setUser(res.data);
    }).finally(() => setLoading(false));
  }, []);

  // When any API call receives a 401 the client dispatches this event.
  // Clear the user so ProtectedRoute can redirect via React Router
  // instead of a hard page reload (which caused an infinite loop).
  useEffect(() => {
    const onUnauthorized = () => setUser(null);
    window.addEventListener('auth:unauthorized', onUnauthorized);
    return () => window.removeEventListener('auth:unauthorized', onUnauthorized);
  }, []);

  async function login(username: string, password: string) {
    const res = await api.login(username, password);
    if (res.success && res.data?.tokens?.access_token) {
      // Store token in memory as fallback (not localStorage -- XSS safe).
      // The httpOnly cookie is the primary auth mechanism for the browser.
      setInMemoryToken(res.data.tokens.access_token);
      const me = await api.me();
      if (me.success && me.data) {
        setUser(me.data);
        return { success: true };
      }
    }
    return { success: false, error: res.error?.message || 'Login failed' };
  }

  async function logout() {
    // Call the server to clear the cookie and invalidate the session
    await api.logout();
    setInMemoryToken(null);
    setUser(null);
  }

  async function refreshUser() {
    const res = await api.me();
    if (res.success && res.data) setUser(res.data);
  }

  return (
    <AuthContext.Provider value={{ user, loading, login, logout, refreshUser }}>
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('useAuth must be used within AuthProvider');
  return ctx;
}
