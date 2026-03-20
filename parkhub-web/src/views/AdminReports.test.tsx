import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

// ── Mocks ──

const mockAdminStats = vi.fn();
const mockGetBookings = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    adminStats: (...args: any[]) => mockAdminStats(...args),
    getBookings: (...args: any[]) => mockGetBookings(...args),
  },
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
}));

vi.mock('@phosphor-icons/react', () => ({
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  Users: (props: any) => <span data-testid="icon-users" {...props} />,
  Buildings: (props: any) => <span data-testid="icon-buildings" {...props} />,
  CalendarCheck: (props: any) => <span data-testid="icon-calendar" {...props} />,
  Lightning: (props: any) => <span data-testid="icon-lightning" {...props} />,
}));

vi.mock('../components/SimpleChart', () => ({
  BarChart: ({ data }: any) => <div data-testid="bar-chart">{data?.length} bars</div>,
  DonutChart: ({ slices }: any) => <div data-testid="donut-chart">{slices?.length} slices</div>,
}));

vi.mock('../components/OccupancyHeatmap', () => ({
  OccupancyHeatmap: ({ bookings, totalSlots }: any) => (
    <div data-testid="occupancy-heatmap">heatmap: {bookings?.length ?? 0} bookings, {totalSlots} slots</div>
  ),
}));

import { AdminReportsPage } from './AdminReports';

describe('AdminReportsPage', () => {
  beforeEach(() => {
    mockAdminStats.mockClear();
    mockGetBookings.mockClear();
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading spinner initially', () => {
    mockAdminStats.mockReturnValue(new Promise(() => {}));
    mockGetBookings.mockReturnValue(new Promise(() => {}));
    render(<AdminReportsPage />);
    expect(screen.getByTestId('icon-spinner')).toBeInTheDocument();
  });

  it('renders Reports heading after loading', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 50, total_lots: 3, total_bookings: 200, active_bookings: 15 },
    });

    render(<AdminReportsPage />);

    await waitFor(() => {
      expect(screen.getByText('Reports')).toBeInTheDocument();
    });
  });

  it('renders stat cards with correct values', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 50, total_lots: 3, total_bookings: 200, active_bookings: 15 },
    });

    render(<AdminReportsPage />);

    await waitFor(() => {
      expect(screen.getByText('Total Users')).toBeInTheDocument();
    });
    expect(screen.getByText('50')).toBeInTheDocument();
    expect(screen.getByText('Total Lots')).toBeInTheDocument();
    expect(screen.getByText('3')).toBeInTheDocument();
    expect(screen.getByText('Total Bookings')).toBeInTheDocument();
    expect(screen.getByText('200')).toBeInTheDocument();
    expect(screen.getByText('Active Bookings')).toBeInTheDocument();
    expect(screen.getByText('15')).toBeInTheDocument();
  });

  it('renders overview summary section', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 50, total_lots: 3, total_bookings: 200, active_bookings: 15 },
    });

    render(<AdminReportsPage />);

    await waitFor(() => {
      expect(screen.getByText('Overview')).toBeInTheDocument();
    });
    expect(screen.getByText('Utilization Rate')).toBeInTheDocument();
    expect(screen.getByText('Avg. Bookings per User')).toBeInTheDocument();
    expect(screen.getByText('Active Booking Rate')).toBeInTheDocument();
  });

  it('renders bar chart component', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 10, total_lots: 2, total_bookings: 50, active_bookings: 5 },
    });

    render(<AdminReportsPage />);

    await waitFor(() => {
      expect(screen.getByTestId('bar-chart')).toBeInTheDocument();
    });
  });

  it('renders donut chart when lots exist', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 10, total_lots: 3, total_bookings: 50, active_bookings: 5 },
    });

    render(<AdminReportsPage />);

    await waitFor(() => {
      expect(screen.getByTestId('donut-chart')).toBeInTheDocument();
    });
  });

  it('renders occupancy heatmap section', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 10, total_lots: 2, total_bookings: 50, active_bookings: 5 },
    });

    render(<AdminReportsPage />);

    await waitFor(() => {
      expect(screen.getByTestId('occupancy-heatmap')).toBeInTheDocument();
    });
    expect(screen.getByText('Occupancy Heatmap')).toBeInTheDocument();
  });

  it('calculates correct utilization rate', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 10, total_lots: 5, total_bookings: 100, active_bookings: 3 },
    });

    render(<AdminReportsPage />);

    await waitFor(() => {
      // 3 / 5 = 60%
      expect(screen.getByText('60%')).toBeInTheDocument();
    });
  });
});
