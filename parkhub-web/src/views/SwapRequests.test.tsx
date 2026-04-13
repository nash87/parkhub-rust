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
}));

vi.mock('react-router-dom', () => ({
  Link: ({ to, children, ...props }: any) => <a href={to} {...props}>{children}</a>,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => {
      const map: Record<string, string> = {
        'swap.title': 'Swap Requests',
        'swap.subtitle': 'Trade parking slots with colleagues',
        'swap.create': 'New Swap',
        'swap.empty': 'No swap requests',
        'swap.emptyHint': 'Create a swap request to trade your slot',
        'swap.accept': 'Accept',
        'swap.decline': 'Decline',
        'swap.accepted': 'Swap accepted',
        'swap.declined': 'Swap declined',
        'swap.created': 'Swap request sent',
        'swap.createTitle': 'New Swap Request',
        'swap.yourBooking': 'Your Booking',
        'swap.selectBooking': 'Select a booking...',
        'swap.targetBookingId': 'Target Booking ID',
        'swap.targetPlaceholder': 'Enter booking ID to swap with',
        'swap.messageLabel': 'Message (optional)',
        'swap.messagePlaceholder': 'Add a note...',
        'swap.send': 'Send Request',
        'swap.yourSlot': 'Your Slot',
        'swap.theirSlot': 'Their Slot',
        'swap.status.pending': 'Pending',
        'swap.status.accepted': 'Accepted',
        'swap.status.declined': 'Declined',
        'common.refresh': 'Refresh',
        'common.cancel': 'Cancel',
        'common.error': 'Error',
        'dashboard.slot': 'Slot',
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
  Swap: (props: any) => <span data-testid="icon-swap" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  Plus: (props: any) => <span data-testid="icon-plus" {...props} />,
  ArrowClockwise: (props: any) => <span data-testid="icon-refresh" {...props} />,
  CalendarBlank: (props: any) => <span data-testid="icon-calendar" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  ChatText: (props: any) => <span data-testid="icon-chat" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: { success: vi.fn(), error: vi.fn() },
}));

import { SwapRequestsPage } from './SwapRequests';

const sampleSwapRequests = [
  {
    id: 'sr-1',
    requester_id: 'u1',
    source_booking_id: 'b1',
    target_booking_id: 'b2',
    source_booking: {
      lot_name: 'Garage West',
      slot_number: 'A3',
      start_time: '2026-04-15T08:00:00Z',
      end_time: '2026-04-15T17:00:00Z',
    },
    target_booking: {
      lot_name: 'Garage East',
      slot_number: 'B7',
      start_time: '2026-04-15T09:00:00Z',
      end_time: '2026-04-15T18:00:00Z',
    },
    message: 'Would you mind swapping?',
    status: 'pending',
    created_at: '2026-04-14T10:00:00Z',
  },
];

describe('SwapRequestsPage', () => {
  beforeEach(() => {
    mockGetBookings.mockClear();
    mockGetBookings.mockResolvedValue({ success: true, data: [] });

    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/api/v1/swap-requests') && !url.includes('/accept') && !url.includes('/decline')) {
        return Promise.resolve({
          json: () => Promise.resolve({ success: true, data: sampleSwapRequests }),
        } as Response);
      }
      return Promise.resolve({
        json: () => Promise.resolve({ success: true, data: {} }),
      } as Response);
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders title and subtitle', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => {
      expect(screen.getByText('Swap Requests')).toBeInTheDocument();
    });
    expect(screen.getByText('Trade parking slots with colleagues')).toBeInTheDocument();
  });

  it('renders swap request cards with lot names', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => {
      expect(screen.getByText('Garage West')).toBeInTheDocument();
      expect(screen.getByText('Garage East')).toBeInTheDocument();
    });
  });

  it('shows pending status badge', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => {
      expect(screen.getByText('Pending')).toBeInTheDocument();
    });
  });

  it('shows message on swap request card', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => {
      expect(screen.getByText('Would you mind swapping?')).toBeInTheDocument();
    });
  });

  it('shows accept and decline buttons for pending requests', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => {
      expect(screen.getByText('Accept')).toBeInTheDocument();
      expect(screen.getByText('Decline')).toBeInTheDocument();
    });
  });

  it('shows empty state when no swap requests', async () => {
    global.fetch = vi.fn(() =>
      Promise.resolve({
        json: () => Promise.resolve({ success: true, data: [] }),
      } as Response),
    );

    render(<SwapRequestsPage />);
    await waitFor(() => {
      expect(screen.getByText('No swap requests')).toBeInTheDocument();
    });
  });

  it('opens create swap modal on button click', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => {
      expect(screen.getByText('Swap Requests')).toBeInTheDocument();
    });

    const newBtn = screen.getByText('New Swap');
    fireEvent.click(newBtn);

    await waitFor(() => {
      expect(screen.getByTestId('swap-modal')).toBeInTheDocument();
      expect(screen.getByText('New Swap Request')).toBeInTheDocument();
    });
  });

  it('calls accept endpoint when accept is clicked', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => {
      expect(screen.getByText('Accept')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Accept'));

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/v1/swap-requests/sr-1/accept',
        expect.objectContaining({ method: 'POST' }),
      );
    });
  });

  it('calls decline endpoint when decline is clicked', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => {
      expect(screen.getByText('Decline')).toBeInTheDocument();
    });

    fireEvent.click(screen.getByText('Decline'));

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/v1/swap-requests/sr-1/decline',
        expect.objectContaining({ method: 'POST' }),
      );
    });
  });
});
