import { createContext, useContext, useState, useEffect, ReactNode } from 'react';
import { api, User } from '../api/client';

interface AuthContextType {
  user: User | null;
  isLoading: boolean;
  isAuthenticated: boolean;
  login: (username: string, password: string) => Promise<boolean>;
  logout: () => void;
  register: (data: { username: string; email: string; password: string; name: string }) => Promise<boolean>;
}

const AuthContext = createContext<AuthContextType | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    // Check for existing token on mount
    const token = api.getToken();
    if (token) {
      loadUser();
    } else {
      setIsLoading(false);
    }
  }, []);

  async function loadUser() {
    try {
      const response = await api.getCurrentUser();
      if (response.success && response.data) {
        setUser(response.data);
      } else {
        api.setToken(null);
      }
    } catch {
      api.setToken(null);
    } finally {
      setIsLoading(false);
    }
  }

  async function login(username: string, password: string): Promise<boolean> {
    const response = await api.login(username, password);
    if (response.success && response.data) {
      api.setToken(response.data.tokens.access_token);
      localStorage.setItem('parkhub_refresh_token', response.data.tokens.refresh_token);
      setUser(response.data.user);
      return true;
    }
    return false;
  }

  function logout() {
    api.setToken(null);
    localStorage.removeItem('parkhub_refresh_token');
    setUser(null);
  }

  async function register(data: { username: string; email: string; password: string; name: string }): Promise<boolean> {
    const response = await api.register(data);
    if (response.success && response.data) {
      api.setToken(response.data.tokens.access_token);
      localStorage.setItem('parkhub_refresh_token', response.data.tokens.refresh_token);
      setUser(response.data.user);
      return true;
    }
    return false;
  }

  return (
    <AuthContext.Provider
      value={{
        user,
        isLoading,
        isAuthenticated: !!user,
        login,
        logout,
        register,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error('useAuth must be used within an AuthProvider');
  }
  return context;
}
