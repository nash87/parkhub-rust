import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

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

vi.mock('../api/client', () => ({
  api: {
    getBookings: (...args: any[]) => mockGetBookings(...args),
  },
  getInMemoryToken: () => 'test-token',
}));

vi.mock('react-router-dom', () => ({
  Link: ({ to, children, ...props }: any) => <a href={to} {...props}>{children}</a>,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => {
      const map: Record<string, string> = {
        'checkin.title': 'Check In',
        'checkin.subtitle': 'Scan your QR code or check in manually',
        'checkin.noBooking': 'No active booking',
        'checkin.noBookingHint': 'Book a parking spot first',
        'checkin.bookNow': 'Book Now',
        'checkin.checkInBtn': 'Check In',
        'checkin.checkOutBtn': 'Check Out',
        'checkin.checkedIn': 'Checked in successfully',
        'checkin.checkedOut': 'Checked out successfully',
        'checkin.elapsed': 'Elapsed Time',
        'checkin.since': `Since ${opts?.time || ''}`,
        'checkin.date': 'Date',
        'checkin.startTime': 'Start',
        'checkin.endTime': 'End',
        'checkin.qrAlt': 'QR Code for check-in',
        'checkin.scanQr': 'Show this QR code at the entrance',
        'dashboard.slot': 'Slot',
        'common.error': 'Error',
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
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  QrCode: (props: any) => <span data-testid="icon-qrcode" {...props} />,
  SignIn: (props: any) => <span data-testid="icon-signin" {...props} />,
  SignOut: (props: any) => <span data-testid="icon-signout" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  MapPin: (props: any) => <span data-testid="icon-mappin" {...props} />,
  CalendarBlank: (props: any) => <span data-testid="icon-calendar" {...props} />,
  ArrowClockwise: (props: any) => <span data-testid="icon-refresh" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: { success: vi.fn(), error: vi.fn() },
}));

import { QRCheckInPage } from './QRCheckIn';
import type { Booking } from '../api/client';

function makeActiveBooking(overrides: Partial<Booking> = {}): Booking {
  const now = Date.now();
  return {
    id: 'b1',
    user_id: 'u1',
    lot_id: 'l1',
    slot_id: 's1',
    lot_name: 'Garage Central',
    slot_number: 'C5',
    start_time: new Date(now - 3600000).toISOString(),
    end_time: new Date(now + 3600000).toISOString(),
    status: 'active',
    ...overrides,
  };
}

// Helper to create a blob response for QR
function mockQrBlob() {
  return new Blob(['fake-png'], { type: 'image/png' });
}

describe('QRCheckInPage', () => {
  beforeEach(() => {
    mockGetBookings.mockClear();

    // Mock URL.createObjectURL / revokeObjectURL
    global.URL.createObjectURL = vi.fn(() => 'blob:mock-qr-url');
    global.URL.revokeObjectURL = vi.fn();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders title', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });

    render(<QRCheckInPage />);
    await waitFor(() => {
      expect(screen.getByText('Check In')).toBeInTheDocument();
    });
  });

  it('shows no-booking state when no active bookings', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });

    render(<QRCheckInPage />);
    await waitFor(() => {
      expect(screen.getByText('No active booking')).toBeInTheDocument();
      expect(screen.getByText('Book a parking spot first')).toBeInTheDocument();
    });
    expect(screen.getByText('Book Now').closest('a')).toHaveAttribute('href', '/book');
  });

  it('shows booking details with lot name and slot', async () => {
    const booking = makeActiveBooking();
    mockGetBookings.mockResolvedValue({ success: true, data: [booking] });

    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/check-in') && !url.includes('POST')) {
        return Promise.resolve({
          json: () => Promise.resolve({
            success: true,
            data: { checked_in: false, checked_in_at: null, checked_out_at: null },
          }),
        } as Response);
      }
      if (typeof url === 'string' && url.includes('/qr')) {
        return Promise.resolve({
          ok: true,
          blob: () => Promise.resolve(mockQrBlob()),
        } as Response);
      }
      return Promise.resolve({
        json: () => Promise.resolve({ success: true, data: {} }),
      } as Response);
    });

    render(<QRCheckInPage />);
    await waitFor(() => {
      expect(screen.getByText('Garage Central')).toBeInTheDocument();
      expect(screen.getByText('C5')).toBeInTheDocument();
    });
  });

  it('shows QR code and check-in button when not checked in', async () => {
    const booking = makeActiveBooking();
    mockGetBookings.mockResolvedValue({ success: true, data: [booking] });

    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/check-in')) {
        return Promise.resolve({
          json: () => Promise.resolve({
            success: true,
            data: { checked_in: false, checked_in_at: null, checked_out_at: null },
          }),
        } as Response);
      }
      if (typeof url === 'string' && url.includes('/qr')) {
        return Promise.resolve({
          ok: true,
          blob: () => Promise.resolve(mockQrBlob()),
        } as Response);
      }
      return Promise.resolve({
        json: () => Promise.resolve({ success: true, data: {} }),
      } as Response);
    });

    render(<QRCheckInPage />);
    await waitFor(() => {
      expect(screen.getByTestId('qr-code')).toBeInTheDocument();
      expect(screen.getByTestId('checkin-btn')).toBeInTheDocument();
    });
  });

  it('shows elapsed timer and check-out button when checked in', async () => {
    const booking = makeActiveBooking();
    mockGetBookings.mockResolvedValue({ success: true, data: [booking] });

    const checkedInAt = new Date(Date.now() - 1800000).toISOString(); // 30 min ago
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/check-in')) {
        return Promise.resolve({
          json: () => Promise.resolve({
            success: true,
            data: { checked_in: true, checked_in_at: checkedInAt, checked_out_at: null },
          }),
        } as Response);
      }
      if (typeof url === 'string' && url.includes('/qr')) {
        return Promise.resolve({
          ok: true,
          blob: () => Promise.resolve(mockQrBlob()),
        } as Response);
      }
      return Promise.resolve({
        json: () => Promise.resolve({ success: true, data: {} }),
      } as Response);
    });

    render(<QRCheckInPage />);
    await waitFor(() => {
      expect(screen.getByText('Elapsed Time')).toBeInTheDocument();
      expect(screen.getByTestId('elapsed-timer')).toBeInTheDocument();
      expect(screen.getByTestId('checkout-btn')).toBeInTheDocument();
    });
  });

  it('calls check-in endpoint when check-in button is clicked', async () => {
    const booking = makeActiveBooking();
    mockGetBookings.mockResolvedValue({ success: true, data: [booking] });

    const fetchMock = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/check-in') && opts?.method === 'POST') {
        return Promise.resolve({
          json: () => Promise.resolve({ success: true, data: {} }),
        } as Response);
      }
      if (typeof url === 'string' && url.includes('/check-in')) {
        return Promise.resolve({
          json: () => Promise.resolve({
            success: true,
            data: { checked_in: false, checked_in_at: null, checked_out_at: null },
          }),
        } as Response);
      }
      if (typeof url === 'string' && url.includes('/qr')) {
        return Promise.resolve({
          ok: true,
          blob: () => Promise.resolve(mockQrBlob()),
        } as Response);
      }
      return Promise.resolve({
        json: () => Promise.resolve({ success: true, data: {} }),
      } as Response);
    });
    global.fetch = fetchMock;

    render(<QRCheckInPage />);
    await waitFor(() => {
      expect(screen.getByTestId('checkin-btn')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId('checkin-btn'));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        `/api/v1/bookings/${booking.id}/check-in`,
        expect.objectContaining({ method: 'POST' }),
      );
    });
  });

  it('calls check-out endpoint when check-out button is clicked', async () => {
    const booking = makeActiveBooking();
    mockGetBookings.mockResolvedValue({ success: true, data: [booking] });

    const checkedInAt = new Date(Date.now() - 600000).toISOString();
    const fetchMock = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/check-out') && opts?.method === 'POST') {
        return Promise.resolve({
          json: () => Promise.resolve({ success: true, data: {} }),
        } as Response);
      }
      if (typeof url === 'string' && url.includes('/check-in')) {
        return Promise.resolve({
          json: () => Promise.resolve({
            success: true,
            data: { checked_in: true, checked_in_at: checkedInAt, checked_out_at: null },
          }),
        } as Response);
      }
      if (typeof url === 'string' && url.includes('/qr')) {
        return Promise.resolve({
          ok: true,
          blob: () => Promise.resolve(mockQrBlob()),
        } as Response);
      }
      return Promise.resolve({
        json: () => Promise.resolve({ success: true, data: {} }),
      } as Response);
    });
    global.fetch = fetchMock;

    render(<QRCheckInPage />);
    await waitFor(() => {
      expect(screen.getByTestId('checkout-btn')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId('checkout-btn'));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        `/api/v1/bookings/${booking.id}/check-out`,
        expect.objectContaining({ method: 'POST' }),
      );
    });
  });

  it('ignores past and future bookings, shows no-booking state', async () => {
    const now = Date.now();
    const futureBooking: Booking = {
      id: 'b2',
      user_id: 'u1',
      lot_id: 'l1',
      slot_id: 's1',
      lot_name: 'Future Lot',
      slot_number: 'F1',
      start_time: new Date(now + 7200000).toISOString(),
      end_time: new Date(now + 10800000).toISOString(),
      status: 'confirmed',
    };
    mockGetBookings.mockResolvedValue({ success: true, data: [futureBooking] });

    render(<QRCheckInPage />);
    await waitFor(() => {
      expect(screen.getByText('No active booking')).toBeInTheDocument();
    });
  });
});
