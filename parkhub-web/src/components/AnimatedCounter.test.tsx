import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { act, render, screen } from '@testing-library/react';

import { AnimatedCounter } from './AnimatedCounter';

describe('AnimatedCounter', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.stubGlobal('requestAnimationFrame', vi.fn(() => 1));
    vi.stubGlobal('cancelAnimationFrame', vi.fn());
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('renders the initial zero value', () => {
    render(<AnimatedCounter value={0} />);
    // Both the aria-hidden visual span AND the aria-live sr-only span
    // contain "0". getAllByText returns both.
    expect(screen.getAllByText('0').length).toBeGreaterThanOrEqual(1);
  });

  it('animates upwards to the provided value', () => {
    let now = 0;
    vi.spyOn(performance, 'now').mockImplementation(() => now);
    let rafId = 0;
    const frames = new Map<number, FrameRequestCallback>();

    vi.stubGlobal('requestAnimationFrame', vi.fn((cb: FrameRequestCallback) => {
      rafId += 1;
      frames.set(rafId, cb);
      return rafId;
    }));
    vi.stubGlobal('cancelAnimationFrame', vi.fn((id: number) => {
      frames.delete(id);
    }));

    render(<AnimatedCounter value={10} duration={1000} />);

    // The aria-hidden visual span starts at 0; the aria-live mirror already
    // renders the final value 10 for screen readers.
    expect(screen.getAllByText('0').length).toBeGreaterThanOrEqual(1);
    expect(requestAnimationFrame).toHaveBeenCalledTimes(1);

    act(() => {
      now = 500;
      frames.get(1)?.(now);
    });

    expect(screen.getAllByText('9').length).toBeGreaterThanOrEqual(1);

    act(() => {
      now = 1000;
      frames.get(2)?.(now);
    });

    // After animation completes, visual and sr-only both show 10.
    expect(screen.getAllByText('10').length).toBeGreaterThanOrEqual(1);
  });

  it('does not schedule animation when the value does not change', () => {
    const raf = vi.fn(() => 1);
    vi.stubGlobal('requestAnimationFrame', raf);
    vi.stubGlobal('cancelAnimationFrame', vi.fn());

    render(<AnimatedCounter value={0} />);

    expect(raf).not.toHaveBeenCalled();
  });

  it('cancels the queued animation frame on unmount', () => {
    const raf = vi.fn(() => 7);
    const cancel = vi.fn();
    vi.stubGlobal('requestAnimationFrame', raf);
    vi.stubGlobal('cancelAnimationFrame', cancel);

    const { unmount } = render(<AnimatedCounter value={5} />);
    unmount();

    expect(raf).toHaveBeenCalledTimes(1);
    expect(cancel).toHaveBeenCalledWith(7);
  });
});
