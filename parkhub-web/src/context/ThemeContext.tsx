import { createContext, useContext, useEffect, useState, useSyncExternalStore, type ReactNode } from 'react';

type Theme = 'light' | 'dark' | 'system';

interface ThemeState {
  theme: Theme;
  resolved: 'light' | 'dark';
  setTheme: (t: Theme) => void;
}

const ThemeContext = createContext<ThemeState | null>(null);

/* Subscribe to prefers-color-scheme changes via useSyncExternalStore
   so React re-renders when the OS theme flips while in "system" mode. */
const mq = typeof window !== 'undefined'
  ? window.matchMedia('(prefers-color-scheme: dark)')
  : null;

function subscribeToSystemTheme(callback: () => void) {
  mq?.addEventListener('change', callback);
  return () => mq?.removeEventListener('change', callback);
}

function getSystemSnapshot(): 'light' | 'dark' {
  return mq?.matches ? 'dark' : 'light';
}

function getServerSnapshot(): 'light' | 'dark' {
  return 'light';
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<Theme>(() =>
    (localStorage.getItem('parkhub_theme') as Theme) || 'system'
  );

  // Reactively track OS preference so `resolved` updates on system change
  const systemTheme = useSyncExternalStore(
    subscribeToSystemTheme,
    getSystemSnapshot,
    getServerSnapshot,
  );

  const resolved = theme === 'system' ? systemTheme : theme;

  useEffect(() => {
    const root = document.documentElement;
    root.classList.toggle('dark', resolved === 'dark');
    // Update meta theme-color for mobile browsers
    const metaLight = document.querySelector('meta[name="theme-color"][media="(prefers-color-scheme: light)"]');
    const metaDark = document.querySelector('meta[name="theme-color"][media="(prefers-color-scheme: dark)"]');
    if (resolved === 'dark') {
      metaLight?.setAttribute('content', '#042f2e');
      metaDark?.setAttribute('content', '#042f2e');
    } else {
      metaLight?.setAttribute('content', '#0d9488');
      metaDark?.setAttribute('content', '#0d9488');
    }
  }, [resolved]);

  function setTheme(t: Theme) {
    setThemeState(t);
    localStorage.setItem('parkhub_theme', t);
  }

  return (
    <ThemeContext.Provider value={{ theme, resolved, setTheme }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error('useTheme must be used within ThemeProvider');
  return ctx;
}
