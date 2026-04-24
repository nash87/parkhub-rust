import { useCallback, useEffect, useRef, useState } from 'react';

const PREFIX = 'ph-v5-filters-';

export function persistedFilterKey(screenId: string): string {
  return `${PREFIX}${screenId}`;
}

/**
 * Tier-2 item 14 — filter-state persistence hook.
 *
 * Stores the filter object under `localStorage['ph-v5-filters-${screenId}']`,
 * rehydrates on mount, and exposes a `reset()` that both restores the
 * initial value and clears the persisted entry (used by the
 * "Filter zurücksetzen" button).
 */
export function usePersistedFilter<T>(
  screenId: string,
  initial: T,
): readonly [T, (next: T) => void, () => void] {
  const storageKey = persistedFilterKey(screenId);
  const initialRef = useRef(initial);

  const read = useCallback((): T => {
    try {
      const raw = typeof window !== 'undefined' ? window.localStorage.getItem(storageKey) : null;
      if (!raw) return initialRef.current;
      return JSON.parse(raw) as T;
    } catch {
      return initialRef.current;
    }
  }, [storageKey]);

  const [value, setValue] = useState<T>(read);

  // Rehydrate once after mount to cover SSR hydration edge cases.
  useEffect(() => { setValue(read()); }, [read]);

  const set = useCallback((next: T) => {
    setValue(next);
    try { window.localStorage.setItem(storageKey, JSON.stringify(next)); }
    catch { /* quota / disabled-storage — ignore */ }
  }, [storageKey]);

  const reset = useCallback(() => {
    setValue(initialRef.current);
    try { window.localStorage.removeItem(storageKey); } catch { /* ignore */ }
  }, [storageKey]);

  return [value, set, reset] as const;
}
