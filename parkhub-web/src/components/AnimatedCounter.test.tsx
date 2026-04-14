import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
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
    expect(screen.getByText('0')).toBeInTheDocument();
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

    expect(screen.getByText('0')).toBeInTheDocument();
    expect(requestAnimationFrame).toHaveBeenCalledTimes(1);

    act(() => {
      now = 500;
      frames.get(1)?.(now);
    });

    expect(screen.getByText('9')).toBeInTheDocument();

    act(() => {
      now = 1000;
      frames.get(2)?.(now);
    });

    expect(screen.getByText('10')).toBeInTheDocument();
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
