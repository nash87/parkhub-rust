import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockIsEnabled = vi.fn();

vi.mock('../context/FeaturesContext', () => ({
  useFeatures: () => ({ isEnabled: mockIsEnabled }),
}));

vi.mock('react-router-dom', () => ({
  Link: ({ to, children, ...props }: any) => <a href={to} {...props}>{children}</a>,
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, variants, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    button: React.forwardRef(({ children, initial, animate, exit, transition, variants, whileHover, whileTap, ...props }: any, ref: any) => (
      <button ref={ref} {...props}>{children}</button>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Plus: (props: any) => <span data-testid="icon-plus" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  CalendarPlus: (props: any) => <span data-testid="icon-cal-plus" {...props} />,
  Car: (props: any) => <span data-testid="icon-car" {...props} />,
  CoinVertical: (props: any) => <span data-testid="icon-coin" {...props} />,
  CalendarCheck: (props: any) => <span data-testid="icon-cal-check" {...props} />,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'dashboard.viewBookings': 'View Bookings',
        'nav.credits': 'Credits',
        'dashboard.myVehicles': 'My Vehicles',
        'dashboard.bookSpot': 'Book a Spot',
        'commandPalette.closeQuickActions': 'Close Quick Actions',
        'commandPalette.openQuickActions': 'Open Quick Actions',
      };
      return map[key] || key;
    },
  }),
}));

import { QuickActionsFab } from './QuickActionsFab';

describe('QuickActionsFab', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('returns null when fab_quick_actions is disabled', () => {
    mockIsEnabled.mockReturnValue(false);
    const { container } = render(<QuickActionsFab />);
    expect(container.innerHTML).toBe('');
  });

  it('renders the FAB button when enabled', () => {
    mockIsEnabled.mockImplementation((f: string) => f === 'fab_quick_actions');
    render(<QuickActionsFab />);
    expect(screen.getByLabelText('Open Quick Actions')).toBeInTheDocument();
  });

  it('opens actions when FAB is clicked', async () => {
    const user = userEvent.setup();
    mockIsEnabled.mockImplementation((f: string) => f === 'fab_quick_actions' || f === 'credits' || f === 'vehicles');
    render(<QuickActionsFab />);

    await user.click(screen.getByLabelText('Open Quick Actions'));

    expect(screen.getByText('View Bookings')).toBeInTheDocument();
    expect(screen.getByText('Book a Spot')).toBeInTheDocument();
    expect(screen.getByText('Credits')).toBeInTheDocument();
    expect(screen.getByText('My Vehicles')).toBeInTheDocument();
  });

  it('hides credits and vehicles when those features are disabled', async () => {
    const user = userEvent.setup();
    mockIsEnabled.mockImplementation((f: string) => f === 'fab_quick_actions');
    render(<QuickActionsFab />);

    await user.click(screen.getByLabelText('Open Quick Actions'));

    expect(screen.getByText('View Bookings')).toBeInTheDocument();
    expect(screen.getByText('Book a Spot')).toBeInTheDocument();
    expect(screen.queryByText('Credits')).not.toBeInTheDocument();
    expect(screen.queryByText('My Vehicles')).not.toBeInTheDocument();
  });

  it('closes actions on Escape key', async () => {
    const user = userEvent.setup();
    mockIsEnabled.mockImplementation((f: string) => f === 'fab_quick_actions');
    render(<QuickActionsFab />);

    await user.click(screen.getByLabelText('Open Quick Actions'));
    expect(screen.getByText('View Bookings')).toBeInTheDocument();

    await user.keyboard('{Escape}');

    // After close, actions hidden
    expect(screen.queryByText('View Bookings')).not.toBeInTheDocument();
  });

  it('closes when backdrop is clicked', async () => {
    const user = userEvent.setup();
    mockIsEnabled.mockImplementation((f: string) => f === 'fab_quick_actions');
    render(<QuickActionsFab />);

    await user.click(screen.getByLabelText('Open Quick Actions'));
    expect(screen.getByText('View Bookings')).toBeInTheDocument();

    // Click the backdrop (the first div with bg-black class)
    const backdrop = document.querySelector('.bg-black\\/20');
    if (backdrop) {
      await user.click(backdrop);
      expect(screen.queryByText('View Bookings')).not.toBeInTheDocument();
    }
  });

  it('closes when an action link is clicked', async () => {
    const user = userEvent.setup();
    mockIsEnabled.mockImplementation((f: string) => f === 'fab_quick_actions');
    render(<QuickActionsFab />);

    await user.click(screen.getByLabelText('Open Quick Actions'));
    const bookLink = screen.getByLabelText('Book a Spot');
    await user.click(bookLink);

    expect(screen.queryByText('View Bookings')).not.toBeInTheDocument();
  });

  it('shows correct link hrefs', async () => {
    const user = userEvent.setup();
    mockIsEnabled.mockImplementation((f: string) => f === 'fab_quick_actions');
    render(<QuickActionsFab />);

    await user.click(screen.getByLabelText('Open Quick Actions'));

    expect(screen.getByLabelText('Book a Spot')).toHaveAttribute('href', '/book');
    expect(screen.getByLabelText('View Bookings')).toHaveAttribute('href', '/bookings');
  });
});
