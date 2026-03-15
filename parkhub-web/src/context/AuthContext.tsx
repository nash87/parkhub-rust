import { createContext, useContext, useEffect, useState, type ReactNode } from 'react';
import { api, type User } from '../api/client';

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
    const token = localStorage.getItem('parkhub_token');
    if (token) {
      api.me().then(res => {
        if (res.success && res.data) setUser(res.data);
        else localStorage.removeItem('parkhub_token');
      }).finally(() => setLoading(false));
    } else {
      setLoading(false);
    }
  }, []);

  async function login(username: string, password: string) {
    const res = await api.login(username, password);
    if (res.success && res.data?.tokens?.access_token) {
      localStorage.setItem('parkhub_token', res.data.tokens.access_token);
      const me = await api.me();
      if (me.success && me.data) {
        setUser(me.data);
        return { success: true };
      }
    }
    return { success: false, error: res.error?.message || 'Login failed' };
  }

  function logout() {
    localStorage.removeItem('parkhub_token');
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
