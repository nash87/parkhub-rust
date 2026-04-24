/**
 * Tiny localStorage-backed "last-used value" helper.
 *
 * Centralises the repeated pattern of
 *
 *   useState(() => window.localStorage.getItem(KEY) ?? '')
 *   useEffect(() => window.localStorage.setItem(KEY, value), [value])
 *
 * into `readLastUsed(key)` / `writeLastUsed(key, value)`.
 *
 * Why we do this: screens like Buchen and Fahrzeuge want to
 * pre-select the lot and vehicle the user picked last time.
 * Admin screens want to resume on the last tab they opened.
 * All of this lives under the `ph-v5-` key namespace so we
 * don't clash with other apps on the same origin.
 */

const NAMESPACE = 'ph-v5-last:';

export function readLastUsed(key: string): string | null {
  if (typeof window === 'undefined') return null;
  try {
    return window.localStorage.getItem(`${NAMESPACE}${key}`);
  } catch {
    return null;
  }
}

export function writeLastUsed(key: string, value: string | null | undefined): void {
  if (typeof window === 'undefined') return;
  try {
    const full = `${NAMESPACE}${key}`;
    if (value == null || value === '') {
      window.localStorage.removeItem(full);
    } else {
      window.localStorage.setItem(full, value);
    }
  } catch {
    // Private-mode / quota errors are non-fatal — smart defaults
    // are a UX nicety, not a correctness requirement.
  }
}
