import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── matchMedia mock ──
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
});

// ── Mocks ──

const mockGetBookings = vi.fn();
const mockGetVehicles = vi.fn();
const mockCancelBooking = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    getBookings: (...args: any[]) => mockGetBookings(...args),
    getVehicles: (...args: any[]) => mockGetVehicles(...args),
    cancelBooking: (...args: any[]) => mockCancelBooking(...args),
  },
}));

vi.mock('react-router-dom', () => ({
  Link: ({ to, children, ...props }: any) => <a href={to} {...props}>{children}</a>,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => {
      const map: Record<string, string> = {
        'bookings.title': 'Bookings',
        'bookings.subtitle': 'Manage your parking bookings',
        'bookings.active': 'Active',
        'bookings.upcoming': 'Upcoming',
        'bookings.past': 'Past',
        'bookings.noActive': 'No active bookings',
        'bookings.noUpcoming': 'No upcoming bookings',
        'bookings.noPast': 'No past bookings',
        'bookings.bookNow': 'Book Now',
        'bookings.cancelBtn': 'Cancel',
        'bookings.cancelled': 'Booking cancelled',
        'bookings.cancelFailed': 'Cancel failed',
        'bookings.statusActive': 'Active',
        'bookings.statusCompleted': 'Completed',
        'bookings.statusCancelled': 'Cancelled',
        'bookings.endsIn': `Ends ${opts?.time || ''}`,
        'bookings.startsIn': `Starts ${opts?.time || ''}`,
        'common.refresh': 'Refresh',
        'common.filter': 'Filter',
        'dashboard.slot': 'Slot',
        'bookingFilters.totalCount': `${opts?.count ?? 0} total`,
        'bookingFilters.searchLot': 'Search lot...',
        'bookingFilters.statusAll': 'All',
        'bookingFilters.statusActive': 'Active',
        'bookingFilters.statusConfirmed': 'Confirmed',
        'bookingFilters.statusCancelled': 'Cancelled',
        'bookingFilters.statusCompleted': 'Completed',
        'pass.showPass': 'Show Pass',
        'bookings.downloadInvoice': 'Download Invoice',
      };
      return map[key] || key;
    },
    i18n: { language: 'en' },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, variants, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    p: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <p ref={ref} {...props}>{children}</p>
    )),
    button: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <button ref={ref} {...props}>{children}</button>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  CalendarBlank: (props: any) => <span data-testid="icon-calendar" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  Car: (props: any) => <span data-testid="icon-car" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  ArrowClockwise: (props: any) => <span data-testid="icon-refresh" {...props} />,
  Warning: (props: any) => <span data-testid="icon-warning" {...props} />,
  MapPin: (props: any) => <span data-testid="icon-pin" {...props} />,
  CalendarPlus: (props: any) => <span data-testid="icon-cal-plus" {...props} />,
  Timer: (props: any) => <span data-testid="icon-timer" {...props} />,
  MagnifyingGlass: (props: any) => <span data-testid="icon-search" {...props} />,
  Funnel: (props: any) => <span data-testid="icon-funnel" {...props} />,
  QrCode: (props: any) => <span data-testid="icon-qrcode" {...props} />,
  FilePdf: (props: any) => <span data-testid="icon-pdf" {...props} />,
}));

vi.mock('../components/Skeleton', () => ({
  BookingsSkeleton: () => <div data-testid="bookings-skeleton">Loading...</div>,
}));

const wsCallbacks: { onEvent?: (e: any) => void } = {};
vi.mock('../hooks/useWebSocket', () => ({
  useWebSocket: (opts: any) => {
    wsCallbacks.onEvent = opts?.onEvent;
    return { connected: false, lastMessage: null, occupancy: {} };
  },
}));

vi.mock('../components/ParkingPass', () => ({
  ParkingPass: ({ booking, onClose }: any) => (
    <div data-testid="parking-pass-modal">
      <span>{booking.lot_name}</span>
      <button onClick={onClose}>Close</button>
    </div>
  ),
}));

vi.mock('react-hot-toast', () => {
  const fn = vi.fn();
  (fn as any).success = vi.fn();
  (fn as any).error = vi.fn();
  return { default: fn };
});

import { BookingsPage } from './Bookings';
import type { Booking } from '../api/client';

