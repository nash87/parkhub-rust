import { useCallback, useEffect, useRef, useState } from 'react';

/**
 * Hook helpers for {@link useDraftFromActive}.
 *
 * Most callers only care about the `[draft, setDraft]` tuple, but `meta`
 * exposes derived state (`isDirty`) and a manual `reset` so screens can offer
 * "Discard changes" affordances without re-implementing the comparison.
 */
export interface UseDraftFromActiveMeta<TDraft> {
  /** True when the current draft differs from the snapshot taken on init. */
  isDirty: boolean;
  /** Re-snapshot the draft from `active` (or clear it if `active` is undefined). */
  reset: () => void;
  /** The exact draft value last seeded from `active` (the "pristine" baseline). */
  pristine: TDraft | undefined;
}

export interface UseDraftFromActiveOptions<TActive, TDraft, TKey extends keyof TActive> {
  /**
   * Property used as the active item's stable discriminator. Defaults to `'id'`
   * — when active items lack an `id`, pass e.g. `idKey: 'slug'`.
   */
  idKey?: TKey;
  /**
   * Project the active item into the draft shape. Defaults to a shallow copy
   * of `active` (or `active` itself when it is a primitive). Override when the
   * draft only mirrors a subset of the active item (e.g. `(p) => p.body`).
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
  // Shallow-clone objects so callers can mutate the draft without touching
  // upstream state; pass primitives through verbatim.
  if (active !== null && typeof active === 'object') {
    return { ...(active as object) } as unknown as TDraft;
  }
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
 * can plug in without falling back to `any`.
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
 */
export function useDraftFromActive<
  TActive extends object,
  TDraft = TActive,
  TKey extends keyof TActive = 'id' extends keyof TActive ? 'id' : keyof TActive,
>(
  active: TActive | null | undefined,
  options?: UseDraftFromActiveOptions<TActive, TDraft, TKey>,
): UseDraftFromActiveResult<TDraft> {
  const idKey = (options?.idKey ?? ('id' as TKey)) as TKey;
  const derive = options?.derive ?? (defaultDerive as (a: TActive) => TDraft);
  // `idKey` is `keyof TActive`, but interfaces don't satisfy
  // `Record<string, unknown>` — cast through `unknown` to read by string.
  const readId = useCallback(
    (a: TActive): unknown => (a as unknown as Record<string, unknown>)[idKey as string],
    [idKey],
  );

  const pristineRef = useRef<TDraft | undefined>(undefined);
  const lastIdRef = useRef<unknown>(undefined);

  const [draft, setDraft] = useState<TDraft | undefined>(() => {
    if (active != null) {
      const seed = derive(active);
      pristineRef.current = seed;
      lastIdRef.current = readId(active);
      return seed;
    }
    return undefined;
  });

  useEffect(() => {
    const nextId: unknown = active != null ? readId(active) : undefined;
    if (lastIdRef.current === nextId) {
      // Same id (or both undefined): preserve in-flight edits.
      return;
    }
    lastIdRef.current = nextId;
    if (active == null) {
      pristineRef.current = undefined;
      setDraft(undefined);
      return;
    }
    const seed = derive(active);
    pristineRef.current = seed;
    setDraft(seed);
  }, [active, derive, readId]);

  const reset = useCallback(() => {
    if (active == null) {
      pristineRef.current = undefined;
      setDraft(undefined);
      return;
    }
    const seed = derive(active);
    pristineRef.current = seed;
    setDraft(seed);
  }, [active, derive]);

  const isDirty =
    pristineRef.current !== undefined && !Object.is(draft, pristineRef.current);

  return [draft, setDraft, { isDirty, reset, pristine: pristineRef.current }];
}
