import { useCallback, useEffect, useState } from 'react';
import type { NavLayout } from '../components/ui/SettingsPrimitives';

// Single source of truth for the nav-layout localStorage key. Kept in a
// shared hook so Settings, Layout, and any future surface (mobile bottom
// nav, kiosk mode, etc.) all read/write the same slot.
export const NAV_LAYOUT_KEY = 'parkhub.nav.layout';
const NAV_LAYOUT_EVENT = 'parkhub:nav-layout';

const ALLOWED: NavLayout[] = ['classic', 'rail', 'top', 'dock', 'focus'];

function read(fallback: NavLayout = 'classic'): NavLayout {
  if (typeof window === 'undefined') return fallback;
  try {
    const v = window.localStorage.getItem(NAV_LAYOUT_KEY);
    return ALLOWED.includes(v as NavLayout) ? (v as NavLayout) : fallback;
  } catch {
    return fallback;
  }
}

function write(value: NavLayout): void {
  if (typeof window === 'undefined') return;
  try {
    window.localStorage.setItem(NAV_LAYOUT_KEY, value);
  } catch {
    /* quota / private mode — accept silently */
  }
}

/**
 * Reactive accessor for the user's nav-layout choice. Persists to
 * localStorage, propagates changes to every mounted consumer in the
 * same tab via a DOM CustomEvent, and picks up cross-tab updates via
 * the `storage` event — so switching the layout in Settings while
 * another tab is open reflows that tab too.
 */
export function useNavLayout(): readonly [NavLayout, (next: NavLayout) => void] {
  const [layout, setLayoutState] = useState<NavLayout>(() => read());

  useEffect(() => {
    function handleCustom(e: Event) {
      const detail = (e as CustomEvent<NavLayout>).detail;
      if (detail && ALLOWED.includes(detail)) setLayoutState(detail);
    }
    function handleStorage(e: StorageEvent) {
      if (e.key !== NAV_LAYOUT_KEY || !e.newValue) return;
      if (ALLOWED.includes(e.newValue as NavLayout)) {
        setLayoutState(e.newValue as NavLayout);
      }
    }
    window.addEventListener(NAV_LAYOUT_EVENT, handleCustom);
    window.addEventListener('storage', handleStorage);
    return () => {
      window.removeEventListener(NAV_LAYOUT_EVENT, handleCustom);
      window.removeEventListener('storage', handleStorage);
    };
  }, []);

  const setLayout = useCallback((next: NavLayout) => {
    if (!ALLOWED.includes(next)) return;
    write(next);
    setLayoutState(next);
    // Broadcast within the current tab so Settings ↔ Layout stay in sync
    // without a full reload (storage events don't fire in the tab that
    // did the write).
    window.dispatchEvent(new CustomEvent<NavLayout>(NAV_LAYOUT_EVENT, { detail: next }));
  }, []);

  return [layout, setLayout] as const;
}
