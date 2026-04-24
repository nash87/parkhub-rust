import { beforeEach, describe, expect, it } from 'vitest';
import { readLastUsed, writeLastUsed } from './lastUsed';

describe('lastUsed', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('returns null when no value has been stored yet', () => {
    expect(readLastUsed('lot')).toBeNull();
  });

  it('round-trips a value through the namespaced key', () => {
    writeLastUsed('lot', 'lot-42');
    expect(readLastUsed('lot')).toBe('lot-42');
    // Stored under the ph-v5-last: prefix so it's debuggable in devtools
    expect(window.localStorage.getItem('ph-v5-last:lot')).toBe('lot-42');
  });

  it('clears the key when writing null or empty string', () => {
    writeLastUsed('lot', 'lot-42');
    writeLastUsed('lot', null);
    expect(readLastUsed('lot')).toBeNull();
    writeLastUsed('lot', 'lot-42');
    writeLastUsed('lot', '');
    expect(readLastUsed('lot')).toBeNull();
  });
});
