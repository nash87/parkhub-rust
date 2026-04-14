import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (k: string, o?: any) => o?.pos ? `Position ${o.pos}` : o?.minutes ? `${o.minutes} min` : o?.total ? `${o.total} slots` : k }) }));
vi.mock('framer-motion', () => ({
  motion: { div: React.forwardRef(({ children, ...p }: any, r: any) => <div ref={r} {...p}>{children}</div>) },
}));
vi.mock('@phosphor-icons/react', () => {
  const C = (p: any) => <span {...p} />;
  return { Bell: C, Queue: C, Check: C, X: C, Question: C, Clock: C, ArrowUp: C };
});
vi.mock('react-hot-toast', () => ({ default: { success: vi.fn(), error: vi.fn() } }));

import { WaitlistPage } from './Waitlist';
import toast from 'react-hot-toast';

const lots = [
  { id: 'l1', name: 'Full Lot', total_slots: 10, available_slots: 0 },
  { id: 'l2', name: 'Open Lot', total_slots: 10, available_slots: 5 },
];

const waitlistEntries = [
  { entry: { id: 'w1', user_id: 'u1', lot_id: 'l1', created_at: '2026-04-10', notified_at: null, status: 'waiting', offer_expires_at: null, accepted_booking_id: null }, position: 2, total_ahead: 1, estimated_wait_minutes: 15 },
];

const offeredEntries = [
  { entry: { id: 'w2', user_id: 'u1', lot_id: 'l1', created_at: '2026-04-10', notified_at: '2026-04-10', status: 'offered', offer_expires_at: '2026-04-11', accepted_booking_id: null }, position: 1, total_ahead: 0, estimated_wait_minutes: null },
];

describe('WaitlistPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST' && url.includes('/subscribe')) return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      if (opts?.method === 'POST' && url.includes('/accept')) return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      if (opts?.method === 'POST' && url.includes('/decline')) return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      if (opts?.method === 'DELETE') return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      if (url.includes('/waitlist')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { entries: waitlistEntries } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
    }) as any;
  });
  afterEach(() => vi.restoreAllMocks());

  it('renders waitlist page', async () => {
    render(<WaitlistPage />);
    await waitFor(() => expect(screen.getByText('waitlistExt.title')).toBeInTheDocument());
  });

  it('shows user entries', async () => {
    render(<WaitlistPage />);
    await waitFor(() => expect(screen.getByText('waitlistExt.yourEntries')).toBeInTheDocument());
  });

  it('shows position and wait time', async () => {
    render(<WaitlistPage />);
    await waitFor(() => {
      expect(screen.getByText('Position 2')).toBeInTheDocument();
      expect(screen.getByText('15 min')).toBeInTheDocument();
    });
  });

  it('join waitlist', async () => {
    // Set up a lot that the user hasn't joined yet (l1 has entries, need new full lot)
    const lotsWithExtra = [...lots, { id: 'l3', name: 'New Full', total_slots: 5, available_slots: 0 }];
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST' && url.includes('/subscribe')) return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      if (url.includes('/l3/waitlist')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { entries: [] } }) } as Response);
      if (url.includes('/waitlist')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { entries: waitlistEntries } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lotsWithExtra }) } as Response);
    }) as any;
    render(<WaitlistPage />);
    await waitFor(() => expect(screen.getByText('New Full')).toBeInTheDocument());
    fireEvent.click(screen.getByText('waitlistExt.joinWaitlist'));
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('waitlistExt.joined'));
  });

  it('leave waitlist', async () => {
    render(<WaitlistPage />);
    await waitFor(() => expect(screen.getByText('waitlistExt.leave')).toBeInTheDocument());
    fireEvent.click(screen.getByText('waitlistExt.leave'));
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('waitlistExt.left'));
  });

  it('accept offered spot', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST' && url.includes('/accept')) return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      if (url.includes('/waitlist')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { entries: offeredEntries } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
    }) as any;
    render(<WaitlistPage />);
    await waitFor(() => expect(screen.getByText('waitlistExt.accept')).toBeInTheDocument());
    fireEvent.click(screen.getByText('waitlistExt.accept'));
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('waitlistExt.accepted'));
  });

  it('decline offered spot', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST' && url.includes('/decline')) return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      if (url.includes('/waitlist')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { entries: offeredEntries } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
    }) as any;
    render(<WaitlistPage />);
    await waitFor(() => expect(screen.getByText('waitlistExt.decline')).toBeInTheDocument());
    fireEvent.click(screen.getByText('waitlistExt.decline'));
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('waitlistExt.declined'));
  });

  it('shows help', async () => {
    render(<WaitlistPage />);
    await waitFor(() => fireEvent.click(screen.getByLabelText('waitlistExt.helpLabel')));
    expect(screen.getByText('waitlistExt.help')).toBeInTheDocument();
  });

  it('shows no full lots state', async () => {
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/waitlist')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { entries: [] } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'l1', name: 'Open', total_slots: 10, available_slots: 5 }] }) } as Response);
    }) as any;
    render(<WaitlistPage />);
    await waitFor(() => expect(screen.getByText('waitlistExt.noFullLots')).toBeInTheDocument());
  });

  it('join error', async () => {
    const lotsWithNew = [{ id: 'l4', name: 'ErrorLot', total_slots: 5, available_slots: 0 }];
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST' && url.includes('/subscribe')) return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Already joined' } }) } as Response);
      if (url.includes('/waitlist')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { entries: [] } }) } as Response);
      if (url.includes('/lots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lotsWithNew }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    }) as any;
    render(<WaitlistPage />);
    await waitFor(() => expect(screen.getByText('waitlistExt.joinWaitlist')).toBeInTheDocument());
    fireEvent.click(screen.getByText('waitlistExt.joinWaitlist'));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Already joined'));
  });

  it('join network error', async () => {
    const lotsWithNew = [{ id: 'l5', name: 'NetErrLot', total_slots: 5, available_slots: 0 }];
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST') return Promise.reject(new Error('net'));
      if (url.includes('/waitlist')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { entries: [] } }) } as Response);
      if (url.includes('/lots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lotsWithNew }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    }) as any;
    render(<WaitlistPage />);
    await waitFor(() => expect(screen.getByText('waitlistExt.joinWaitlist')).toBeInTheDocument());
    fireEvent.click(screen.getByText('waitlistExt.joinWaitlist'));
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });

  it('leave error', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'DELETE') return Promise.reject(new Error('net'));
      if (url.includes('/waitlist')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { entries: waitlistEntries } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
    }) as any;
    render(<WaitlistPage />);
    await waitFor(() => fireEvent.click(screen.getByText('waitlistExt.leave')));
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });

  it('accept error', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST' && url.includes('/accept')) return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Expired' } }) } as Response);
      if (url.includes('/waitlist')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { entries: offeredEntries } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
    }) as any;
    render(<WaitlistPage />);
    await waitFor(() => fireEvent.click(screen.getByText('waitlistExt.accept')));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Expired'));
  });

  it('accept network error', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST' && url.includes('/accept')) return Promise.reject(new Error('net'));
      if (url.includes('/waitlist')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { entries: offeredEntries } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
    }) as any;
    render(<WaitlistPage />);
    await waitFor(() => fireEvent.click(screen.getByText('waitlistExt.accept')));
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });

  it('decline network error', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST' && url.includes('/decline')) return Promise.reject(new Error('net'));
      if (url.includes('/waitlist')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { entries: offeredEntries } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
    }) as any;
    render(<WaitlistPage />);
    await waitFor(() => fireEvent.click(screen.getByText('waitlistExt.decline')));
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });
});
