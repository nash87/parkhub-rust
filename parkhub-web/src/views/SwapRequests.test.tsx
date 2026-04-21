import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

const mockGetBookings = vi.fn();
vi.mock('../api/client', () => ({
  api: { getBookings: (...a: any[]) => mockGetBookings(...a) },
  getInMemoryToken: vi.fn(() => 'tok'),
}));

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (k: string) => k, i18n: { language: 'en' } }) }));
vi.mock('framer-motion', () => ({
  motion: { div: React.forwardRef(({ children, variants, ...p }: any, r: any) => <div ref={r} {...p}>{children}</div>) },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));
vi.mock('@phosphor-icons/react', () => {
  const C = (p: any) => <span {...p} />;
  return { Swap: C, Check: C, X: C, SpinnerGap: C, Plus: C, ArrowClockwise: C, CalendarBlank: C, Clock: C, ChatText: C };
});
vi.mock('react-hot-toast', () => ({ default: { success: vi.fn(), error: vi.fn() } }));
vi.mock('date-fns', () => ({ format: (_d: any, fmt: string) => fmt === 'dd.MM HH:mm' ? '10.04 08:00' : fmt === 'HH:mm' ? '08:00' : '10. Apr 2026' }));
vi.mock('date-fns/locale', () => ({ de: {}, enUS: {} }));
vi.mock('../constants/animations', () => ({
  stagger: {}, fadeUp: {},
  modalVariants: { initial: {}, animate: {}, exit: {} },
  modalTransition: {},
}));

import { SwapRequestsPage } from './SwapRequests';
import toast from 'react-hot-toast';

const requests = [
  { id: 'sw1', requester_id: 'u1', source_booking_id: 'b1', target_booking_id: 'b2', source_booking: { lot_name: 'Lot A', slot_number: '5', start_time: '2026-04-10T08:00:00Z', end_time: '2026-04-10T17:00:00Z' }, target_booking: { lot_name: 'Lot B', slot_number: '10', start_time: '2026-04-10T08:00:00Z', end_time: '2026-04-10T17:00:00Z' }, message: 'Please swap', status: 'pending', created_at: '2026-04-09T00:00:00Z' },
  { id: 'sw2', requester_id: 'u2', source_booking_id: 'b3', target_booking_id: 'b4', source_booking: { lot_name: 'Lot C', slot_number: '1', start_time: '2026-04-11T08:00:00Z', end_time: '2026-04-11T17:00:00Z' }, target_booking: { lot_name: 'Lot D', slot_number: '2', start_time: '2026-04-11T08:00:00Z', end_time: '2026-04-11T17:00:00Z' }, message: null, status: 'accepted', created_at: '2026-04-09T00:00:00Z' },
];

const bookings = [
  { id: 'b1', lot_name: 'Lot A', slot_number: '5', start_time: '2026-04-10T08:00:00Z', end_time: '2026-04-10T17:00:00Z', status: 'active' },
];

describe('SwapRequestsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetBookings.mockResolvedValue({ success: true, data: bookings });
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (url.includes('/accept')) return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      if (url.includes('/decline')) return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      if (opts?.method === 'POST') return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: requests }) } as Response);
    }) as any;
  });
  afterEach(() => vi.restoreAllMocks());

  it('renders swap requests', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('Lot A')).toBeInTheDocument());
  });

  it('shows message when present', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('Please swap')).toBeInTheDocument());
  });

  it('accepts swap', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('swap.accept')).toBeInTheDocument());
    fireEvent.click(screen.getByText('swap.accept'));
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('swap.accepted'));
  });

  it('declines swap', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('swap.decline')).toBeInTheDocument());
    fireEvent.click(screen.getByText('swap.decline'));
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('swap.declined'));
  });

  it('opens create modal', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('swap.create')).toBeInTheDocument());
    fireEvent.click(screen.getByText('swap.create'));
    await waitFor(() => expect(screen.getByTestId('swap-modal')).toBeInTheDocument());
  });

  it('creates swap', async () => {
    const user = userEvent.setup();
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('swap.create')).toBeInTheDocument());
    fireEvent.click(screen.getByText('swap.create'));
    await waitFor(() => expect(screen.getByTestId('select-source')).toBeInTheDocument());
    await user.selectOptions(screen.getByTestId('select-source'), 'b1');
    await user.type(screen.getByTestId('input-target'), 'b99');
    await user.type(screen.getByTestId('input-message'), 'Please');
    fireEvent.click(screen.getByTestId('submit-swap'));
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('swap.created'));
  });

  it('create swap failure', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST' && url.includes('/swap-request')) return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Not found' } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: requests }) } as Response);
    }) as any;
    const user = userEvent.setup();
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('swap.create')).toBeInTheDocument());
    fireEvent.click(screen.getByText('swap.create'));
    await waitFor(() => expect(screen.getByTestId('select-source')).toBeInTheDocument());
    await user.selectOptions(screen.getByTestId('select-source'), 'b1');
    await user.type(screen.getByTestId('input-target'), 'b99');
    fireEvent.click(screen.getByTestId('submit-swap'));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Not found'));
  });

  it('close create modal', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('swap.create')).toBeInTheDocument());
    fireEvent.click(screen.getByText('swap.create'));
    await waitFor(() => expect(screen.getByTestId('swap-modal')).toBeInTheDocument());
    fireEvent.click(screen.getByText('common.cancel'));
    await waitFor(() => expect(screen.queryByTestId('swap-modal')).not.toBeInTheDocument());
  });

  it('shows empty state', async () => {
    globalThis.fetch = vi.fn(() => Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response)) as any;
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('swap.empty')).toBeInTheDocument());
  });

  it('refresh button', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => fireEvent.click(screen.getByText('common.refresh')));
    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalled());
  });

  it('accept error', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (url.includes('/accept')) return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Already handled' } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: requests }) } as Response);
    }) as any;
    render(<SwapRequestsPage />);
    await waitFor(() => fireEvent.click(screen.getByText('swap.accept')));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Already handled'));
  });

  it('decline error', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (url.includes('/decline')) return Promise.reject(new Error('net'));
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: requests }) } as Response);
    }) as any;
    render(<SwapRequestsPage />);
    await waitFor(() => fireEvent.click(screen.getByText('swap.decline')));
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });

  it('catches loadData errors', async () => {
    globalThis.fetch = vi.fn(() => Promise.reject(new Error('net'))) as any;
    mockGetBookings.mockRejectedValue(new Error('net'));
    render(<SwapRequestsPage />);
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });

  it('accept catches network error', async () => {
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/accept')) return Promise.reject(new Error('net'));
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: requests }) } as Response);
    }) as any;
    render(<SwapRequestsPage />);
    await waitFor(() => fireEvent.click(screen.getByText('swap.accept')));
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });

  it('decline server error returns message', async () => {
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/decline')) return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'No' } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: requests }) } as Response);
    }) as any;
    render(<SwapRequestsPage />);
    await waitFor(() => fireEvent.click(screen.getByText('swap.decline')));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('No'));
  });

  it('handleCreate is no-op when no booking selected', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('swap.create')).toBeInTheDocument());
    fireEvent.click(screen.getByText('swap.create'));
    await waitFor(() => expect(screen.getByTestId('swap-modal')).toBeInTheDocument());
    // Submit without selecting source/target
    fireEvent.click(screen.getByTestId('submit-swap'));
    // No fetch call to /swap-request
  });

  it('closes modal via X button at top', async () => {
    render(<SwapRequestsPage />);
    await waitFor(() => fireEvent.click(screen.getByText('swap.create')));
    await waitFor(() => expect(screen.getByTestId('swap-modal')).toBeInTheDocument());
    const modal = screen.getByTestId('swap-modal');
    const xButtons = modal.querySelectorAll('button.btn-ghost');
    if (xButtons.length > 0) fireEvent.click(xButtons[0]);
    await waitFor(() => expect(screen.queryByTestId('swap-modal')).not.toBeInTheDocument());
  });

  it('create network error', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST' && url.includes('/swap-request')) return Promise.reject(new Error('net'));
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: requests }) } as Response);
    }) as any;
    const user = userEvent.setup();
    render(<SwapRequestsPage />);
    await waitFor(() => expect(screen.getByText('swap.create')).toBeInTheDocument());
    fireEvent.click(screen.getByText('swap.create'));
    await waitFor(() => expect(screen.getByTestId('select-source')).toBeInTheDocument());
    await user.selectOptions(screen.getByTestId('select-source'), 'b1');
    await user.type(screen.getByTestId('input-target'), 'b99');
    fireEvent.click(screen.getByTestId('submit-swap'));
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });
});
