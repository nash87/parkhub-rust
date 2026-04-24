import { describe, it, expect, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { usePersistedFilter, persistedFilterKey } from './usePersistedFilter';

describe('usePersistedFilter', () => {
  beforeEach(() => { window.localStorage.clear(); });

  it('key helper uses the ph-v5-filters- prefix', () => {
    expect(persistedFilterKey('nutzer')).toBe('ph-v5-filters-nutzer');
  });

  it('returns the initial value when nothing is persisted', () => {
    const { result } = renderHook(() => usePersistedFilter('nutzer', { q: '' }));
    expect(result.current[0]).toEqual({ q: '' });
  });

  it('rehydrates the persisted value on mount', () => {
    window.localStorage.setItem('ph-v5-filters-nutzer', JSON.stringify({ q: 'admin' }));
    const { result } = renderHook(() => usePersistedFilter('nutzer', { q: '' }));
    expect(result.current[0]).toEqual({ q: 'admin' });
  });

  it('writes to localStorage when the value is updated', () => {
    const { result } = renderHook(() => usePersistedFilter('nutzer', { q: '' }));
    act(() => { result.current[1]({ q: 'support' }); });
    expect(JSON.parse(window.localStorage.getItem('ph-v5-filters-nutzer')!)).toEqual({ q: 'support' });
  });

  it('reset() restores the initial value and clears the persisted entry', () => {
    window.localStorage.setItem('ph-v5-filters-nutzer', JSON.stringify({ q: 'admin' }));
    const { result } = renderHook(() => usePersistedFilter('nutzer', { q: '' }));
    act(() => { result.current[2](); });
    expect(result.current[0]).toEqual({ q: '' });
    expect(window.localStorage.getItem('ph-v5-filters-nutzer')).toBeNull();
  });

  it('ignores malformed persisted values and falls back to initial', () => {
    window.localStorage.setItem('ph-v5-filters-nutzer', '{not json');
    const { result } = renderHook(() => usePersistedFilter('nutzer', { q: '' }));
    expect(result.current[0]).toEqual({ q: '' });
  });
});
