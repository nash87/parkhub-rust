import { describe, it, expect, beforeEach } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import { useNavLayout, NAV_LAYOUT_KEY } from './useNavLayout';

describe('useNavLayout', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('defaults to classic when nothing is stored', () => {
    const { result } = renderHook(() => useNavLayout());
    expect(result.current[0]).toBe('classic');
  });

  it('reads the stored layout on first render', () => {
    window.localStorage.setItem(NAV_LAYOUT_KEY, 'rail');
    const { result } = renderHook(() => useNavLayout());
    expect(result.current[0]).toBe('rail');
  });

  it('ignores unknown values and falls back to classic', () => {
    window.localStorage.setItem(NAV_LAYOUT_KEY, 'martian');
    const { result } = renderHook(() => useNavLayout());
    expect(result.current[0]).toBe('classic');
  });

  it('writes the new value to localStorage when set', () => {
    const { result } = renderHook(() => useNavLayout());
    act(() => result.current[1]('dock'));
    expect(window.localStorage.getItem(NAV_LAYOUT_KEY)).toBe('dock');
    expect(result.current[0]).toBe('dock');
  });

  it('rejects unknown values via setLayout', () => {
    const { result } = renderHook(() => useNavLayout());
    // @ts-expect-error — intentionally wrong type to verify runtime guard
    act(() => result.current[1]('not-a-layout'));
    expect(result.current[0]).toBe('classic');
  });

  it('propagates same-tab changes to other consumers via custom event', () => {
    const { result: a } = renderHook(() => useNavLayout());
    const { result: b } = renderHook(() => useNavLayout());
    act(() => a.current[1]('top'));
    expect(a.current[0]).toBe('top');
    expect(b.current[0]).toBe('top');
  });

  it('picks up cross-tab storage events', () => {
    const { result } = renderHook(() => useNavLayout());
    // jsdom's StorageEvent constructor in some CodeQL ruleset versions is
    // typed as one-arg. Build the event with `new Event` + Object.assign to
    // sidestep that without changing the dispatched shape our listener
    // actually reads (e.key / e.newValue).
    act(() => {
      const ev = Object.assign(new Event('storage'), {
        key: NAV_LAYOUT_KEY,
        newValue: 'rail',
      });
      window.dispatchEvent(ev);
    });
    expect(result.current[0]).toBe('rail');
  });
});
