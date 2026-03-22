import { createContext, useContext, useEffect, useState, useCallback, useSyncExternalStore, type ReactNode } from 'react';
import { getInMemoryToken } from '../api/client';

// ── Light/Dark Mode ──

type ColorMode = 'light' | 'dark' | 'system';

// ── Design Themes ──

export type DesignThemeId = 'classic' | 'glass' | 'bento' | 'brutalist' | 'neon' | 'warm' | 'liquid' | 'mono' | 'ocean' | 'forest';

export interface DesignThemeInfo {
  id: DesignThemeId;
  name: string;
  description: string;
  /** Preview palette colors [bg, card, accent, text, border] */
  previewColors: {
    light: [string, string, string, string, string];
    dark: [string, string, string, string, string];
  };
  tags: string[];
}

export const DESIGN_THEMES: DesignThemeInfo[] = [
  {
    id: 'classic',
    name: 'Classic',
    description: 'Clean, professional. The original ParkHub look.',
    previewColors: {
      light: ['#fafaf9', '#ffffff', '#0d9488', '#1e293b', '#e2e8f0'],
      dark: ['#0f172a', '#1e293b', '#14b8a6', '#f1f5f9', '#334155'],
    },
    tags: ['clean', 'professional', 'default'],
  },
  {
    id: 'glass',
    name: 'Glass',
    description: 'Frosted glassmorphism with blur and translucency.',
    previewColors: {
      light: ['#f8fafc', 'rgba(255,255,255,0.6)', '#14b8a6', '#0f172a', 'rgba(255,255,255,0.3)'],
      dark: ['#0c0a1a', 'rgba(255,255,255,0.05)', '#6366f1', '#e2e8f0', 'rgba(255,255,255,0.06)'],
    },
    tags: ['modern', 'glassmorphism', 'blur'],
  },
  {
    id: 'bento',
    name: 'Bento',
    description: 'Grid-focused, minimal. Japanese-inspired clean lines.',
    previewColors: {
      light: ['#fafaf9', '#ffffff', '#0d9488', '#1c1917', '#e7e5e4'],
      dark: ['#0c0a09', '#1c1917', '#14b8a6', '#fafaf9', '#292524'],
    },
    tags: ['minimal', 'grid', 'japanese'],
  },
  {
    id: 'brutalist',
    name: 'Brutalist',
    description: 'Raw, bold, high-contrast. No rounded corners.',
    previewColors: {
      light: ['#ffffff', '#ffffff', '#000000', '#000000', '#000000'],
      dark: ['#0a0a0a', '#171717', '#ffffff', '#ffffff', '#ffffff'],
    },
    tags: ['bold', 'raw', 'high-contrast'],
  },
  {
    id: 'neon',
    name: 'Neon',
    description: 'Vibrant accents with cyberpunk-inspired glow.',
    previewColors: {
      light: ['#f8fafc', '#ffffff', '#6366f1', '#0f172a', '#e2e8f0'],
      dark: ['#050510', '#0a0a1a', '#6366f1', '#e2e8f0', 'rgba(99,102,241,0.2)'],
    },
    tags: ['vibrant', 'cyberpunk', 'glow'],
  },
  {
    id: 'warm',
    name: 'Warm',
    description: 'Earth tones, soft gradients, cozy feel.',
    previewColors: {
      light: ['#fef7ee', '#fffbf5', '#ea580c', '#431407', '#fed7aa'],
      dark: ['#1a0e04', '#271505', '#ea580c', '#fed7aa', '#7c2d12'],
    },
    tags: ['cozy', 'earth', 'soft'],
  },
  {
    id: 'liquid',
    name: 'Liquid',
    description: 'iOS-inspired translucent depth with frosted layers.',
    previewColors: {
      light: ['#f0f4f8', 'rgba(255,255,255,0.45)', '#007aff', '#1a2332', 'rgba(255,255,255,0.5)'],
      dark: ['#0d1117', 'rgba(255,255,255,0.04)', '#388bfd', '#e6edf3', 'rgba(255,255,255,0.08)'],
    },
    tags: ['ios', 'translucent', 'depth', '2026'],
  },
  {
    id: 'mono',
    name: 'Mono',
    description: 'Hyper-minimal monochrome. Developer-aesthetic precision.',
    previewColors: {
      light: ['#fafafa', '#ffffff', '#171717', '#171717', '#e5e5e5'],
      dark: ['#0a0a0a', '#111111', '#ededed', '#ededed', '#222222'],
    },
    tags: ['minimal', 'monochrome', 'developer', 'linear'],
  },
  {
    id: 'ocean',
    name: 'Ocean',
    description: 'Deep blue maritime palette with teal accents.',
    previewColors: {
      light: ['#eff8ff', '#ffffff', '#0891b2', '#0c2d48', '#bae6fd'],
      dark: ['#031e30', '#042f4a', '#06b6d4', '#bae6fd', '#0e4d6e'],
    },
    tags: ['blue', 'maritime', 'calming', 'teal'],
  },
  {
    id: 'forest',
    name: 'Forest',
    description: 'Nature-inspired organic greens. Grounded and sustainable.',
    previewColors: {
      light: ['#f5f7f2', '#fafcf8', '#16a34a', '#1a2e1a', '#c8dcc0'],
      dark: ['#0d1a0d', '#142814', '#22c55e', '#c8dcc0', '#1e3a1e'],
    },
    tags: ['nature', 'green', 'organic', 'sustainable'],
  },
];

