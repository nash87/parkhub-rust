import { useCallback, useEffect, useRef, useState } from 'react';

/**
 * Hook helpers for {@link useDraftFromActive}.
 *
 * Most callers only care about the `[draft, setDraft]` tuple, but `meta`
 * exposes derived state (`isDirty`) and a manual `reset` so screens can offer
 * "Discard changes" affordances without re-implementing the comparison.
 */
export interface UseDraftFromActiveMeta<TDraft> {
  /** True when the draft has been seeded AND differs from the snapshot. */
  isDirty: boolean;
  /** Re-snapshot the draft from `active` (or clear it if `active` is undefined). */
  reset: () => void;
  /** The exact draft value last seeded from `active` (the "pristine" baseline). */
  pristine: TDraft | undefined;
}

export interface UseDraftFromActiveOptions<TActive, TDraft, TKey extends keyof TActive> {
  /**
   * Property used as the active item's stable discriminator.
   *
   * When omitted, the hook tries `active.id` if present, then falls back to
   * reference equality (`Object.is(active, last)`). Reference fallback is
   * correct when callers always pass a stable object reference for the same
   * logical item — typical for state that lives in a parent ref or selector.
   */
  idKey?: TKey;
  /**
   * Project the active item into the draft shape. Defaults to a shallow copy
   * — `[...arr]` for arrays, `{ ...obj }` for objects, `value` for primitives.
   * Override when the draft only mirrors a subset of the active item
   * (e.g. `(p) => p.body`).
   *
   * MUST be stable across renders (wrap in `useCallback` upstream). The hook
   * does not memoise it for you.
   */
  derive?: (active: TActive) => TDraft;
}

export type UseDraftFromActiveResult<TDraft> = [
  TDraft | undefined,
  React.Dispatch<React.SetStateAction<TDraft | undefined>>,
  UseDraftFromActiveMeta<TDraft>,
];

function defaultDerive<TActive, TDraft>(active: TActive): TDraft {
  // Arrays must be cloned with [...arr] — object spread on an array produces
  // `{ '0': ..., '1': ... }` which is silently wrong.
  if (Array.isArray(active)) {
    return [...active] as unknown as TDraft;
  }
  if (active !== null && typeof active === 'object') {
    return { ...(active as object) } as unknown as TDraft;
  }
  // Primitives pass through verbatim.
  return active as unknown as TDraft;
}

/**
 * Track a draft mirror of an "active" item without clobbering in-flight edits.
 *
 * The naive shape (`useEffect(() => setDraft(active.body), [active])`) re-seeds
 * the draft every time the parent rerenders the same active object — which is
 * exactly what happens when react-query refetches the list and produces a new
 * array. The classic workaround is `[activeId]` plus an
 * `eslint-disable-next-line react-hooks/exhaustive-deps`, which silences the
 * linter but loses the dependency-tracking guarantee.
 *
 * This hook keeps the lint rule honest: it depends on the full `active`
 * reference, but uses a ref-tracked id to bail out when the discriminator has
 * not actually changed. Switching ids snapshots `derive(active)`; same id +
 * parent rerender leaves the draft alone; `active === undefined` clears the
 * draft.
 *
 * Generic over the discriminator key so callers using `slug`, `uuid`, etc.
 * can plug in without falling back to `any`. When the type has neither `id`
 * nor a custom `idKey`, the hook falls back to reference equality.
 *
 * @example Mirror a string body (the Policies screen)
 * ```ts
 * const [draft, setDraft] = useDraftFromActive(active, {
 *   derive: (p) => p.body,
 * });
 * ```
 *
 * @example Snapshot the whole record (default)
 * ```ts
 * const [draft, setDraft, { isDirty, reset }] = useDraftFromActive(active);
 * ```
 *
 * @example Custom discriminator
 * ```ts
 * useDraftFromActive(active, { idKey: 'slug', derive: (p) => p.body });
 * ```
 */
export function useDraftFromActive<
  TActive extends object,
  TDraft = TActive,
  TKey extends keyof TActive = keyof TActive,
>(
  active: TActive | null | undefined,
  options?: UseDraftFromActiveOptions<TActive, TDraft, TKey>,
): UseDraftFromActiveResult<TDraft> {
  const idKey = options?.idKey;
  const derive = options?.derive ?? (defaultDerive as (a: TActive) => TDraft);

  // Discriminator strategy:
  //   1. Explicit `idKey` → read that property.
  //   2. No `idKey` but `active.id` exists at runtime → use it.
  //   3. Neither → reference equality on `active` itself (correct when the
  //      caller passes a stable parent-owned reference per logical item).
  const readId = useCallback(
    (a: TActive): unknown => {
      if (idKey !== undefined) {
        return (a as unknown as Record<string, unknown>)[idKey as string];
      }
      const maybeId = (a as unknown as { id?: unknown }).id;
      return maybeId !== undefined ? maybeId : a;
    },
    [idKey],
  );

  // `seededRef` distinguishes "never seeded" from "seeded with a value that
  // happens to be undefined" — `pristineRef.current === undefined` alone is
  // ambiguous when the draft is intentionally undefined.
  const seededRef = useRef(false);
  const pristineRef = useRef<TDraft | undefined>(undefined);
  const lastIdRef = useRef<unknown>(undefined);

  const [draft, setDraft] = useState<TDraft | undefined>(() => {
    if (active != null) {
      const seed = derive(active);
      pristineRef.current = seed;
      lastIdRef.current = readId(active);
      seededRef.current = true;
      return seed;
    }
    return undefined;
  });

  useEffect(() => {
    const nextId: unknown = active != null ? readId(active) : undefined;
    if (lastIdRef.current === nextId && seededRef.current === (active != null)) {
      // Same id (or both null) AND seeding state matches: preserve in-flight edits.
      return;
    }
    lastIdRef.current = nextId;
    if (active == null) {
      pristineRef.current = undefined;
      seededRef.current = false;
      setDraft(undefined);
      return;
    }
    const seed = derive(active);
    pristineRef.current = seed;
    seededRef.current = true;
    setDraft(seed);
  }, [active, derive, readId]);

  const reset = useCallback(() => {
    if (active == null) {
      pristineRef.current = undefined;
      seededRef.current = false;
      setDraft(undefined);
      return;
    }
    const seed = derive(active);
    pristineRef.current = seed;
    seededRef.current = true;
    setDraft(seed);
  }, [active, derive]);

  const isDirty = seededRef.current && !Object.is(draft, pristineRef.current);

  return [draft, setDraft, { isDirty, reset, pristine: pristineRef.current }];
}
