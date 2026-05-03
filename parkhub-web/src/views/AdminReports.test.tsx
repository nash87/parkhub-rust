import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

// ── Mocks ──

const mockAdminStats = vi.fn();
const mockAdminBookings = vi.fn();
const mockGetLots = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    adminStats: (...args: any[]) => mockAdminStats(...args),
    adminBookings: (...args: any[]) => mockAdminBookings(...args),
    getLots: (...args: any[]) => mockGetLots(...args),
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
  SpinnerGapIcon: (props: any) => <span data-testid="icon-spinner" {...props} />,
  UsersIcon: (props: any) => <span data-testid="icon-users" {...props} />,
  BuildingsIcon: (props: any) => <span data-testid="icon-buildings" {...props} />,
  CalendarCheckIcon: (props: any) => <span data-testid="icon-calendar" {...props} />,
  LightningIcon: (props: any) => <span data-testid="icon-lightning" {...props} />,
}));

vi.mock('../components/SimpleChart', () => ({
  BarChart: ({ data }: any) => <div data-testid="bar-chart">{data?.length} bars</div>,
  DonutChart: ({ slices }: any) => <div data-testid="donut-chart">{slices?.length} slices</div>,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'common.loading': 'Loading',
        'admin.reports': 'Reports',
        'admin.totalUsers': 'Total Users',
        'admin.totalLots': 'Total Lots',
        'admin.totalBookings': 'Total Bookings',
        'admin.activeBookings': 'Active Bookings',
        'admin.overview': 'Overview',
        'admin.utilizationRate': 'Utilization Rate',
        'admin.avgBookingsPerUser': 'Avg. Bookings per User',
        'admin.activeBookingRate': 'Active Booking Rate',
        'admin.bookingsThisWeek': 'Bookings This Week',
        'admin.lotOccupancy': 'Lot Occupancy',
        'heatmap.title': 'Occupancy Heatmap',
        'heatmap.subtitle': 'Average hourly occupancy by day of week (last 30 days)',
        'reports.weekdays.mon': 'Mon',
        'reports.weekdays.tue': 'Tue',
        'reports.weekdays.wed': 'Wed',
        'reports.weekdays.thu': 'Thu',
        'reports.weekdays.fri': 'Fri',
        'reports.weekdays.sat': 'Sat',
        'reports.weekdays.sun': 'Sun',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('../components/ExportButton', () => ({
  ExportButton: () => <button data-testid="export-button">Export</button>,
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
    mockAdminBookings.mockClear();
    mockGetLots.mockClear();
    mockAdminBookings.mockResolvedValue({
      success: true,
      data: { items: [], page: 1, per_page: 500, total: 0, total_pages: 0 },
    });
    mockGetLots.mockResolvedValue({ success: true, data: [] });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading spinner initially', () => {
    mockAdminStats.mockReturnValue(new Promise(() => {}));
    mockAdminBookings.mockReturnValue(new Promise(() => {}));
    render(<AdminReportsPage />);
    expect(screen.getByTestId('icon-spinner')).toBeInTheDocument();
  });

  it('renders Reports heading after loading', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 50, total_lots: 3, total_slots: 60, total_bookings: 200, active_bookings: 15, occupancy_percent: 25 },
    });

    render(<AdminReportsPage />);

    await waitFor(() => {
      expect(screen.getByText('Reports')).toBeInTheDocument();
    });
  });

  it('renders stat cards with correct values', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 50, total_lots: 3, total_slots: 60, total_bookings: 200, active_bookings: 15, occupancy_percent: 25 },
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
      data: { total_users: 50, total_lots: 3, total_slots: 60, total_bookings: 200, active_bookings: 15, occupancy_percent: 25 },
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
      data: { total_users: 10, total_lots: 2, total_slots: 40, total_bookings: 50, active_bookings: 5, occupancy_percent: 12.5 },
    });
    mockAdminBookings.mockResolvedValue({
      success: true,
      data: {
        items: [
          {
            id: 'b1',
            user_id: 'u1',
            lot_id: 'l1',
            slot_id: 's1',
            lot_name: 'Lot A',
            slot_number: 'S01',
            start_time: '2026-04-20T08:00:00Z',
            end_time: '2026-04-20T17:00:00Z',
            status: 'confirmed',
            created_at: '2026-04-01T12:00:00Z',
          },
        ],
        page: 1,
        per_page: 500,
        total: 1,
        total_pages: 1,
      },
    });

    render(<AdminReportsPage />);

    await waitFor(() => {
      expect(screen.getByTestId('bar-chart')).toBeInTheDocument();
    });
  });

  it('renders donut chart when lots exist', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 10, total_lots: 3, total_slots: 65, total_bookings: 50, active_bookings: 5, occupancy_percent: 8 },
    });
    mockGetLots.mockResolvedValue({
      success: true,
      data: [
        { id: 'l1', name: 'Lot A', total_slots: 20, available_slots: 10, status: 'open' },
        { id: 'l2', name: 'Lot B', total_slots: 30, available_slots: 5, status: 'open' },
        { id: 'l3', name: 'Lot C', total_slots: 15, available_slots: 15, status: 'open' },
      ],
    });

    render(<AdminReportsPage />);

    await waitFor(() => {
      expect(screen.getByTestId('donut-chart')).toBeInTheDocument();
    });
  });

  it('renders occupancy heatmap section', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 10, total_lots: 2, total_slots: 40, total_bookings: 50, active_bookings: 5, occupancy_percent: 12.5 },
    });
    mockAdminBookings.mockResolvedValue({
      success: true,
      data: {
        items: [
          {
            id: 'b1',
            user_id: 'u1',
            lot_id: 'l1',
            slot_id: 's1',
            lot_name: 'Lot A',
            slot_number: 'S01',
            start_time: '2026-04-20T08:00:00Z',
            end_time: '2026-04-20T17:00:00Z',
            status: 'confirmed',
            created_at: '2026-04-19T12:00:00Z',
          },
          {
            id: 'b2',
            user_id: 'u1',
            lot_id: 'l1',
            slot_id: 's2',
            lot_name: 'Lot A',
            slot_number: 'S02',
            start_time: '2026-04-20T09:00:00Z',
            end_time: '2026-04-20T17:00:00Z',
            status: 'pending',
            created_at: '2026-04-19T12:00:00Z',
          },
          {
            id: 'b3',
            user_id: 'u1',
            lot_id: 'l1',
            slot_id: 's3',
            lot_name: 'Lot A',
            slot_number: 'S03',
            start_time: '2026-04-20T10:00:00Z',
            end_time: '2026-04-20T17:00:00Z',
            status: 'expired',
            created_at: '2026-04-19T12:00:00Z',
          },
        ],
        page: 1,
        per_page: 500,
        total: 3,
        total_pages: 1,
      },
    });

    render(<AdminReportsPage />);

    await waitFor(() => {
      expect(screen.getByTestId('occupancy-heatmap')).toBeInTheDocument();
    });
    expect(mockAdminBookings).toHaveBeenCalledWith(500);
    expect(screen.getByText('Occupancy Heatmap')).toBeInTheDocument();
    expect(screen.getByText('heatmap: 1 bookings, 40 slots')).toBeInTheDocument();
  });

  it('uses backend occupancy percent for utilization rate', async () => {
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 10, total_lots: 5, total_slots: 100, total_bookings: 100, active_bookings: 3, occupancy_percent: 3 },
    });

    render(<AdminReportsPage />);

    await waitFor(() => {
      expect(screen.getAllByText('3%').length).toBeGreaterThanOrEqual(1);
    });
  });
});
