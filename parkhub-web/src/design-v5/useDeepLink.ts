import { useEffect } from 'react';
import type { ScreenId } from './nav';
import { byId } from './nav';

/**
 * Deep-linking for the v5 SPA.
 *
 * Astro owns the outer route (`/v5`), so inside the React SPA we
 * can't use react-router's `<Route>` declaratively — we'd have
 * two routers fighting over the same URL. Instead we use the
 * History API directly:
 *
 *  - `readScreenFromUrl()`  — parses `/v5/<screenId>` once on
 *    first render, falling back to whatever the caller passes
 *    when the URL doesn't have a screen segment.
 *  - `useSyncScreenToUrl()` — pushes a new `/v5/<id>` entry
 *    whenever the screen changes, and listens for `popstate`
 *    so browser back/forward restores the correct screen.
 *
 * Notes:
 *  - Only accepts known screen IDs (checked against `byId`).
 *  - Debounces identical pushes so a React double-render in
 *    dev doesn't spam the history stack.
 *  - Never touches `window.history` during SSR / jsdom-without-window.
 */

const BASE_PATH = '/v5';

export function readScreenFromUrl(fallback: ScreenId): ScreenId {
  if (typeof window === 'undefined') return fallback;
  const path = window.location.pathname;
  // Accept both `/v5/<id>` and `/v5/<id>/`, trim query + hash.
  const match = path.match(/^\/v5\/([a-z][a-z0-9_-]*)\/?$/i);
  if (!match) return fallback;
  const id = match[1].toLowerCase();
  return byId.has(id) ? (id as ScreenId) : fallback;
}

export function writeScreenToUrl(screen: ScreenId): void {
  if (typeof window === 'undefined') return;
  const target = `${BASE_PATH}/${screen}`;
  if (window.location.pathname === target) return;
  try {
    window.history.pushState({ screen }, '', target);
  } catch {
    // Some test runners (and very old browsers) disallow pushState
    // on file:// URLs; deep-linking is a progressive enhancement.
  }
}

/**
 * Keep the URL and the in-memory screen state in sync.
 *
 * - Pushes `/v5/<screen>` into history whenever `screen` changes.
 * - Subscribes to `popstate` so the browser back/forward buttons
 *   call `onPopState` with whatever screen the URL now encodes.
 */
export function useSyncScreenToUrl(
  screen: ScreenId,
  onPopState: (screen: ScreenId) => void,
): void {
  useEffect(() => {
    writeScreenToUrl(screen);
  }, [screen]);

  useEffect(() => {
    if (typeof window === 'undefined') return;
    const handler = () => {
      onPopState(readScreenFromUrl(screen));
    };
    window.addEventListener('popstate', handler);
    return () => window.removeEventListener('popstate', handler);
  }, [screen, onPopState]);
}
