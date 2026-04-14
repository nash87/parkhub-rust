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
        'dashboard.live': 'Live',
        'dashboard.wsConnected': 'Live updates active',
        'dashboard.wsBookingCreated': 'New booking created',
        'dashboard.wsBookingCancelled': 'A booking was cancelled',
        'dashboard.wsOccupancyChanged': 'Occupancy updated',
        'dashboard.totalBookings': 'Total Bookings',
        'dashboard.weeklyActivityTitle': 'Weekly Activity',
        'dashboard.weeklyActivitySubtitle': 'Booking volume',
        'dashboard.period7d': '7 Days',
        'dashboard.period30d': '30 Days',
        'dashboard.liveSensorFeed': 'Live Sensor Feed',
        'dashboard.sensorFeedSubtitle': 'Real-time gate status',
        'dashboard.recentActivity': 'Recent Activity',
        'dashboard.noActivity': 'No recent activity yet',
        'dashboard.unknownVehicle': 'Unknown vehicle',
        'dashboard.entranceGateA': 'Entrance Gate A',
        'dashboard.entranceGateB': 'Entrance Gate B',
        'dashboard.exitGate': 'Exit Gate',
        'dashboard.colVehicleOwner': 'Vehicle / Location',
        'dashboard.colSlot': 'Slot No.',
        'dashboard.colCheckIn': 'Check-In Time',
        'dashboard.colDuration': 'Duration',
        'dashboard.colStatus': 'Status',
        'dashboard.statistics': 'Statistics',
        'dashboard.loadingDashboard': 'Loading dashboard',
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
  ChartLine: (props: any) => <span data-testid="icon-chart-line" {...props} />,
  Gauge: (props: any) => <span data-testid="icon-gauge" {...props} />,
  CurrencyDollar: (props: any) => <span data-testid="icon-dollar" {...props} />,
  Timer: (props: any) => <span data-testid="icon-timer" {...props} />,
  ArrowUp: (props: any) => <span data-testid="icon-arrow-up" {...props} />,
  ArrowDown: (props: any) => <span data-testid="icon-arrow-down" {...props} />,
  CircleDashed: (props: any) => <span data-testid="icon-circle-dashed" {...props} />,
}));

vi.mock('../components/KineticObservatory', () => ({
  KpiCard: ({ label, value, live, delta }: any) => (
    <div data-testid={`kpi-${String(label).toLowerCase().replace(/\s+/g, '-')}`}>
      <span>{label}</span>
      <span data-testid="kpi-value">{value}</span>
      {live && <span data-testid="live-badge">Live</span>}
      {delta && <span data-testid="delta-badge">{delta.value}{delta.suffix || '%'}</span>}
    </div>
  ),
  TrendCard: ({ title, subtitle, periods, activePeriod, onPeriodChange }: any) => (
    <section data-testid="trend-card">
      <h3>{title}</h3>
      {subtitle && <p>{subtitle}</p>}
      {periods?.map((p: any) => (
        <button key={p.key} onClick={() => onPeriodChange?.(p.key)} aria-pressed={activePeriod === p.key}>
          {p.label}
        </button>
      ))}
    </section>
  ),
  SensorFeedCard: ({ title, sensors }: any) => (
    <section data-testid="sensor-feed-card">
      <h3>{title}</h3>
      <ul>
        {sensors?.map((s: any) => (
          <li key={s.name} data-testid={`sensor-${s.name.toLowerCase().replace(/\s+/g, '-')}`}>
            {s.name} — {s.status}
          </li>
        ))}
      </ul>
    </section>
  ),
  RecentActivityCard: ({ title, rows, emptyText }: any) => (
    <section data-testid="recent-activity-card">
      <h3>{title}</h3>
      {rows?.length === 0 ? <p>{emptyText}</p> : (
        <table data-testid="recent-activity-table">
          {rows?.map((r: any) => <tr key={r.id} data-testid={`activity-row-${r.id}`}><td>{r.vehicle}</td></tr>)}
        </table>
      )}
    </section>
  ),
}));

vi.mock('../components/Skeleton', () => ({
  DashboardSkeleton: () => <div data-testid="dashboard-skeleton">Loading...</div>,
}));

vi.mock('../constants/animations', () => ({
  staggerSlow: { hidden: {}, show: {} },
  fadeUp: { hidden: {}, show: {} },
}));

const mockUseWebSocket = vi.fn().mockReturnValue({ connected: false, lastMessage: null, occupancy: {} });
vi.mock('../hooks/useWebSocket', () => ({
  useWebSocket: (...args: any[]) => mockUseWebSocket(...args),
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
    expect(screen.getByText(/Total Bookings/i)).toBeInTheDocument();
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
    expect(screen.getAllByText(/M-AB 123/).length).toBeGreaterThan(0);
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

  it('shows live indicator when WebSocket is connected', async () => {
    mockUseWebSocket.mockReturnValue({ connected: true, lastMessage: null, occupancy: {} });
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetUserStats.mockResolvedValue({ success: true, data: null });

    render(<DashboardPage />);

    await waitFor(() => {
      expect(screen.getByTestId('ws-connected-indicator')).toBeInTheDocument();
    });
    expect(screen.getByText('Live')).toBeInTheDocument();
  });

  it('hides live indicator when WebSocket is disconnected', async () => {
    mockUseWebSocket.mockReturnValue({ connected: false, lastMessage: null, occupancy: {} });
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetUserStats.mockResolvedValue({ success: true, data: null });

    render(<DashboardPage />);

    await waitFor(() => {
      expect(screen.getByText(/Good/)).toBeInTheDocument();
    });
    expect(screen.queryByTestId('ws-connected-indicator')).not.toBeInTheDocument();
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

  it('websocket handler routes all three event types without throwing', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetUserStats.mockResolvedValue({ success: true, data: null });
    let captured: ((e: any) => void) | undefined;
    mockUseWebSocket.mockImplementation((opts: any) => {
      captured = opts?.onEvent;
      return { connected: false, lastMessage: null, occupancy: {} };
    });
    render(<DashboardPage />);
    await waitFor(() => expect(screen.getByText(/Good/)).toBeInTheDocument());
    expect(() => captured?.({ event: 'booking_created', lot_id: 'l1' })).not.toThrow();
    expect(() => captured?.({ event: 'booking_cancelled', lot_id: 'l1' })).not.toThrow();
    expect(() => captured?.({ event: 'occupancy_changed', lot_id: 'l1', occupancy: 5 })).not.toThrow();
  });
});
