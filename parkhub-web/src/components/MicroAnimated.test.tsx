import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';

// ── Mocks ──

const mockIsEnabled = vi.fn();

vi.mock('../context/FeaturesContext', () => ({
  useFeatures: () => ({ isEnabled: mockIsEnabled }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, variants, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props} data-testid="motion-div">{children}</div>
    )),
  },
}));

import { MicroAnimated, StaggerContainer, StaggerItem } from './MicroAnimated';

describe('MicroAnimated', () => {
  const originalMatchMedia = window.matchMedia;

  beforeEach(() => {
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
    }));
  });

  afterEach(() => {
    window.matchMedia = originalMatchMedia;
    vi.restoreAllMocks();
  });

  it('renders children in a plain div when micro_animations disabled', () => {
    mockIsEnabled.mockReturnValue(false);
    render(<MicroAnimated data-testid="wrapper">content</MicroAnimated>);
    expect(screen.getByText('content')).toBeInTheDocument();
    // Should NOT have motion-div testid (plain div)
    expect(screen.queryByTestId('motion-div')).not.toBeInTheDocument();
  });

  it('renders children in motion.div when micro_animations enabled', () => {
    mockIsEnabled.mockReturnValue(true);
    render(<MicroAnimated>animated content</MicroAnimated>);
    expect(screen.getByText('animated content')).toBeInTheDocument();
    expect(screen.getByTestId('motion-div')).toBeInTheDocument();
  });

  it('renders plain div when prefers-reduced-motion is active', () => {
    mockIsEnabled.mockReturnValue(true);
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: query === '(prefers-reduced-motion: reduce)',
      media: query,
    }));
    render(<MicroAnimated data-testid="wrapper">reduced</MicroAnimated>);
    expect(screen.getByText('reduced')).toBeInTheDocument();
    expect(screen.queryByTestId('motion-div')).not.toBeInTheDocument();
  });

  it('passes noHover and noPress props', () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(
      <MicroAnimated noHover noPress>child</MicroAnimated>
    );
    expect(container.textContent).toBe('child');
  });
});

describe('StaggerContainer', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders plain div when disabled', () => {
    mockIsEnabled.mockReturnValue(false);
    render(<StaggerContainer className="test-class">items</StaggerContainer>);
    const div = screen.getByText('items');
    expect(div.className).toBe('test-class');
    expect(div.tagName).toBe('DIV');
  });

  it('renders motion.div when enabled', () => {
    mockIsEnabled.mockReturnValue(true);
    render(<StaggerContainer>animated items</StaggerContainer>);
    expect(screen.getByTestId('motion-div')).toBeInTheDocument();
    expect(screen.getByText('animated items')).toBeInTheDocument();
  });

  it('passes delay prop (no crash)', () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(
      <StaggerContainer delay={0.5}>delayed</StaggerContainer>
    );
    expect(container.textContent).toBe('delayed');
  });
});

describe('StaggerItem', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders plain div when disabled', () => {
    mockIsEnabled.mockReturnValue(false);
    render(<StaggerItem className="item-class">item</StaggerItem>);
    const div = screen.getByText('item');
    expect(div.className).toBe('item-class');
  });

  it('renders motion.div when enabled', () => {
    mockIsEnabled.mockReturnValue(true);
    render(<StaggerItem>animated item</StaggerItem>);
    expect(screen.getByTestId('motion-div')).toBeInTheDocument();
  });
});
