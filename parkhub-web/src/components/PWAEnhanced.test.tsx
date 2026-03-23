import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'pwa.offlineMessage': 'You are offline. Some features may be unavailable.',
        'pwa.nextBooking': 'Your Next Booking',
        'pwa.mobileNav': 'Mobile navigation',
        'pwa.pullToRefresh': 'Pull to refresh',
        'pwa.releaseToRefresh': 'Release to refresh',
        'nav.dashboard': 'Dashboard',
        'nav.book': 'Book',
        'nav.bookings': 'Bookings',
        'nav.vehicles': 'Vehicles',
        'nav.profile': 'Profile',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('react-router-dom', () => ({
  useNavigate: () => vi.fn(),
  useLocation: () => ({ pathname: '/' }),
}));

vi.mock('@phosphor-icons/react', () => ({
  WifiSlash: (props: any) => <span data-testid="icon-wifi-slash" {...props} />,
  ArrowDown: (props: any) => <span data-testid="icon-arrow-down" {...props} />,
  House: (props: any) => <span data-testid="icon-house" {...props} />,
  CalendarBlank: (props: any) => <span data-testid="icon-calendar" {...props} />,
  Car: (props: any) => <span data-testid="icon-car" {...props} />,
  User: (props: any) => <span data-testid="icon-user" {...props} />,
}));

import { OfflineIndicator, CachedBookingCard, BottomNavBar, PullToRefresh } from './PWAEnhanced';

describe('OfflineIndicator', () => {
  it('does not render when online', () => {
    Object.defineProperty(navigator, 'onLine', { value: true, writable: true });
    const { container } = render(<OfflineIndicator />);
    expect(container.innerHTML).toBe('');
  });

  it('renders when offline', () => {
    Object.defineProperty(navigator, 'onLine', { value: false, writable: true });
    render(<OfflineIndicator />);
    expect(screen.getByText('You are offline. Some features may be unavailable.')).toBeDefined();
  });
});

describe('CachedBookingCard', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    localStorage.clear();
  });

  it('does not render when online', () => {
    Object.defineProperty(navigator, 'onLine', { value: true, writable: true });
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: { next_booking: null, lot_info: [] } }),
    });
    const { container } = render(<CachedBookingCard />);
    expect(container.innerHTML).toBe('');
  });

  it('shows cached booking when offline', async () => {
    Object.defineProperty(navigator, 'onLine', { value: false, writable: true });
    localStorage.setItem('parkhub_offline_data', JSON.stringify({
      next_booking: {
        id: 'b-1',
        lot_name: 'Main Lot',
        slot_label: 'A-5',
        date: '2026-03-24',
        start_time: '08:00',
        end_time: '17:00',
      },
    }));

    render(<CachedBookingCard />);
    await waitFor(() => {
      expect(screen.getByText('Your Next Booking')).toBeDefined();
    });
  });
});

describe('BottomNavBar', () => {
  it('renders all navigation tabs', () => {
    render(<BottomNavBar />);
    expect(screen.getByText('Dashboard')).toBeDefined();
    expect(screen.getByText('Book')).toBeDefined();
    expect(screen.getByText('Bookings')).toBeDefined();
    expect(screen.getByText('Vehicles')).toBeDefined();
    expect(screen.getByText('Profile')).toBeDefined();
  });

  it('has 5 navigation buttons', () => {
    render(<BottomNavBar />);
    const buttons = screen.getAllByRole('button');
    expect(buttons.length).toBe(5);
  });
});

describe('PullToRefresh', () => {
  it('renders children', () => {
    render(
      <PullToRefresh>
        <div>Content</div>
      </PullToRefresh>
    );
    expect(screen.getByText('Content')).toBeDefined();
  });
});
