import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

// ── Mocks ──

const mockAdminStats = vi.fn();
const mockGetBookings = vi.fn();
const mockGetLots = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    adminStats: (...args: any[]) => mockAdminStats(...args),
    getBookings: (...args: any[]) => mockGetBookings(...args),
    getLots: (...args: any[]) => mockGetLots(...args),
  },
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, variants, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
}));

vi.mock('@phosphor-icons/react', () => ({
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  CurrencyEur: (props: any) => <span data-testid="icon-eur" {...props} />,
  ChartDonut: (props: any) => <span data-testid="icon-donut" {...props} />,
  UsersThree: (props: any) => <span data-testid="icon-users" {...props} />,
  Timer: (props: any) => <span data-testid="icon-timer" {...props} />,
  ArrowUp: (props: any) => <span data-testid="icon-arrow-up" {...props} />,
  TrendUp: (props: any) => <span data-testid="icon-trend-up" {...props} />,
}));

vi.mock('../components/SimpleChart', () => ({
  BarChart: ({ data }: any) => <div data-testid="bar-chart">{data?.length} bars</div>,
}));

vi.mock('../components/OccupancyHeatmap', () => ({
  OccupancyHeatmap: ({ bookings, totalSlots }: any) => (
    <div data-testid="occupancy-heatmap">heatmap: {bookings?.length ?? 0} bookings, {totalSlots} slots</div>
  ),
}));

vi.mock('../components/AnimatedCounter', () => ({
  AnimatedCounter: ({ value }: any) => <span>{value}</span>,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'common.loading': 'Loading',
        'common.noData': 'No data',
        'analytics.title': 'Analytics Overview',
        'analytics.liveStatus': 'Live System Status',
        'analytics.totalRevenue': 'Total Revenue',
        'analytics.occupancyRate': 'Current Occupancy',
        'analytics.activeUsers': 'Active Users',
        'analytics.avgDuration': 'Average Stay',
        'analytics.peakUsage': 'Peak Usage Hours',
        'analytics.peakUsageSubtitle': 'Weekly occupancy trends',
        'analytics.revenueTrend': 'Revenue Trend',
        'analytics.revenueTrendSubtitle': 'Past 30 days',
        'analytics.topLots': 'Top Parking Lots',
        'analytics.topLotsSubtitle': 'Performance ranking',
        'analytics.capacity': 'Capacity',
        'analytics.dailyRev': 'Daily Rev.',
        'analytics.parking': 'Parking',
        'heatmap.title': 'Occupancy Heatmap',
        'heatmap.subtitle': 'Average hourly occupancy',
      };
      return map[key] || (typeof fallback === 'string' ? fallback : key);
    },
  }),
}));

vi.mock('../constants/animations', () => ({
  staggerSlow: { hidden: {}, show: {} },
  fadeUp: { hidden: {}, show: {} },
}));

import { AdminAnalyticsPage } from './AdminAnalytics';

describe('AdminAnalyticsPage', () => {
  beforeEach(() => {
    mockAdminStats.mockClear();
    mockGetBookings.mockClear();
    mockGetLots.mockClear();
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetLots.mockResolvedValue({ success: true, data: [] });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading spinner initially', () => {
    mockAdminStats.mockReturnValue(new Promise(() => {}));
    mockGetBookings.mockReturnValue(new Promise(() => {}));
    mockGetLots.mockReturnValue(new Promise(() => {}));
    render(<AdminAnalyticsPage />);
    expect(screen.getByTestId('icon-spinner')).toBeInTheDocument();
  });

  it('renders analytics heading after loading', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 50, total_lots: 3, total_bookings: 200, active_bookings: 15 },
    });

    render(<AdminAnalyticsPage />);

    await waitFor(() => {
      expect(screen.getByText('Analytics Overview')).toBeInTheDocument();
    });
  });

  it('renders 4 KPI cards', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 234, total_lots: 5, total_bookings: 500, active_bookings: 20 },
    });

    render(<AdminAnalyticsPage />);

    await waitFor(() => {
      expect(screen.getByText('Total Revenue')).toBeInTheDocument();
    });
    expect(screen.getByText('Current Occupancy')).toBeInTheDocument();
    expect(screen.getByText('Active Users')).toBeInTheDocument();
    expect(screen.getByText('Average Stay')).toBeInTheDocument();
  });

  it('renders occupancy heatmap', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 10, total_lots: 2, total_bookings: 50, active_bookings: 5 },
    });

    render(<AdminAnalyticsPage />);

    await waitFor(() => {
      expect(screen.getByTestId('occupancy-heatmap')).toBeInTheDocument();
    });
    expect(screen.getByText('Peak Usage Hours')).toBeInTheDocument();
  });

  it('renders revenue trend chart', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 10, total_lots: 2, total_bookings: 50, active_bookings: 5 },
    });

    render(<AdminAnalyticsPage />);

    await waitFor(() => {
      expect(screen.getByTestId('bar-chart')).toBeInTheDocument();
    });
    expect(screen.getByText('Revenue Trend')).toBeInTheDocument();
  });

  it('renders top parking lots section', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 10, total_lots: 3, total_bookings: 50, active_bookings: 5 },
    });
    mockGetLots.mockResolvedValue({
      success: true,
      data: [
        { id: 'l1', name: 'Central Plaza', total_slots: 500, available_slots: 88, status: 'open' },
        { id: 'l2', name: 'Marina Waterfront', total_slots: 300, available_slots: 55, status: 'open' },
      ],
    });

    render(<AdminAnalyticsPage />);

    await waitFor(() => {
      expect(screen.getByText('Top Parking Lots')).toBeInTheDocument();
    });
    expect(screen.getByText('Central Plaza')).toBeInTheDocument();
    expect(screen.getByText('Marina Waterfront')).toBeInTheDocument();
  });

  it('shows live system status badge', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 10, total_lots: 2, total_bookings: 50, active_bookings: 5 },
    });

    render(<AdminAnalyticsPage />);

    await waitFor(() => {
      expect(screen.getByText('Live System Status')).toBeInTheDocument();
    });
  });
});
