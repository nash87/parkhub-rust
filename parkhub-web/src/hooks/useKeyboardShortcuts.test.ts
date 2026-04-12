import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook } from '@testing-library/react';

// ── Mocks ──

const mockNavigate = vi.fn();

vi.mock('react-router-dom', () => ({
  useNavigate: () => mockNavigate,
}));

import { useKeyboardShortcuts } from './useKeyboardShortcuts';

describe('useKeyboardShortcuts', () => {
  let onToggleCommandPalette: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    mockNavigate.mockClear();
    onToggleCommandPalette = vi.fn();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  function fireKey(key: string, opts: Partial<KeyboardEventInit> = {}) {
    window.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, ...opts }));
  }

  it('navigates to /book on Ctrl+B', () => {
    renderHook(() => useKeyboardShortcuts({ onToggleCommandPalette }));

    fireKey('b', { ctrlKey: true });

    expect(mockNavigate).toHaveBeenCalledWith('/book');
  });

  it('navigates to /book on Meta+B (macOS)', () => {
    renderHook(() => useKeyboardShortcuts({ onToggleCommandPalette }));

    fireKey('b', { metaKey: true });

    expect(mockNavigate).toHaveBeenCalledWith('/book');
  });

  it('toggles command palette on Ctrl+K', () => {
    renderHook(() => useKeyboardShortcuts({ onToggleCommandPalette }));

    fireKey('k', { ctrlKey: true });

    expect(onToggleCommandPalette).toHaveBeenCalledOnce();
  });

  it('toggles command palette on Meta+K (macOS)', () => {
    renderHook(() => useKeyboardShortcuts({ onToggleCommandPalette }));

    fireKey('k', { metaKey: true });

    expect(onToggleCommandPalette).toHaveBeenCalledOnce();
  });

  it('does not navigate on B without modifier', () => {
    renderHook(() => useKeyboardShortcuts({ onToggleCommandPalette }));

    fireKey('b');

    expect(mockNavigate).not.toHaveBeenCalled();
  });

  it('does not toggle command palette on K without modifier', () => {
    renderHook(() => useKeyboardShortcuts({ onToggleCommandPalette }));

    fireKey('k');

    expect(onToggleCommandPalette).not.toHaveBeenCalled();
  });

  it('does not respond to unrelated keys with modifier', () => {
    renderHook(() => useKeyboardShortcuts({ onToggleCommandPalette }));

    fireKey('a', { ctrlKey: true });

    expect(mockNavigate).not.toHaveBeenCalled();
    expect(onToggleCommandPalette).not.toHaveBeenCalled();
  });

  it('cleans up event listener on unmount', () => {
    const removeSpy = vi.spyOn(window, 'removeEventListener');
    const { unmount } = renderHook(() => useKeyboardShortcuts({ onToggleCommandPalette }));

    unmount();

    expect(removeSpy).toHaveBeenCalledWith('keydown', expect.any(Function));
  });

  it('handles multiple rapid key presses', () => {
    renderHook(() => useKeyboardShortcuts({ onToggleCommandPalette }));

    fireKey('k', { ctrlKey: true });
    fireKey('k', { ctrlKey: true });
    fireKey('k', { ctrlKey: true });

    expect(onToggleCommandPalette).toHaveBeenCalledTimes(3);
  });
});
