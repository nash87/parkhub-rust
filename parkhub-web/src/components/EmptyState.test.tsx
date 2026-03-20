import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';

// ── Mocks ──

vi.mock('react-router-dom', () => ({
  Link: ({ to, children, ...props }: any) => <a href={to} {...props}>{children}</a>,
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
}));

const mockIsEnabled = vi.fn();

vi.mock('../context/FeaturesContext', () => ({
  useFeatures: () => ({
    isEnabled: mockIsEnabled,
  }),
}));

import { EmptyState } from './EmptyState';

describe('EmptyState', () => {
  it('renders title', () => {
    mockIsEnabled.mockReturnValue(false);
    render(<EmptyState variant="no-bookings" title="No bookings" />);
    expect(screen.getByText('No bookings')).toBeInTheDocument();
  });

  it('renders description when provided', () => {
    mockIsEnabled.mockReturnValue(false);
    render(<EmptyState variant="no-bookings" title="No bookings" description="Book a spot to get started" />);
    expect(screen.getByText('Book a spot to get started')).toBeInTheDocument();
  });

  it('does not render description when not provided', () => {
    mockIsEnabled.mockReturnValue(false);
    const { container } = render(<EmptyState variant="no-bookings" title="No bookings" />);
    // Only the title heading, no description paragraph
    const paragraphs = container.querySelectorAll('p');
    expect(paragraphs.length).toBe(0);
  });

  it('renders action link when provided', () => {
    mockIsEnabled.mockReturnValue(false);
    render(
      <EmptyState
        variant="no-bookings"
        title="No bookings"
        actionLabel="Book now"
        actionTo="/book"
      />
    );
    const link = screen.getByText('Book now');
    expect(link).toBeInTheDocument();
    expect(link.closest('a')).toHaveAttribute('href', '/book');
  });

  it('does not render action link when not provided', () => {
    mockIsEnabled.mockReturnValue(false);
    render(<EmptyState variant="no-bookings" title="No bookings" />);
    expect(screen.queryByRole('link')).not.toBeInTheDocument();
  });

  it('renders fallback icon when rich_empty_states is disabled', () => {
    mockIsEnabled.mockReturnValue(false);
    const FakeIcon = (props: any) => <span data-testid="fallback-icon" {...props} />;
    render(<EmptyState variant="no-bookings" title="No bookings" icon={FakeIcon} />);
    expect(screen.getByTestId('fallback-icon')).toBeInTheDocument();
  });

  it('renders SVG illustration when rich_empty_states is enabled', () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(<EmptyState variant="no-bookings" title="No bookings" />);
    const svg = container.querySelector('svg');
    expect(svg).toBeInTheDocument();
  });

  it('renders different illustration for no-vehicles variant', () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(<EmptyState variant="no-vehicles" title="No vehicles" />);
    const svg = container.querySelector('svg');
    expect(svg).toBeInTheDocument();
  });

  it('renders different illustration for no-transactions variant', () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(<EmptyState variant="no-transactions" title="No transactions" />);
    const svg = container.querySelector('svg');
    expect(svg).toBeInTheDocument();
  });

  it('renders no-data illustration as default', () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(<EmptyState variant="no-data" title="No data" />);
    const svg = container.querySelector('svg');
    expect(svg).toBeInTheDocument();
  });
});
