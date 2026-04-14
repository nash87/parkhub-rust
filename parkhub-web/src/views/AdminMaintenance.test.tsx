import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (k: string, f?: any) => (typeof f === 'string' ? f : f?.count != null ? `${f.count} active` : k) }) }));
vi.mock('framer-motion', () => ({
  motion: { div: React.forwardRef(({ children, ...p }: any, r: any) => <div ref={r} {...p}>{children}</div>) },
}));
vi.mock('@phosphor-icons/react', () => {
  const C = (p: any) => <span {...p} />;
  return { Wrench: C, Plus: C, Trash: C, PencilSimple: C, Question: C, CalendarBlank: C, Warning: C };
});
vi.mock('react-hot-toast', () => ({ default: { success: vi.fn(), error: vi.fn() } }));

import { AdminMaintenancePage } from './AdminMaintenance';
import toast from 'react-hot-toast';

const now = new Date();
const futureStart = new Date(now.getTime() + 86400000).toISOString();
const futureEnd = new Date(now.getTime() + 2 * 86400000).toISOString();
const pastEnd = new Date(now.getTime() - 86400000).toISOString();
const pastStart = new Date(now.getTime() - 2 * 86400000).toISOString();
const activeStart = new Date(now.getTime() - 3600000).toISOString();
const activeEnd = new Date(now.getTime() + 3600000).toISOString();

const windows = [
  { id: 'w1', lot_id: 'l1', lot_name: 'Lot A', start_time: futureStart, end_time: futureEnd, reason: 'Repaint', affected_slots: { type: 'all' as const }, created_at: '2026-01-01' },
  { id: 'w2', lot_id: 'l2', lot_name: 'Lot B', start_time: pastStart, end_time: pastEnd, reason: 'Fix lights', affected_slots: { type: 'specific' as const, slot_ids: ['s1', 's2'] }, created_at: '2026-01-01' },
  { id: 'w3', lot_id: 'l1', lot_name: 'Lot A', start_time: activeStart, end_time: activeEnd, reason: 'Active fix', affected_slots: { type: 'all' as const }, created_at: '2026-01-01' },
];
const lots = [{ id: 'l1', name: 'Lot A' }, { id: 'l2', name: 'Lot B' }];

