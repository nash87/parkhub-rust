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

vi.mock('../constants/animations', () => ({
  stagger: { hidden: {}, show: {} },
  fadeUp: { hidden: {}, show: {} },
  modalVariants: { initial: {}, animate: {}, exit: {} },
  modalTransition: { duration: 0 },
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

  it('shows slot numbers', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => {
      expect(screen.getByText('A3', { exact: false })).toBeInTheDocument();
      expect(screen.getByText('B7', { exact: false })).toBeInTheDocument();
    });
  });

  it('shows Your Slot and Their Slot labels', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => {
      expect(screen.getByText('Your Slot')).toBeInTheDocument();
      expect(screen.getByText('Their Slot')).toBeInTheDocument();
    });
  });

  it('shows create modal fields correctly', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('Swap Requests')).toBeInTheDocument());

    fireEvent.click(screen.getByText('New Swap'));
    await waitFor(() => {
      expect(screen.getByTestId('swap-modal')).toBeInTheDocument();
      expect(screen.getByText('Your Booking')).toBeInTheDocument();
      expect(screen.getByText('Target Booking ID')).toBeInTheDocument();
    });
  });

  it('does not show accept/decline for accepted requests', async () => {
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/api/v1/swap-requests') && !url.includes('/accept') && !url.includes('/decline')) {
        return Promise.resolve({
          json: () => Promise.resolve({
            success: true,
            data: [{
              ...sampleSwapRequests[0],
              status: 'accepted',
            }],
          }),
        } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: {} }) } as Response);
    });

    render(<SwapRequestsPage />);
    await waitFor(() => {
      expect(screen.getByText('Accepted')).toBeInTheDocument();
    });
    expect(screen.queryByText('Accept')).not.toBeInTheDocument();
    expect(screen.queryByText('Decline')).not.toBeInTheDocument();
  });

  it('shows refresh button', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => {
      expect(screen.getByText('Refresh')).toBeInTheDocument();
    });
  });

  it('refreshes data when clicking refresh', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('Refresh')).toBeInTheDocument());

    const fetchCountBefore = (global.fetch as ReturnType<typeof vi.fn>).mock.calls.length;
    fireEvent.click(screen.getByText('Refresh'));

    await waitFor(() => {
      expect((global.fetch as ReturnType<typeof vi.fn>).mock.calls.length).toBeGreaterThan(fetchCountBefore);
    });
  });

  it('shows modal fields when creating a swap', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('Swap Requests')).toBeInTheDocument());

    fireEvent.click(screen.getByText('New Swap'));
    await waitFor(() => {
      expect(screen.getByTestId('swap-modal')).toBeInTheDocument();
      expect(screen.getByText('New Swap Request')).toBeInTheDocument();
      expect(screen.getByText('Your Booking')).toBeInTheDocument();
      expect(screen.getByText('Target Booking ID')).toBeInTheDocument();
    });
  });

  it('closes create modal on cancel', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('Swap Requests')).toBeInTheDocument());

    fireEvent.click(screen.getByText('New Swap'));
    await waitFor(() => expect(screen.getByTestId('swap-modal')).toBeInTheDocument());

    fireEvent.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(screen.queryByTestId('swap-modal')).not.toBeInTheDocument();
    });
  });

  it('handles accept failure', async () => {
    global.fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/accept') && opts?.method === 'POST') {
        return Promise.resolve({
          json: () => Promise.resolve({ success: false, error: { message: 'Cannot accept' } }),
        } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/swap-requests')) {
        return Promise.resolve({
          json: () => Promise.resolve({ success: true, data: sampleSwapRequests }),
        } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });

    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('Accept')).toBeInTheDocument());

    fireEvent.click(screen.getByText('Accept'));
    // Error path exercised
  });

  it('handles decline failure', async () => {
    global.fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/decline') && opts?.method === 'POST') {
        return Promise.resolve({
          json: () => Promise.resolve({ success: false, error: { message: 'Cannot decline' } }),
        } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/swap-requests')) {
        return Promise.resolve({
          json: () => Promise.resolve({ success: true, data: sampleSwapRequests }),
        } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });

    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('Decline')).toBeInTheDocument());

    fireEvent.click(screen.getByText('Decline'));
    // Error path exercised
  });

  it('handles accept network exception', async () => {
    global.fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/accept') && opts?.method === 'POST') {
        return Promise.reject(new Error('Network'));
      }
      if (typeof url === 'string' && url.includes('/api/v1/swap-requests')) {
        return Promise.resolve({
          json: () => Promise.resolve({ success: true, data: sampleSwapRequests }),
        } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });

    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('Accept')).toBeInTheDocument());

    fireEvent.click(screen.getByText('Accept'));
    // Exception path exercised
  });

  it('handles create swap with no selection (button disabled)', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('Swap Requests')).toBeInTheDocument());

    fireEvent.click(screen.getByText('New Swap'));
    await waitFor(() => expect(screen.getByTestId('swap-modal')).toBeInTheDocument());

    // Submit button should be disabled without source/target
    const submitBtn = screen.getByTestId('submit-swap');
    expect(submitBtn).toBeDisabled();
  });

  it('shows declined status badge', async () => {
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/api/v1/swap-requests') && !url.includes('/accept') && !url.includes('/decline')) {
        return Promise.resolve({
          json: () => Promise.resolve({
            success: true,
            data: [{ ...sampleSwapRequests[0], status: 'declined', message: null }],
          }),
        } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: {} }) } as Response);
    });

    render(<SwapRequestsPage />);
    await waitFor(() => {
      expect(screen.getByText('Declined')).toBeInTheDocument();
    });
    // No accept/decline buttons for declined
    expect(screen.queryByText('Accept')).not.toBeInTheDocument();
  });

  it('handles load data fetch exception', async () => {
    global.fetch = vi.fn(() => Promise.reject(new Error('Network')));
    mockGetBookings.mockRejectedValue(new Error('Network'));

    render(<SwapRequestsPage />);
    // Should show something, not crash
    await waitFor(() => {
      expect(screen.getByText('Swap Requests')).toBeInTheDocument();
    });
  });
});