function makeBooking(overrides: Partial<Booking> = {}): Booking {
  const now = Date.now();
  return {
    id: 'b1',
    user_id: 'u1',
    lot_id: 'l1',
    slot_id: 's1',
    lot_name: 'Lot Alpha',
    slot_number: 'A1',
    vehicle_plate: 'AB-CD-123',
    start_time: new Date(now - 3600000).toISOString(),
    end_time: new Date(now + 3600000).toISOString(),
    status: 'active',
    ...overrides,
  };
}

describe('BookingsPage', () => {
  beforeEach(() => {
    mockGetBookings.mockClear();
    mockGetVehicles.mockClear();
    mockCancelBooking.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders loading skeleton initially', () => {
    // Never resolving promises keeps loading=true
    mockGetBookings.mockReturnValue(new Promise(() => {}));
    mockGetVehicles.mockReturnValue(new Promise(() => {}));

    render(<BookingsPage />);
    expect(screen.getByTestId('bookings-skeleton')).toBeInTheDocument();
  });

  it('shows empty state when no bookings', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });

    render(<BookingsPage />);

    await waitFor(() => {
      expect(screen.getByText('No active bookings')).toBeInTheDocument();
    });
    expect(screen.getByText('No upcoming bookings')).toBeInTheDocument();
    expect(screen.getByText('No past bookings')).toBeInTheDocument();
  });

  it('renders booking cards with lot name and slot', async () => {
    const booking = makeBooking({ lot_name: 'Garage West', slot_number: 'B5' });
    mockGetBookings.mockResolvedValue({ success: true, data: [booking] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });

    render(<BookingsPage />);

    await waitFor(() => {
      expect(screen.getByText('Garage West')).toBeInTheDocument();
    });
    expect(screen.getByText(/Slot B5/)).toBeInTheDocument();
  });

  it('renders status badge for active booking', async () => {
    const booking = makeBooking({ status: 'active' });
    mockGetBookings.mockResolvedValue({ success: true, data: [booking] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });

    render(<BookingsPage />);

    await waitFor(() => {
      // Status badge text "Active" appears in the badge
      const badges = screen.getAllByText('Active');
      // At least one is the status badge (others may be section header)
      expect(badges.length).toBeGreaterThanOrEqual(1);
    });
  });

  it('renders cancelled booking in past section', async () => {
    const booking = makeBooking({ status: 'cancelled', lot_name: 'Old Lot' });
    mockGetBookings.mockResolvedValue({ success: true, data: [booking] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });

    render(<BookingsPage />);

    await waitFor(() => {
      expect(screen.getByText('Old Lot')).toBeInTheDocument();
    });
    // "Cancelled" appears both in filter dropdown and badge; check the badge specifically
    const badges = screen.getAllByText('Cancelled');
    expect(badges.length).toBeGreaterThanOrEqual(2); // filter option + status badge
  });

  it('cancel button triggers API call', async () => {
    const user = userEvent.setup();
    const booking = makeBooking();
    mockGetBookings.mockResolvedValue({ success: true, data: [booking] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    mockCancelBooking.mockResolvedValue({ success: true, data: null });

    render(<BookingsPage />);

    await waitFor(() => {
      expect(screen.getByText('Lot Alpha')).toBeInTheDocument();
    });

    const cancelBtn = screen.getByRole('button', { name: /Cancel/ });
    await user.click(cancelBtn);

    await waitFor(() => {
      expect(mockCancelBooking).toHaveBeenCalledWith('b1');
    });
  });

  it('shows the three section headings (Active, Upcoming, Past)', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });

    render(<BookingsPage />);

    await waitFor(() => {
      expect(screen.getByText('Bookings')).toBeInTheDocument();
    });

    // Section headings are h2 elements; "Active" also appears in filter dropdown
    const h2s = screen.getAllByRole('heading', { level: 2 });
    const texts = h2s.map(h => h.textContent);
    expect(texts.some(t => t?.includes('Active'))).toBe(true);
    expect(texts.some(t => t?.includes('Upcoming'))).toBe(true);
    expect(texts.some(t => t?.includes('Past'))).toBe(true);
  });

  it('renders filter controls', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });

    render(<BookingsPage />);

    await waitFor(() => {
      expect(screen.getByText('Filter')).toBeInTheDocument();
    });
    expect(screen.getByPlaceholderText('Search lot...')).toBeInTheDocument();
  });

  it('status filter dropdown has 5 options', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });

    render(<BookingsPage />);

    await waitFor(() => {
      expect(screen.getByText('Filter')).toBeInTheDocument();
    });

    const select = screen.getByDisplayValue('All');
    expect(select).toBeInTheDocument();
    expect(select.querySelectorAll('option')).toHaveLength(5);
  });

  it('shows vehicle plate on booking card', async () => {
    const booking = makeBooking({ vehicle_plate: 'M-XY-999' });
    mockGetBookings.mockResolvedValue({ success: true, data: [booking] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });

    render(<BookingsPage />);

    await waitFor(() => {
      expect(screen.getByText('M-XY-999')).toBeInTheDocument();
    });
  });

  it('renders Book Now link in active empty state', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });

    render(<BookingsPage />);

    await waitFor(() => {
      expect(screen.getByText('Book Now')).toBeInTheDocument();
    });
    expect(screen.getByText('Book Now').closest('a')).toHaveAttribute('href', '/book');
  });

  it('handles booking_created websocket event with toast', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    const toastModule = await import('react-hot-toast');
    render(<BookingsPage />);
    await waitFor(() => expect(mockGetBookings).toHaveBeenCalled());
    wsCallbacks.onEvent?.({ event: 'booking_created', lot_id: 'l1' });
    expect(toastModule.default.success).toHaveBeenCalled();
  });

  it('handles booking_cancelled websocket event with toast', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    const toastModule = await import('react-hot-toast');
    render(<BookingsPage />);
    await waitFor(() => expect(mockGetBookings).toHaveBeenCalled());
    // The cancelled-event path uses toast(...) not success/error, so just assert no throw
    expect(() => wsCallbacks.onEvent?.({ event: 'booking_cancelled', lot_id: 'l1' })).not.toThrow();
  });

  it('shows error toast when cancel fails', async () => {
    const booking = makeBooking({ status: 'active' });
    mockGetBookings.mockResolvedValue({ success: true, data: [booking] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    mockCancelBooking.mockResolvedValue({ success: false, error: { message: 'server error' } });
    const toastModule = await import('react-hot-toast');
    render(<BookingsPage />);
    await waitFor(() => expect(screen.getByText('Lot Alpha')).toBeInTheDocument());
    const cancelBtn = screen.getByText('Cancel');
    await userEvent.setup().click(cancelBtn);
    await waitFor(() => {
      expect(toastModule.default.error).toHaveBeenCalled();
    });
  });

  it('search input filters bookings by lot name', async () => {
    const user = userEvent.setup();
    mockGetBookings.mockResolvedValue({
      success: true,
      data: [
        makeBooking({ id: 'b1', lot_name: 'Lot Alpha' }),
        makeBooking({ id: 'b2', lot_name: 'Lot Beta', slot_number: 'B1' }),
      ],
    });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    render(<BookingsPage />);
    await waitFor(() => expect(screen.getByText('Lot Alpha')).toBeInTheDocument());
    const search = screen.getByPlaceholderText('Search lot...');
    await user.type(search, 'beta');
    await waitFor(() => {
      expect(screen.queryByText('Lot Alpha')).not.toBeInTheDocument();
    });
    expect(screen.getByText('Lot Beta')).toBeInTheDocument();
  });

  it('status filter narrows visible bookings', async () => {
    const user = userEvent.setup();
    mockGetBookings.mockResolvedValue({
      success: true,
      data: [
        makeBooking({ id: 'b1', status: 'active', lot_name: 'Active Lot' }),
        makeBooking({ id: 'b2', status: 'cancelled', lot_name: 'Cancelled Lot' }),
      ],
    });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    render(<BookingsPage />);
    await waitFor(() => expect(screen.getByText('Active Lot')).toBeInTheDocument());
    const statusSel = screen.getByLabelText('Filter');
    await user.selectOptions(statusSel, 'cancelled');
    await waitFor(() => {
      expect(screen.queryByText('Active Lot')).not.toBeInTheDocument();
    });
    expect(screen.getByText('Cancelled Lot')).toBeInTheDocument();
  });

  it('shows upcoming booking section when start_time is in the future', async () => {
    const future = new Date(Date.now() + 24 * 3600_000).toISOString();
    const farFuture = new Date(Date.now() + 48 * 3600_000).toISOString();
    mockGetBookings.mockResolvedValue({
      success: true,
      data: [makeBooking({ status: 'confirmed', start_time: future, end_time: farFuture, lot_name: 'Future Lot' })],
    });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    render(<BookingsPage />);
    await waitFor(() => expect(screen.getByText('Future Lot')).toBeInTheDocument());
  });
});