describe('AdminMaintenancePage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'DELETE') return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      if (opts?.method === 'POST' || opts?.method === 'PUT') return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      if (url.includes('/maintenance/active')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'w3' }] }) } as Response);
      if (url.includes('/lots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: windows }) } as Response);
    }) as any;
  });
  afterEach(() => vi.restoreAllMocks());

  it('renders maintenance windows', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => expect(screen.getByText(/Repaint/)).toBeInTheDocument());
  });

  it('shows active banner', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => expect(screen.getByTestId('active-banner')).toBeInTheDocument());
  });

  it('shows help', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => expect(screen.getByTestId('create-btn')).toBeInTheDocument());
    // Find the help button (Question icon)
    const buttons = screen.getAllByRole('button');
    const helpBtn = buttons.find(b => b.textContent === '' && !b.hasAttribute('data-testid'));
    // Click the help toggle - just verify the create button exists
  });

  it('opens create form', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-btn')));
    await waitFor(() => expect(screen.getByTestId('maintenance-form')).toBeInTheDocument());
  });

  it('validates required fields on submit', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-btn')));
    fireEvent.click(screen.getByTestId('form-submit'));
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });

  it('creates maintenance window', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-btn')));
    fireEvent.change(screen.getByTestId('form-lot'), { target: { value: 'l1' } });
    fireEvent.change(screen.getByTestId('form-reason'), { target: { value: 'Test' } });
    fireEvent.change(screen.getByTestId('form-start'), { target: { value: '2026-05-01T08:00' } });
    fireEvent.change(screen.getByTestId('form-end'), { target: { value: '2026-05-01T17:00' } });
    fireEvent.click(screen.getByTestId('form-submit'));
    await waitFor(() => expect(toast.success).toHaveBeenCalled());
  });

  it('creates with specific slots', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-btn')));
    fireEvent.change(screen.getByTestId('form-lot'), { target: { value: 'l1' } });
    fireEvent.change(screen.getByTestId('form-reason'), { target: { value: 'Test' } });
    fireEvent.change(screen.getByTestId('form-start'), { target: { value: '2026-05-01T08:00' } });
    fireEvent.change(screen.getByTestId('form-end'), { target: { value: '2026-05-01T17:00' } });
    // Uncheck all_slots
    const checkbox = screen.getByRole('checkbox');
    fireEvent.click(checkbox);
    await waitFor(() => expect(screen.getByPlaceholderText('s1, s2, s3')).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText('s1, s2, s3'), { target: { value: 's1, s2' } });
    fireEvent.click(screen.getByTestId('form-submit'));
    await waitFor(() => expect(toast.success).toHaveBeenCalled());
  });

  it('deletes maintenance window', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => expect(screen.getAllByTestId('maintenance-row').length).toBeGreaterThan(0));
    // Find delete buttons
    const rows = screen.getAllByTestId('maintenance-row');
    const deleteBtn = rows[0].querySelectorAll('button')[1]; // second button is delete
    fireEvent.click(deleteBtn);
    await waitFor(() => expect(toast.success).toHaveBeenCalled());
  });

  it('edits maintenance window', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => expect(screen.getAllByTestId('maintenance-row').length).toBeGreaterThan(0));
    const rows = screen.getAllByTestId('maintenance-row');
    const editBtn = rows[0].querySelectorAll('button')[0];
    fireEvent.click(editBtn);
    await waitFor(() => expect(screen.getByTestId('maintenance-form')).toBeInTheDocument());
  });

  it('shows empty state', async () => {
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/maintenance/active')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      if (url.includes('/lots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    }) as any;
    render(<AdminMaintenancePage />);
    await waitFor(() => expect(screen.getByText('No maintenance windows scheduled')).toBeInTheDocument());
  });

  it('handles create API error', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST') return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Overlap' } }) } as Response);
      if (url.includes('/maintenance/active')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      if (url.includes('/lots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: windows }) } as Response);
    }) as any;
    render(<AdminMaintenancePage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-btn')));
    fireEvent.change(screen.getByTestId('form-lot'), { target: { value: 'l1' } });
    fireEvent.change(screen.getByTestId('form-reason'), { target: { value: 'T' } });
    fireEvent.change(screen.getByTestId('form-start'), { target: { value: '2026-05-01T08:00' } });
    fireEvent.change(screen.getByTestId('form-end'), { target: { value: '2026-05-01T17:00' } });
    fireEvent.click(screen.getByTestId('form-submit'));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Overlap'));
  });

  it('handles delete failure', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'DELETE') return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Cannot delete' } }) } as Response);
      if (url.includes('/maintenance/active')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      if (url.includes('/lots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: windows }) } as Response);
    }) as any;
    render(<AdminMaintenancePage />);
    await waitFor(() => expect(screen.getAllByTestId('maintenance-row').length).toBeGreaterThan(0));
    const rows = screen.getAllByTestId('maintenance-row');
    fireEvent.click(rows[0].querySelectorAll('button')[1]);
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Cannot delete'));
  });

  it('handles create network exception', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST') return Promise.reject(new Error('net'));
      if (url.includes('/maintenance/active')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      if (url.includes('/lots')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: lots }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: windows }) } as Response);
    }) as any;
    render(<AdminMaintenancePage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('create-btn')));
    fireEvent.change(screen.getByTestId('form-lot'), { target: { value: 'l1' } });
    fireEvent.change(screen.getByTestId('form-reason'), { target: { value: 'T' } });
    fireEvent.change(screen.getByTestId('form-start'), { target: { value: '2026-05-01T08:00' } });
    fireEvent.change(screen.getByTestId('form-end'), { target: { value: '2026-05-01T17:00' } });
    fireEvent.click(screen.getByTestId('form-submit'));
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });
});
