import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent, act } from '@testing-library/react';

const { mockNavigate, mockLocation } = vi.hoisted(() => ({
  mockNavigate: vi.fn(),
  mockLocation: { pathname: '/' },
}));

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
  useNavigate: () => mockNavigate,
  useLocation: () => mockLocation,
  Link: ({ to, children, ...rest }: any) => (
    <a href={typeof to === 'string' ? to : '#'} {...rest}>
      {children}
    </a>
  ),
}));

vi.mock('@phosphor-icons/react', () => ({
  WifiSlashIcon: (props: any) => <span data-testid="icon-wifi-slash" {...props} />,
  ArrowDownIcon: (props: any) => <span data-testid="icon-arrow-down" {...props} />,
  HouseIcon: (props: any) => <span data-testid="icon-house" {...props} />,
  CalendarBlankIcon: (props: any) => <span data-testid="icon-calendar" {...props} />,
  CarIcon: (props: any) => <span data-testid="icon-car" {...props} />,
  UserIcon: (props: any) => <span data-testid="icon-user" {...props} />,
}));

import { OfflineIndicator, CachedBookingCard, BottomNavBar, PullToRefresh } from './PWAEnhanced';

describe('OfflineIndicator', () => {
  beforeEach(() => {
    mockNavigate.mockReset();
    mockLocation.pathname = '/';
  });

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
    mockNavigate.mockReset();
    mockLocation.pathname = '/';
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
  beforeEach(() => {
    mockNavigate.mockReset();
    mockLocation.pathname = '/';
  });

  it('renders all navigation tabs', () => {
    render(<BottomNavBar />);
    expect(screen.getByText('Dashboard')).toBeDefined();
    expect(screen.getByText('Book')).toBeDefined();
    expect(screen.getByText('Bookings')).toBeDefined();
    expect(screen.getByText('Vehicles')).toBeDefined();
    expect(screen.getByText('Profile')).toBeDefined();
  });

  it('renders 5 navigation links (not buttons)', () => {
    // a11y: tabs must be real <a> links so cmd-click / middle-click open
    // in a new tab, right-click opens the context menu, status bar shows
    // destination URL, and screen readers announce them as links.
    render(<BottomNavBar />);
    const links = screen.getAllByRole('link');
    expect(links.length).toBe(5);
    // No buttons should be used for navigation in the bottom bar.
    expect(screen.queryAllByRole('button')).toHaveLength(0);
  });

  it('exposes the route path in the href so middle/cmd-click open in a new tab', async () => {
    render(<BottomNavBar />);
    expect(screen.getByLabelText('Vehicles').getAttribute('href')).toBe('/vehicles');
    expect(screen.getByLabelText('Dashboard').getAttribute('href')).toBe('/');
  });

  it('marks nested booking routes as active', () => {
    mockLocation.pathname = '/bookings/today';
    render(<BottomNavBar />);

    expect(screen.getByLabelText('Bookings')).toHaveAttribute('aria-current', 'page');
  });
});

describe('PullToRefresh', () => {
  beforeEach(() => {
    mockNavigate.mockReset();
    mockLocation.pathname = '/';
  });

  it('renders children', () => {
    render(
      <PullToRefresh>
        <div>Content</div>
      </PullToRefresh>
    );
    expect(screen.getByText('Content')).toBeDefined();
  });

  it('handles touch events', () => {
    Object.defineProperty(window, 'scrollY', { value: 0, writable: true, configurable: true });
    const { container } = render(
      <PullToRefresh>
        <div>Touch Content</div>
      </PullToRefresh>
    );
    const wrapper = container.firstChild as HTMLElement;
    // Simulate touch start
    fireEvent.touchStart(wrapper, {
      touches: [{ clientY: 0 }],
    });
    expect(screen.getByText('Touch Content')).toBeDefined();
  });

  it('shows release state and reloads after crossing the threshold', async () => {
    const onRefresh = vi.fn();
    Object.defineProperty(window, 'scrollY', { value: 0, writable: true, configurable: true });

    const { container } = render(
      <PullToRefresh onRefresh={onRefresh}>
        <div>Refreshable Content</div>
      </PullToRefresh>
    );

    const wrapper = container.firstChild as HTMLElement;
    fireEvent.touchStart(wrapper, {
      touches: [{ clientY: 0 }],
    });

    fireEvent.touchMove(document, {
      touches: [{ clientY: 120 }],
    });

    await waitFor(() => {
      expect(screen.getByText('Release to refresh')).toBeInTheDocument();
    });

    fireEvent.touchEnd(document);

    expect(onRefresh).toHaveBeenCalledOnce();
  });
});

describe('OfflineIndicator - events', () => {
  it('responds to offline/online events', async () => {
    Object.defineProperty(navigator, 'onLine', { value: true, writable: true });
    render(<OfflineIndicator />);
    // Go offline
    await act(() => { window.dispatchEvent(new Event('offline')); });
    expect(screen.getByText('You are offline. Some features may be unavailable.')).toBeDefined();
    // Go back online
    await act(() => { window.dispatchEvent(new Event('online')); });
    expect(screen.queryByText('You are offline. Some features may be unavailable.')).toBeNull();
  });
});

describe('CachedBookingCard - online data refresh', () => {
  it('fetches offline data when online', async () => {
    Object.defineProperty(navigator, 'onLine', { value: true, writable: true });
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({
        success: true,
        data: {
          next_booking: { id: 'b-2', lot_name: 'Lot X', slot_label: 'B-2', date: '2026-04-20', start_time: '09:00', end_time: '18:00' },
        },
      }),
    });
    render(<CachedBookingCard />);
    await waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalledWith('/api/v1/pwa/offline-data');
    });
  });

  it('handles invalid cached data gracefully', async () => {
    Object.defineProperty(navigator, 'onLine', { value: false, writable: true });
    localStorage.setItem('parkhub_offline_data', 'not json');
    const { container } = render(<CachedBookingCard />);
    expect(container.innerHTML).toBe('');
  });

  it('handles offline data without next_booking', async () => {
    Object.defineProperty(navigator, 'onLine', { value: false, writable: true });
    localStorage.setItem('parkhub_offline_data', JSON.stringify({}));
    const { container } = render(<CachedBookingCard />);
    expect(container.innerHTML).toBe('');
  });

  it('handles fetch error for offline data', async () => {
    Object.defineProperty(navigator, 'onLine', { value: true, writable: true });
    globalThis.fetch = vi.fn().mockRejectedValue(new Error('net'));
    render(<CachedBookingCard />);
    // Should not crash
  });
});

describe('BottomNavBar - active state', () => {
  it('marks current path as active', () => {
    render(<BottomNavBar />);
    const dashBtn = screen.getByLabelText('Dashboard');
    expect(dashBtn.getAttribute('aria-current')).toBe('page');
  });
});
