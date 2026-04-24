import { afterEach, describe, expect, it, vi } from 'vitest';
import { startViewTransition } from './viewTransitions';

describe('startViewTransition', () => {
  const originalStartVT = (document as unknown as {
    startViewTransition?: unknown;
  }).startViewTransition;
  const originalMatchMedia = window.matchMedia;

  afterEach(() => {
    if (originalStartVT === undefined) {
      delete (document as unknown as { startViewTransition?: unknown }).startViewTransition;
    } else {
      (document as unknown as { startViewTransition?: unknown }).startViewTransition =
        originalStartVT;
    }
    window.matchMedia = originalMatchMedia;
  });

  it('calls the update callback directly when the API is missing', () => {
    delete (document as unknown as { startViewTransition?: unknown }).startViewTransition;
    const cb = vi.fn();
    startViewTransition(cb);
    expect(cb).toHaveBeenCalledTimes(1);
  });

  it('routes the update through document.startViewTransition when present', () => {
    const transition = { finished: Promise.resolve() };
    const vt = vi.fn((cb: () => void) => {
      cb();
      return transition;
    });
    (document as unknown as { startViewTransition: typeof vt }).startViewTransition = vt;
    window.matchMedia = vi.fn().mockReturnValue({ matches: false }) as unknown as typeof window.matchMedia;
    const cb = vi.fn();
    startViewTransition(cb);
    expect(vt).toHaveBeenCalledTimes(1);
    expect(cb).toHaveBeenCalledTimes(1);
  });

  it('skips the API when prefers-reduced-motion matches', () => {
    const vt = vi.fn();
    (document as unknown as { startViewTransition: typeof vt }).startViewTransition = vt;
    window.matchMedia = vi.fn().mockReturnValue({ matches: true }) as unknown as typeof window.matchMedia;
    const cb = vi.fn();
    startViewTransition(cb);
    expect(vt).not.toHaveBeenCalled();
    expect(cb).toHaveBeenCalledTimes(1);
  });
});
