import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

// ── Mocks ──

const mockGetBookings = vi.fn();
const mockGetUserStats = vi.fn();

vi.mock('react-router-dom', () => ({
  Link: ({ to, children, ...props }: any) => <a href={to} {...props}>{children}</a>,
}));

vi.mock('../context/AuthContext', () => ({
  useAuth: () => ({
    user: {
      id: 'u-1',
      username: 'florian',
      name: 'Florian Test',
      email: 'f@test.com',
      role: 'admin',
      credits_balance: 7,
      credits_monthly_quota: 10,
    },
  }),
}));

vi.mock('../api/client', () => ({
  api: {
    getBookings: (...args: any[]) => mockGetBookings(...args),
    getUserStats: (...args: any[]) => mockGetUserStats(...args),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => {
      const map: Record<string, string> = {
        'dashboard.morning': 'Morning',
        'dashboard.afternoon': 'Afternoon',
        'dashboard.evening': 'Evening',
        'dashboard.greeting': `Good ${opts?.timeOfDay}, ${opts?.name}`,
        'dashboard.activeBookings': 'Active Bookings',
        'dashboard.creditsLeft': 'Credits Left',
        'dashboard.thisMonth': 'This Month',
        'dashboard.nextBooking': 'Next Booking',
        'dashboard.noActiveBookings': 'No active bookings',
        'dashboard.bookSpot': 'Book a Spot',
        'dashboard.quickActions': 'Quick Actions',
        'dashboard.myVehicles': 'My Vehicles',
        'dashboard.viewBookings': 'View Bookings',
        'dashboard.slot': 'Slot',
        'nav.bookings': 'Bookings',
        'nav.credits': 'Credits',
        'bookings.statusActive': 'Active',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, variants, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  CalendarCheck: (props: any) => <span data-testid="icon-calendar-check" {...props} />,
  Car: (props: any) => <span data-testid="icon-car" {...props} />,
  Coins: (props: any) => <span data-testid="icon-coins" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  CalendarPlus: (props: any) => <span data-testid="icon-calendar-plus" {...props} />,
  ArrowRight: (props: any) => <span data-testid="icon-arrow-right" {...props} />,
  TrendUp: (props: any) => <span data-testid="icon-trend-up" {...props} />,
  MapPin: (props: any) => <span data-testid="icon-map-pin" {...props} />,
}));

vi.mock('../components/Skeleton', () => ({
  DashboardSkeleton: () => <div data-testid="dashboard-skeleton">Loading...</div>,
}));

vi.mock('../constants/animations', () => ({
  staggerSlow: { hidden: {}, show: {} },
  fadeUp: { hidden: {}, show: {} },
}));

vi.mock('../hooks/useWebSocket', () => ({
  useWebSocket: () => ({ connected: false, lastEvent: null }),
}));

vi.mock('../components/SimpleChart', () => ({
  BarChart: ({ data }: any) => <div data-testid="bar-chart">{data?.length} bars</div>,
}));

import { DashboardPage } from './Dashboard';

describe('DashboardPage', () => {
  beforeEach(() => {
    mockGetBookings.mockClear();
    mockGetUserStats.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading skeleton initially', () => {
    mockGetBookings.mockReturnValue(new Promise(() => {}));
    mockGetUserStats.mockReturnValue(new Promise(() => {}));

    render(<DashboardPage />);
    expect(screen.getByTestId('dashboard-skeleton')).toBeInTheDocument();
  });

  it('renders greeting with user name after loading', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetUserStats.mockResolvedValue({
      success: true,
      data: { total_bookings: 10, bookings_this_month: 3, homeoffice_days_this_month: 2, avg_duration_minutes: 60 },
    });

    render(<DashboardPage />);

    await waitFor(() => {
      expect(screen.getByText(/Florian/)).toBeInTheDocument();
    });
  });

  it('renders stat cards', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetUserStats.mockResolvedValue({
      success: true,
      data: { total_bookings: 10, bookings_this_month: 3, homeoffice_days_this_month: 2, avg_duration_minutes: 60 },
    });

    render(<DashboardPage />);

    await waitFor(() => {
      // "Active Bookings" appears as both stat card label and section heading
      expect(screen.getAllByText('Active Bookings').length).toBeGreaterThanOrEqual(2);
    });
    expect(screen.getByText('Credits Left')).toBeInTheDocument();
    // "This Month" appears as stat card label and chart section heading
    expect(screen.getAllByText('This Month').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('Next Booking')).toBeInTheDocument();
  });

  it('shows empty state when no active bookings', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetUserStats.mockResolvedValue({ success: true, data: null });

    render(<DashboardPage />);

    await waitFor(() => {
      // "No active bookings" appears in both empty booking list and empty chart section
      expect(screen.getAllByText('No active bookings').length).toBeGreaterThanOrEqual(1);
    });
    // "Book a Spot" appears both in empty state and quick actions
    expect(screen.getAllByText('Book a Spot').length).toBeGreaterThanOrEqual(1);
  });

  it('shows active bookings when present', async () => {
    mockGetBookings.mockResolvedValue({
      success: true,
      data: [
        {
          id: 'b-1',
          user_id: 'u-1',
          lot_id: 'l-1',
          slot_id: 's-1',
          lot_name: 'Garage Alpha',
          slot_number: 'A1',
          vehicle_plate: 'M-AB 123',
          start_time: new Date().toISOString(),
          end_time: new Date(Date.now() + 3600000).toISOString(),
          status: 'active',
        },
      ],
    });
    mockGetUserStats.mockResolvedValue({ success: true, data: null });

    render(<DashboardPage />);

    await waitFor(() => {
      expect(screen.getByText('Garage Alpha')).toBeInTheDocument();
    });
    // Slot number appears in both the badge and inline
    expect(screen.getAllByText('A1').length).toBeGreaterThanOrEqual(1);
    // Vehicle plate is inside a div with other text, use regex
    expect(screen.getByText(/M-AB 123/)).toBeInTheDocument();
    // Active status badge
    expect(screen.getAllByText('Active').length).toBeGreaterThanOrEqual(1);
  });

  it('renders quick actions with links', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetUserStats.mockResolvedValue({ success: true, data: null });

    render(<DashboardPage />);

    await waitFor(() => {
      expect(screen.getByText('Quick Actions')).toBeInTheDocument();
    });
    // Book a Spot appears multiple times (empty state + quick action)
    expect(screen.getAllByText('Book a Spot').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('My Vehicles')).toBeInTheDocument();
    expect(screen.getByText('View Bookings')).toBeInTheDocument();
    expect(screen.getByText('Credits')).toBeInTheDocument();
  });

  it('renders bookings link pointing to /bookings', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetUserStats.mockResolvedValue({ success: true, data: null });

    render(<DashboardPage />);

    await waitFor(() => {
      expect(screen.getByText('Bookings')).toBeInTheDocument();
    });
    expect(screen.getByText('Bookings').closest('a')).toHaveAttribute('href', '/bookings');
  });
});
