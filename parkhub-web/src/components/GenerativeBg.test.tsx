import { describe, it, expect, vi, afterEach } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';
import { renderHook } from '@testing-library/react';

// ── Mocks ──

const mockIsEnabled = vi.fn();

vi.mock('../context/FeaturesContext', () => ({
  useFeatures: () => ({ isEnabled: mockIsEnabled }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, variants, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
}));

import { GenerativeBg, useBgClass, type BgPattern } from './GenerativeBg';

describe('GenerativeBg', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('returns null when generative_bg feature is disabled', () => {
    mockIsEnabled.mockReturnValue(false);
    const { container } = render(<GenerativeBg />);
    expect(container.innerHTML).toBe('');
  });

  it('renders a div with topo-bg class by default when enabled', () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(<GenerativeBg />);
    const div = container.firstChild as HTMLElement;
    expect(div).toBeTruthy();
    expect(div.className).toContain('topo-bg');
    expect(div.getAttribute('aria-hidden')).toBe('true');
  });

  it('renders dots pattern class', () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(<GenerativeBg pattern="dots" />);
    expect((container.firstChild as HTMLElement).className).toContain('dot-matrix');
  });

  it('renders hatch pattern class', () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(<GenerativeBg pattern="hatch" />);
    expect((container.firstChild as HTMLElement).className).toContain('hatch-bg');
  });

  it('renders mesh pattern class', () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(<GenerativeBg pattern="mesh" />);
    expect((container.firstChild as HTMLElement).className).toContain('mesh-gradient');
  });

  it('renders grid pattern class', () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(<GenerativeBg pattern="grid" />);
    expect((container.firstChild as HTMLElement).className).toContain('parking-grid');
  });

  it('applies additional className', () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(<GenerativeBg className="extra-class" />);
    expect((container.firstChild as HTMLElement).className).toContain('extra-class');
  });

  it('has fixed inset-0 -z-10 classes', () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(<GenerativeBg />);
    const div = container.firstChild as HTMLElement;
    expect(div.className).toContain('fixed');
    expect(div.className).toContain('inset-0');
    expect(div.className).toContain('-z-10');
  });
});

describe('useBgClass', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('returns pattern class when generative_bg is enabled', () => {
    mockIsEnabled.mockReturnValue(true);
    const wrapper = ({ children }: { children: React.ReactNode }) => <>{children}</>;
    const { result } = renderHook(() => useBgClass('dots'), { wrapper });
    expect(result.current).toBe('dot-matrix');
  });

  it('returns empty string when generative_bg is disabled', () => {
    mockIsEnabled.mockReturnValue(false);
    const wrapper = ({ children }: { children: React.ReactNode }) => <>{children}</>;
    const { result } = renderHook(() => useBgClass('dots'), { wrapper });
    expect(result.current).toBe('');
  });

  it('defaults to topo pattern', () => {
    mockIsEnabled.mockReturnValue(true);
    const wrapper = ({ children }: { children: React.ReactNode }) => <>{children}</>;
    const { result } = renderHook(() => useBgClass(), { wrapper });
    expect(result.current).toBe('topo-bg');
  });
});
