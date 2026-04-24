/**
 * View Transitions API helper for v5 client navigation.
 *
 * The browser-native View Transitions API (Chromium 111+, Safari 18+)
 * lets us fade between screen states with zero JS animation code —
 * the browser snapshots before/after and cross-fades pseudo-elements
 * (::view-transition-old / ::view-transition-new).
 *
 * Fallback behaviour: when the API is missing (older browsers), we
 * simply invoke the callback synchronously. No jank, no try/catch
 * noise in the caller.
 *
 * We also skip transitions when the user prefers reduced motion —
 * the CSS already disables the cross-fade, but skipping the API
 * call entirely avoids the (tiny) layout-snapshot cost too.
 */

type StartViewTransition = (cb: () => void) => { finished: Promise<void> };

export function startViewTransition(update: () => void): void {
  if (typeof document === 'undefined') {
    update();
    return;
  }

  const prefersReducedMotion =
    typeof window !== 'undefined' &&
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  const doc = document as Document & { startViewTransition?: StartViewTransition };

  if (prefersReducedMotion || typeof doc.startViewTransition !== 'function') {
    update();
    return;
  }

  doc.startViewTransition(update);
}
