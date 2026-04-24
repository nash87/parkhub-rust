import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useUndoToast, UNDO_WINDOW_MS } from './useUndoToast';

describe('useUndoToast', () => {
  beforeEach(() => { vi.useFakeTimers(); });
  afterEach(() => { vi.runOnlyPendingTimers(); vi.useRealTimers(); });

  it('calls the inverse callback when undo() is invoked within the window', () => {
    const inverse = vi.fn().mockResolvedValue(undefined);
    const { result } = renderHook(() => useUndoToast());
    act(() => { result.current.offer({ inverse, label: 'Zurück' }); });
    act(() => { result.current.undo(); });
    expect(inverse).toHaveBeenCalledTimes(1);
  });

  it('expires after UNDO_WINDOW_MS so undo() becomes a no-op', () => {
    const inverse = vi.fn();
    const { result } = renderHook(() => useUndoToast());
    act(() => { result.current.offer({ inverse, label: 'Zurück' }); });
    act(() => { vi.advanceTimersByTime(UNDO_WINDOW_MS + 50); });
    act(() => { result.current.undo(); });
    expect(inverse).not.toHaveBeenCalled();
  });

  it('is active while an offer is pending and inactive after it expires', () => {
    const { result } = renderHook(() => useUndoToast());
    expect(result.current.active).toBe(false);
    act(() => { result.current.offer({ inverse: vi.fn(), label: 'Zurück' }); });
    expect(result.current.active).toBe(true);
    act(() => { vi.advanceTimersByTime(UNDO_WINDOW_MS + 50); });
    expect(result.current.active).toBe(false);
  });
});
