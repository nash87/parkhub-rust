import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from 'react';

/** Three canonical v5 modes. Each flips the full token bundle on <html>. */
export type V5Mode = 'marble_light' | 'marble_dark' | 'void';

const MODES: readonly V5Mode[] = ['marble_light', 'marble_dark', 'void'] as const;
const STORAGE_KEY = 'ph-v5-mode';
const DEFAULT_MODE: V5Mode = 'marble_light';

interface V5ThemeCtx {
  mode: V5Mode;
  setMode: (m: V5Mode) => void;
  /** Convenience flags — avoid string comparisons scattered across components. */
  isVoid: boolean;
  isDark: boolean;
}

const Ctx = createContext<V5ThemeCtx | null>(null);

function readInitial(): V5Mode {
  if (typeof window === 'undefined') return DEFAULT_MODE;
  const stored = window.localStorage.getItem(STORAGE_KEY);
  if (stored && (MODES as readonly string[]).includes(stored)) return stored as V5Mode;
  // First paint honors OS preference for marble; void is opt-in only.
  const prefersDark = window.matchMedia?.('(prefers-color-scheme: dark)').matches;
  return prefersDark ? 'marble_dark' : 'marble_light';
}

export function V5ThemeProvider({ children }: { children: ReactNode }) {
  const [mode, setModeState] = useState<V5Mode>(readInitial);

  useEffect(() => {
    document.documentElement.setAttribute('data-ph-mode', mode);
    window.localStorage.setItem(STORAGE_KEY, mode);
    return () => {
      // Leave attribute in place — other pages outside the v5 shell may want
      // to inherit; cleanup would cause a flash on client-side nav back in.
    };
  }, [mode]);

  const setMode = useCallback((m: V5Mode) => setModeState(m), []);

  const value = useMemo<V5ThemeCtx>(
    () => ({ mode, setMode, isVoid: mode === 'void', isDark: mode !== 'marble_light' }),
    [mode, setMode]
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useV5Theme(): V5ThemeCtx {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error('useV5Theme must be used within <V5ThemeProvider>');
  return ctx;
}

export const V5_MODES = MODES;

export const V5_MODE_LABELS: Record<V5Mode, string> = {
  marble_light: '☀ Marble',
  marble_dark: '● Marble Dark',
  void: '◼ Void',
};
