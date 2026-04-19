import { describe, it, expect, beforeEach } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import { useDensity, DENSITY_KEY } from './useDensity';

describe('useDensity', () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.removeAttribute('data-density');
  });

  it('defaults to cozy and applies the attribute immediately', () => {
    const { result } = renderHook(() => useDensity());
    expect(result.current[0]).toBe('cozy');
    expect(document.documentElement.getAttribute('data-density')).toBe('cozy');
  });

  it('reads the stored density on first render', () => {
    window.localStorage.setItem(DENSITY_KEY, 'compact');
    const { result } = renderHook(() => useDensity());
    expect(result.current[0]).toBe('compact');
    expect(document.documentElement.getAttribute('data-density')).toBe('compact');
  });

  it('ignores unknown values and falls back to cozy', () => {
    window.localStorage.setItem(DENSITY_KEY, 'extreme');
    const { result } = renderHook(() => useDensity());
    expect(result.current[0]).toBe('cozy');
  });

  it('writes to localStorage and updates the attribute when set', () => {
    const { result } = renderHook(() => useDensity());
    act(() => result.current[1]('comfortable'));
    expect(window.localStorage.getItem(DENSITY_KEY)).toBe('comfortable');
    expect(document.documentElement.getAttribute('data-density')).toBe('comfortable');
  });

  it('propagates same-tab changes between consumers', () => {
    const { result: a } = renderHook(() => useDensity());
    const { result: b } = renderHook(() => useDensity());
    act(() => a.current[1]('compact'));
    expect(a.current[0]).toBe('compact');
    expect(b.current[0]).toBe('compact');
  });
});
