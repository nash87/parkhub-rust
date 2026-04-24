import { useEffect } from 'react';

/**
 * Context-aware keyboard shortcuts hook.
 *
 * Pass a `{ key: handler }` map. Keys match `KeyboardEvent.key`
 * (case-insensitive for letters). The hook attaches a single
 * `keydown` listener on `document` for the lifetime of the
 * subscribing component.
 *
 * Safeguards (industry-standard):
 *  - Ignores events that originate from `<input>`, `<textarea>`,
 *    `<select>`, or any element with `contenteditable`. Users
 *    typing "n" into a plate number must never trigger the
 *    shortcut to open a new booking.
 *  - Ignores events while a meta/ctrl/alt modifier is held, so
 *    we don't collide with browser / OS shortcuts.
 *  - Preserves the `?` shortcut even inside non-input elements
 *    so the help overlay remains discoverable globally.
 *
 * Usage:
 *   useKeyboardShortcuts({
 *     'n': () => navigate('buchen'),
 *     '/': () => focusSearch(),
 *     'Escape': () => closeModal(),
 *   });
 */
export type ShortcutMap = Record<string, (event: KeyboardEvent) => void>;

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target instanceof HTMLInputElement) return true;
  if (target instanceof HTMLTextAreaElement) return true;
  if (target instanceof HTMLSelectElement) return true;
  if (target.isContentEditable) return true;
  return false;
}

export function useKeyboardShortcuts(shortcuts: ShortcutMap, enabled = true): void {
  useEffect(() => {
    if (!enabled) return;
    if (typeof window === 'undefined') return;

    const handler = (event: KeyboardEvent) => {
      if (event.metaKey || event.ctrlKey || event.altKey) return;
      if (isEditableTarget(event.target)) return;
      // Normalise single-letter keys so we don't have to worry about
      // locale-specific Shift handling (KeyboardEvent.key returns
      // uppercase when Shift is held — we want "n" and "N" to be the
      // same shortcut unless the caller explicitly registers "N").
      const bareKey = event.key.length === 1 ? event.key.toLowerCase() : event.key;
      const handlerFn = shortcuts[bareKey] ?? shortcuts[event.key];
      if (handlerFn) {
        handlerFn(event);
      }
    };

    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [shortcuts, enabled]);
}