const DEFAULT_DESIGN_THEME: DesignThemeId = 'classic';

// ── Context ──

interface ThemeState {
  /** Light/dark/system selection */
  theme: ColorMode;
  /** Resolved light or dark */
  resolved: 'light' | 'dark';
  /** Set light/dark mode */
  setTheme: (t: ColorMode) => void;
  /** Current design theme ID */
  designTheme: DesignThemeId;
  /** Set design theme */
  setDesignTheme: (id: DesignThemeId) => void;
  /** All available design themes */
  designThemes: DesignThemeInfo[];
  /** Get current design theme metadata */
  currentDesignTheme: DesignThemeInfo;
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
  const [theme, setThemeState] = useState<ColorMode>(() =>
    (localStorage.getItem('parkhub_theme') as ColorMode) || 'system'
  );

  const [designTheme, setDesignThemeState] = useState<DesignThemeId>(() => {
    const stored = localStorage.getItem('parkhub_design_theme');
    if (stored && DESIGN_THEMES.some(t => t.id === stored)) {
      return stored as DesignThemeId;
    }
    return DEFAULT_DESIGN_THEME;
  });

  // Reactively track OS preference so `resolved` updates on system change
  const systemTheme = useSyncExternalStore(
    subscribeToSystemTheme,
    getSystemSnapshot,
    getServerSnapshot,
  );

  const resolved = theme === 'system' ? systemTheme : theme;

  // Apply light/dark class
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

  // Apply design theme data attribute
  useEffect(() => {
    document.documentElement.dataset.designTheme = designTheme;
  }, [designTheme]);

  const setTheme = useCallback((t: ColorMode) => {
    setThemeState(t);
    localStorage.setItem('parkhub_theme', t);
  }, []);

  const setDesignTheme = useCallback((id: DesignThemeId) => {
    if (!DESIGN_THEMES.some(t => t.id === id)) return;
    setDesignThemeState(id);
    localStorage.setItem('parkhub_design_theme', id);
    // Sync to server if user is logged in (uses cookie or in-memory token)
    const token = getInMemoryToken();
    fetch('/api/v1/preferences/theme', {
      method: 'PUT',
      credentials: 'include',
      headers: {
        'Content-Type': 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
      },
      body: JSON.stringify({ design_theme: id }),
    }).catch(() => {});
  }, []);

  const currentDesignTheme = DESIGN_THEMES.find(t => t.id === designTheme) || DESIGN_THEMES[0];

  return (
    <ThemeContext.Provider value={{
      theme,
      resolved,
      setTheme,
      designTheme,
      setDesignTheme,
      designThemes: DESIGN_THEMES,
      currentDesignTheme,
    }}>
      {children}
    </ThemeContext.Provider>
  );
}

export function useTheme() {
  const ctx = useContext(ThemeContext);
  if (!ctx) throw new Error('useTheme must be used within ThemeProvider');
  return ctx;
}
