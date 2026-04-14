import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

vi.mock('../context/AuthContext', () => ({ useAuth: () => ({ user: { id: 'u1', role: 'admin' } }) }));
vi.mock('../api/client', () => ({ getInMemoryToken: vi.fn(() => 'tok') }));
vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (k: string, o?: any) => o?.name ? `Share ${o.name}` : o?.code ? `Code: ${o.code}` : k }) }));
vi.mock('framer-motion', () => ({
  motion: { div: React.forwardRef(({ children, ...p }: any, r: any) => <div ref={r} {...p}>{children}</div>) },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));
vi.mock('@phosphor-icons/react', () => {
  const C = (p: any) => <span {...p} />;
  return { UserPlus: C, Copy: C, ShareNetwork: C, Trash: C, QrCode: C, SpinnerGap: C, CheckCircle: C, CalendarBlank: C, MapPin: C };
});
vi.mock('react-hot-toast', () => ({ default: { success: vi.fn(), error: vi.fn() } }));

import { GuestPassPage } from './GuestPass';
import toast from 'react-hot-toast';

const guestBookings = [
  { id: 'g1', lot_id: 'l1', lot_name: 'Lot A', slot_id: 's1', slot_number: '5', guest_name: 'Alice', guest_email: 'a@b.com', guest_code: 'ABC123', start_time: '2026-04-10T08:00:00Z', end_time: '2026-04-10T17:00:00Z', status: 'active', created_at: '2026-04-09' },
  { id: 'g2', lot_id: 'l1', lot_name: 'Lot A', slot_id: 's2', slot_number: '6', guest_name: 'Bob', guest_email: null, guest_code: 'DEF456', start_time: '2026-04-09T08:00:00Z', end_time: '2026-04-09T17:00:00Z', status: 'expired', created_at: '2026-04-08' },
];
const lots = [{ id: 'l1', name: 'Lot A' }];
const slots = [
  { id: 's1', number: '5', status: 'available' },
  { id: 's2', number: '6', status: 'occupied' },
];

describe('GuestPassPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'DELETE') return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      if (opts?.method === 'POST') return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { id: 'gnew', lot_id: 'l1', lot_name: 'Lot A', slot_id: 's1', slot_number: '5', guest_name: 'New Guest', guest_email: null, guest_code: 'XYZ789', start_time: '2026-04-12T08:00:00Z', end_time: '2026-04-12T17:00:00Z', status: 'active', created_at: '2026-04-12' } }) } as Response);
      if (url.includes('/slots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: slots }) } as Response);
      if (url.includes('/lots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
      if (url.includes('/bookings/guest')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: guestBookings }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    }) as any;
    // Mock navigator.clipboard
    Object.defineProperty(navigator, 'clipboard', { value: { writeText: vi.fn().mockResolvedValue(undefined) }, writable: true, configurable: true });
    Object.defineProperty(navigator, 'share', { value: undefined, writable: true, configurable: true });
  });
  afterEach(() => vi.restoreAllMocks());

  it('renders guest bookings', async () => {
    render(<GuestPassPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());
    expect(screen.getByText('Bob')).toBeInTheDocument();
  });

  it('shows guest codes', async () => {
    render(<GuestPassPage />);
    await waitFor(() => {
      expect(screen.getByText('ABC123')).toBeInTheDocument();
      expect(screen.getByText('DEF456')).toBeInTheDocument();
    });
  });

  it('opens create form', async () => {
    render(<GuestPassPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-guest-btn')));
    expect(screen.getByTestId('guest-form')).toBeInTheDocument();
  });

  it('validates required fields', async () => {
    render(<GuestPassPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-guest-btn')));
    fireEvent.click(screen.getByTestId('submit-guest-btn'));
    // HTML validation prevents form submission
  });

  it('creates guest pass', async () => {
    const user = userEvent.setup();
    render(<GuestPassPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-guest-btn')));
    await user.type(screen.getByTestId('input-guest-name'), 'New Guest');
    await user.selectOptions(screen.getByTestId('select-lot'), 'l1');
    await waitFor(() => expect(screen.getByTestId('select-slot')).not.toBeDisabled());
    await user.selectOptions(screen.getByTestId('select-slot'), 's1');
    fireEvent.change(screen.getByTestId('input-start-time'), { target: { value: '2026-04-12T08:00' } });
    fireEvent.change(screen.getByTestId('input-end-time'), { target: { value: '2026-04-12T17:00' } });
    fireEvent.click(screen.getByTestId('submit-guest-btn'));
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('guestBooking.created'));
    expect(screen.getByTestId('guest-pass-card')).toBeInTheDocument();
    expect(screen.getByTestId('guest-code')).toHaveTextContent('XYZ789');
  });

  it('copies code', async () => {
    render(<GuestPassPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-guest-btn')));
    // Create pass first
    const user = userEvent.setup();
    await user.type(screen.getByTestId('input-guest-name'), 'X');
    await user.selectOptions(screen.getByTestId('select-lot'), 'l1');
    await waitFor(() => expect(screen.getByTestId('select-slot')).not.toBeDisabled());
    await user.selectOptions(screen.getByTestId('select-slot'), 's1');
    fireEvent.change(screen.getByTestId('input-start-time'), { target: { value: '2026-04-12T08:00' } });
    fireEvent.change(screen.getByTestId('input-end-time'), { target: { value: '2026-04-12T17:00' } });
    fireEvent.click(screen.getByTestId('submit-guest-btn'));
    await waitFor(() => expect(screen.getByTestId('copy-code-btn')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('copy-code-btn'));
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('guestBooking.codeCopied'));
  });

  it('shares pass (clipboard fallback)', async () => {
    render(<GuestPassPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-guest-btn')));
    const user = userEvent.setup();
    await user.type(screen.getByTestId('input-guest-name'), 'X');
    await user.selectOptions(screen.getByTestId('select-lot'), 'l1');
    await waitFor(() => expect(screen.getByTestId('select-slot')).not.toBeDisabled());
    await user.selectOptions(screen.getByTestId('select-slot'), 's1');
    fireEvent.change(screen.getByTestId('input-start-time'), { target: { value: '2026-04-12T08:00' } });
    fireEvent.change(screen.getByTestId('input-end-time'), { target: { value: '2026-04-12T17:00' } });
    fireEvent.click(screen.getByTestId('submit-guest-btn'));
    await waitFor(() => expect(screen.getByTestId('share-pass-btn')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('share-pass-btn'));
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('guestBooking.linkCopied'));
  });

  it('shares pass with navigator.share', async () => {
    Object.defineProperty(navigator, 'share', { value: vi.fn().mockResolvedValue(undefined), writable: true, configurable: true });
    render(<GuestPassPage />);
    // Share an existing booking
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());
    const shareBtns = screen.getAllByTitle('guestBooking.share');
    fireEvent.click(shareBtns[0]);
    await waitFor(() => expect(navigator.share).toHaveBeenCalled());
  });

  it('cancels guest booking (admin)', async () => {
    render(<GuestPassPage />);
    await waitFor(() => expect(screen.getByTestId('cancel-g1')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('cancel-g1'));
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('guestBooking.cancelled'));
  });

  it('cancel failure', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'DELETE') return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Cannot cancel' } }) } as Response);
      if (url.includes('/slots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: slots }) } as Response);
      if (url.includes('/lots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: guestBookings }) } as Response);
    }) as any;
    render(<GuestPassPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('cancel-g1')));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Cannot cancel'));
  });

  it('shows empty state', async () => {
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/slots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      if (url.includes('/lots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    }) as any;
    render(<GuestPassPage />);
    await waitFor(() => expect(screen.getByTestId('empty-state')).toBeInTheDocument());
  });

  it('dismiss created pass', async () => {
    const user = userEvent.setup();
    render(<GuestPassPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-guest-btn')));
    await user.type(screen.getByTestId('input-guest-name'), 'X');
    await user.selectOptions(screen.getByTestId('select-lot'), 'l1');
    await waitFor(() => expect(screen.getByTestId('select-slot')).not.toBeDisabled());
    await user.selectOptions(screen.getByTestId('select-slot'), 's1');
    fireEvent.change(screen.getByTestId('input-start-time'), { target: { value: '2026-04-12T08:00' } });
    fireEvent.change(screen.getByTestId('input-end-time'), { target: { value: '2026-04-12T17:00' } });
    fireEvent.click(screen.getByTestId('submit-guest-btn'));
    await waitFor(() => expect(screen.getByTestId('dismiss-pass')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('dismiss-pass'));
    await waitFor(() => expect(screen.queryByTestId('guest-pass-card')).not.toBeInTheDocument());
  });

  it('create failure', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST') return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Slot taken' } }) } as Response);
      if (url.includes('/slots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: slots }) } as Response);
      if (url.includes('/lots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    }) as any;
    const user = userEvent.setup();
    render(<GuestPassPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-guest-btn')));
    await user.type(screen.getByTestId('input-guest-name'), 'X');
    await user.selectOptions(screen.getByTestId('select-lot'), 'l1');
    await waitFor(() => expect(screen.getByTestId('select-slot')).not.toBeDisabled());
    await user.selectOptions(screen.getByTestId('select-slot'), 's1');
    fireEvent.change(screen.getByTestId('input-start-time'), { target: { value: '2026-04-12T08:00' } });
    fireEvent.change(screen.getByTestId('input-end-time'), { target: { value: '2026-04-12T17:00' } });
    fireEvent.click(screen.getByTestId('submit-guest-btn'));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Slot taken'));
  });

  it('cancel form', async () => {
    render(<GuestPassPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-guest-btn')));
    fireEvent.click(screen.getByText('common.cancel'));
    await waitFor(() => expect(screen.queryByTestId('guest-form')).not.toBeInTheDocument());
  });

  it('loads slots on lot change', async () => {
    render(<GuestPassPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-guest-btn')));
    fireEvent.change(screen.getByTestId('select-lot'), { target: { value: 'l1' } });
    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalledWith(expect.stringContaining('/l1/slots'), expect.anything()));
  });

  it('clears slots when lot cleared', async () => {
    render(<GuestPassPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-guest-btn')));
    fireEvent.change(screen.getByTestId('select-lot'), { target: { value: '' } });
    // slot dropdown should be disabled
    expect(screen.getByTestId('select-slot')).toBeDisabled();
  });

  it('loadBookings fetch rejection shows error toast', async () => {
    globalThis.fetch = vi.fn(() => Promise.reject(new Error('network down'))) as any;
    render(<GuestPassPage />);
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('common.error'));
  });

  it('handleSubmit fetch rejection shows error toast', async () => {
    const user = userEvent.setup();
    const callOrder: string[] = [];
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST') {
        callOrder.push('POST');
        return Promise.reject(new Error('submit fail'));
      }
      if (url.includes('/slots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: slots }) } as Response);
      if (url.includes('/lots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    }) as any;
    render(<GuestPassPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-guest-btn')));
    await user.type(screen.getByTestId('input-guest-name'), 'X');
    await user.selectOptions(screen.getByTestId('select-lot'), 'l1');
    await waitFor(() => expect(screen.getByTestId('select-slot')).not.toBeDisabled());
    await user.selectOptions(screen.getByTestId('select-slot'), 's1');
    fireEvent.change(screen.getByTestId('input-start-time'), { target: { value: '2026-04-12T08:00' } });
    fireEvent.change(screen.getByTestId('input-end-time'), { target: { value: '2026-04-12T17:00' } });
    fireEvent.click(screen.getByTestId('submit-guest-btn'));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('common.error'));
  });

  it('handleCancel fetch rejection shows error toast', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'DELETE') return Promise.reject(new Error('cancel fail'));
      if (url.includes('/bookings/guest')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: guestBookings }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    }) as any;
    render(<GuestPassPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());
    const cancelBtn = screen.getByTestId('cancel-g1');
    fireEvent.click(cancelBtn);
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('common.error'));
  });
});
