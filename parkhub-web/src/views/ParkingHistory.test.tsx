import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockGetBookingHistory = vi.fn();
const mockGetBookingStats = vi.fn();
const mockGetLots = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    getBookingHistory: (...args: any[]) => mockGetBookingHistory(...args),
    getBookingStats: (...args: any[]) => mockGetBookingStats(...args),
    getLots: (...args: any[]) => mockGetLots(...args),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => {
      const map: Record<string, string> = {
        'history.title': 'Parking History',
        'history.subtitle': 'View your past bookings and parking statistics',
        'history.help': 'Track your parking history.',
        'history.totalBookings': 'Total Bookings',
        'history.favoriteLot': 'Favorite Lot',
        'history.avgDuration': 'Avg. Duration',
        'history.creditsSpent': 'Credits Spent',
        'history.monthlyTrend': 'Monthly Trend',
        'history.busiestDay': 'Busiest Day',
        'history.busiestDayDesc': 'Day with the most bookings',
        'history.filters': 'Filters',
        'history.filterLot': 'Filter by lot',
        'history.allLots': 'All lots',
        'history.dateFrom': 'From date',
        'history.dateTo': 'To date',
        'history.noHistory': 'No booking history yet',
        'history.slot': 'Slot',
        'history.showing': `${opts?.from}-${opts?.to} of ${opts?.total}`,
        'history.prevPage': 'Previous page',
        'history.nextPage': 'Next page',
        'history.monday': 'Monday',
        'history.status.completed': 'Completed',
        'history.status.cancelled': 'Cancelled',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, variants, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  ClockCounterClockwise: (props: any) => <span data-testid="icon-history" {...props} />,
  Star: (props: any) => <span data-testid="icon-star" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  TrendUp: (props: any) => <span data-testid="icon-trend-up" {...props} />,
  CalendarBlank: (props: any) => <span data-testid="icon-calendar" {...props} />,
  FunnelSimple: (props: any) => <span data-testid="icon-filter" {...props} />,
  CaretLeft: (props: any) => <span data-testid="icon-left" {...props} />,
  CaretRight: (props: any) => <span data-testid="icon-right" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  Coins: (props: any) => <span data-testid="icon-coins" {...props} />,
}));

vi.mock('../components/OnboardingHint', () => ({
  OnboardingHint: ({ text }: any) => <span data-testid="onboarding-hint">{text}</span>,
}));

vi.mock('../constants/animations', () => ({
  stagger: {},
  fadeUp: {},
}));

import { ParkingHistoryPage } from './ParkingHistory';

const makeStats = () => ({
  total_bookings: 42,
  favorite_lot: 'Garage Alpha',
  avg_duration_minutes: 120,
  busiest_day: 'Monday',
  credits_spent: 150,
  monthly_trend: [
    { month: '2025-10', bookings: 5 },
    { month: '2025-11', bookings: 8 },
    { month: '2025-12', bookings: 3 },
    { month: '2026-01', bookings: 10 },
    { month: '2026-02', bookings: 7 },
    { month: '2026-03', bookings: 9 },
  ],
});

const makeBooking = (id: string, status = 'completed') => ({
  id,
  user_id: 'u1',
  lot_id: 'lot-1',
  slot_id: 's1',
  lot_name: 'Garage Alpha',
  slot_number: '12',
  start_time: '2026-03-10T08:00:00Z',
  end_time: '2026-03-10T10:00:00Z',
  status,
  total_price: 5,
  currency: 'EUR',
});

describe('ParkingHistoryPage', () => {
  beforeEach(() => {
    mockGetBookingHistory.mockClear();
    mockGetBookingStats.mockClear();
    mockGetLots.mockClear();

    mockGetLots.mockResolvedValue({ success: true, data: [{ id: 'lot-1', name: 'Garage Alpha', total_slots: 10, available_slots: 5, status: 'open' }] });
    mockGetBookingStats.mockResolvedValue({ success: true, data: makeStats() });
    mockGetBookingHistory.mockResolvedValue({
      success: true,
      data: {
        items: [makeBooking('b1'), makeBooking('b2', 'cancelled')],
        page: 1,
        per_page: 10,
        total: 2,
        total_pages: 1,
      },
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the page title', async () => {
    render(<ParkingHistoryPage />);
    await waitFor(() => {
      expect(screen.getByText('Parking History')).toBeInTheDocument();
    });
  });

  it('displays stats cards', async () => {
    render(<ParkingHistoryPage />);
    await waitFor(() => {
      expect(screen.getByText('42')).toBeInTheDocument();
      expect(screen.getByText('150')).toBeInTheDocument();
      expect(screen.getByText('Total Bookings')).toBeInTheDocument();
      expect(screen.getByText('Credits Spent')).toBeInTheDocument();
    });
  });

  it('shows booking history items', async () => {
    render(<ParkingHistoryPage />);
    await waitFor(() => {
      expect(screen.getByText('Completed')).toBeInTheDocument();
      expect(screen.getByText('Cancelled')).toBeInTheDocument();
    });
  });

  it('shows empty state when no bookings', async () => {
    mockGetBookingHistory.mockResolvedValue({
      success: true,
      data: { items: [], page: 1, per_page: 10, total: 0, total_pages: 0 },
    });
    render(<ParkingHistoryPage />);
    await waitFor(() => {
      expect(screen.getByText('No booking history yet')).toBeInTheDocument();
    });
  });

  it('renders filter controls', async () => {
    render(<ParkingHistoryPage />);
    await waitFor(() => {
      expect(screen.getByText('Filters')).toBeInTheDocument();
      expect(screen.getByLabelText('Filter by lot')).toBeInTheDocument();
    });
  });

  it('calls API with correct params on mount', async () => {
    render(<ParkingHistoryPage />);
    await waitFor(() => {
      expect(mockGetBookingHistory).toHaveBeenCalledWith({
        lot_id: undefined,
        from: undefined,
        to: undefined,
        page: 1,
        per_page: 10,
      });
      expect(mockGetBookingStats).toHaveBeenCalled();
      expect(mockGetLots).toHaveBeenCalled();
    });
  });
});
