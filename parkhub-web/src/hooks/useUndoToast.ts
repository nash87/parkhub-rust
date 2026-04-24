import { useCallback, useEffect, useRef, useState } from 'react';

export const UNDO_WINDOW_MS = 10_000;

export interface UndoOffer {
  /** Inverse mutation — runs when the user clicks the Zurückgängig button. */
  inverse: () => void | Promise<void>;
  /** Label displayed on the undo affordance (e.g. the toast action text). */
  label: string;
}

/**
 * Tier-2 item 11 — "Rückgängig" / undo-last-action helper.
 *
 * After a destructive mutation finishes, call `offer(...)` with the inverse
 * mutation. Within {@link UNDO_WINDOW_MS} the caller can invoke `undo()` to
 * trigger the inverse; after the window expires the offer is silently
 * dropped. `active` mirrors the pending state so UI can render the button.
 */
export function useUndoToast() {
  const [active, setActive] = useState(false);
  const offerRef = useRef<UndoOffer | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clear = useCallback(() => {
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
    offerRef.current = null;
    setActive(false);
  }, []);

  const offer = useCallback((next: UndoOffer) => {
    if (timerRef.current) clearTimeout(timerRef.current);
    offerRef.current = next;
    setActive(true);
    timerRef.current = setTimeout(() => { clear(); }, UNDO_WINDOW_MS);
  }, [clear]);

  const undo = useCallback(() => {
    const current = offerRef.current;
    if (!current) return;
    const inverse = current.inverse;
    clear();
    void inverse();
  }, [clear]);

  useEffect(() => () => {
    if (timerRef.current) clearTimeout(timerRef.current);
  }, []);

  return { offer, undo, active };
}
