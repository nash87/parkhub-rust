import { useCallback, useEffect, useState } from 'react';

// Shared density preference — applied as a `data-density` attribute on
// <html> so every component can opt into tighter/looser spacing via CSS
// (see styles/global.css density section). Changing the attribute reflows
// the whole app without a remount.
export type Density = 'compact' | 'cozy' | 'comfortable';

export const DENSITY_KEY = 'parkhub.ui.density';
const DENSITY_EVENT = 'parkhub:density';
const ALLOWED: Density[] = ['compact', 'cozy', 'comfortable'];

function read(fallback: Density = 'cozy'): Density {
  if (typeof window === 'undefined') return fallback;
  try {
    const v = window.localStorage.getItem(DENSITY_KEY);
    return ALLOWED.includes(v as Density) ? (v as Density) : fallback;
  } catch {
    return fallback;
  }
}

function write(value: Density): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(DENSITY_KEY, value);
  } catch {
    /* quota / private mode — accept silently */
  }
}

/** Apply the attribute to <html> as soon as the hook is consumed. */
function applyToRoot(value: Density) {
  if (typeof document !== 'undefined') {
    document.documentElement.setAttribute('data-density', value);
  }
}

export function useDensity(): readonly [Density, (next: Density) => void] {
  const [density, setDensityState] = useState<Density>(() => {
    const initial = read();
    applyToRoot(initial);
    return initial;
  });

  useEffect(() => {
    applyToRoot(density);
  }, [density]);

  useEffect(() => {
    function handleCustom(e: Event) {
      const detail = (e as CustomEvent<Density>).detail;
      if (detail && ALLOWED.includes(detail)) setDensityState(detail);
    }
    function handleStorage(e: StorageEvent) {
      if (e.key !== DENSITY_KEY || !e.newValue) return;
      if (ALLOWED.includes(e.newValue as Density)) setDensityState(e.newValue as Density);
    }
    window.addEventListener(DENSITY_EVENT, handleCustom);
    window.addEventListener('storage', handleStorage);
    return () => {
      window.removeEventListener(DENSITY_EVENT, handleCustom);
      window.removeEventListener('storage', handleStorage);
    };
  }, []);

  const setDensity = useCallback((next: Density) => {
    if (!ALLOWED.includes(next)) return;
    write(next);
    setDensityState(next);
    window.dispatchEvent(new CustomEvent<Density>(DENSITY_EVENT, { detail: next }));
  }, []);

  return [density, setDensity] as const;
}
