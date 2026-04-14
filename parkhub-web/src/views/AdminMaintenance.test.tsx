import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallbackOrOpts?: string | Record<string, any>) => {
      const map: Record<string, string> = {
        'maintenance.title': 'Maintenance Scheduling',
        'maintenance.subtitle': 'Schedule and manage maintenance windows',
        'maintenance.create': 'New',
        'maintenance.empty': 'No maintenance windows scheduled',
        'maintenance.allSlots': 'All slots',
        'maintenance.lot': 'Lot',
        'maintenance.reason': 'Reason',
        'maintenance.start': 'Start',
        'maintenance.end': 'End',
        'maintenance.help': 'Schedule maintenance windows.',
        'maintenance.activeBanner': 'Active maintenance',
        'maintenance.createTitle': 'New Maintenance Window',
        'maintenance.selectLot': 'Select lot...',
        'maintenance.created': 'Created',
        'maintenance.deleted': 'Cancelled',
        'common.save': 'Save',
        'common.cancel': 'Cancel',
        'common.error': 'Error',
      };
      return map[key] || (typeof fallbackOrOpts === 'string' ? fallbackOrOpts : key);
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Wrench: (props: any) => <span data-testid="icon-wrench" {...props} />,
  Plus: (props: any) => <span data-testid="icon-plus" {...props} />,
  Trash: (props: any) => <span data-testid="icon-trash" {...props} />,
  PencilSimple: (props: any) => <span data-testid="icon-pencil" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  CalendarBlank: (props: any) => <span data-testid="icon-calendar" {...props} />,
  Warning: (props: any) => <span data-testid="icon-warning" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: { success: vi.fn(), error: vi.fn() },
}));

import { AdminMaintenancePage } from './AdminMaintenance';

const sampleWindows = [
  {
    id: 'm1', lot_id: 'lot-1', lot_name: 'Lot A',
    start_time: '2026-04-01T08:00:00Z', end_time: '2026-04-01T12:00:00Z',
    reason: 'Elevator repair', affected_slots: { type: 'all' as const }, created_at: '2026-03-20T00:00:00Z',
  },
  {
    id: 'm2', lot_id: 'lot-2', lot_name: 'Lot B',
    start_time: '2026-04-05T06:00:00Z', end_time: '2026-04-05T18:00:00Z',
    reason: 'Painting', affected_slots: { type: 'specific' as const, slot_ids: ['s1', 's2'] }, created_at: '2026-03-21T00:00:00Z',
  },
];

const sampleLots = [
  { id: 'lot-1', name: 'Lot A' },
  { id: 'lot-2', name: 'Lot B' },
];

describe('AdminMaintenancePage', () => {
  beforeEach(() => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/maintenance/active')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (url.includes('/admin/maintenance')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleWindows }) } as Response);
      }
      if (url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
    }) as any;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the title', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => {
      expect(screen.getByText('Maintenance Scheduling')).toBeInTheDocument();
    });
  });

  it('renders maintenance rows', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => {
      const rows = screen.getAllByTestId('maintenance-row');
      expect(rows).toHaveLength(2);
    });
  });

  it('shows create button', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => {
      expect(screen.getByTestId('create-btn')).toBeInTheDocument();
    });
  });

  it('shows empty state when no windows', async () => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/maintenance/active')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (url.includes('/admin/maintenance')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
    }) as any;

    render(<AdminMaintenancePage />);
    await waitFor(() => {
      expect(screen.getByText('No maintenance windows scheduled')).toBeInTheDocument();
    });
  });

  it('opens create form on button click', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => expect(screen.getByTestId('create-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('create-btn'));
    expect(screen.getByTestId('maintenance-form')).toBeInTheDocument();
    expect(screen.getByTestId('form-lot')).toBeInTheDocument();
    expect(screen.getByTestId('form-reason')).toBeInTheDocument();
  });

  it('shows active maintenance banner when active', async () => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/maintenance/active')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [sampleWindows[0]] }) } as Response);
      }
      if (url.includes('/admin/maintenance')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleWindows }) } as Response);
      }
      if (url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
    }) as any;

    render(<AdminMaintenancePage />);
    await waitFor(() => {
      expect(screen.getByTestId('active-banner')).toBeInTheDocument();
    });
  });

  it('shows help text when help button clicked', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => screen.getByText('Maintenance Scheduling'));
    const helpBtns = document.querySelectorAll('[data-testid="icon-question"]');
    const helpBtn = helpBtns[0]?.closest('button');
    if (helpBtn) fireEvent.click(helpBtn);
    expect(screen.getByText('Schedule maintenance windows.')).toBeInTheDocument();
  });

  it('submits create form successfully', async () => {
    vi.mock('react-hot-toast', () => ({
      default: { success: vi.fn(), error: vi.fn() },
    }));

    render(<AdminMaintenancePage />);
    await waitFor(() => screen.getByTestId('create-btn'));
    fireEvent.click(screen.getByTestId('create-btn'));
    await waitFor(() => screen.getByTestId('maintenance-form'));

    // Fill form
    fireEvent.change(screen.getByTestId('form-lot'), { target: { value: 'lot-1' } });
    fireEvent.change(screen.getByTestId('form-reason'), { target: { value: 'Test maintenance' } });
    fireEvent.change(screen.getByTestId('form-start'), { target: { value: '2026-05-01T08:00' } });
    fireEvent.change(screen.getByTestId('form-end'), { target: { value: '2026-05-01T12:00' } });

    fireEvent.click(screen.getByTestId('form-submit'));
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/v1/admin/maintenance',
        expect.objectContaining({
          method: 'POST',
          body: expect.stringContaining('Test maintenance'),
        }),
      );
    });
  });

  it('shows validation error when required fields are missing', async () => {
    const toast = (await import('react-hot-toast')).default;
    render(<AdminMaintenancePage />);
    await waitFor(() => screen.getByTestId('create-btn'));
    fireEvent.click(screen.getByTestId('create-btn'));
    await waitFor(() => screen.getByTestId('maintenance-form'));

    // Submit without filling anything
    fireEvent.click(screen.getByTestId('form-submit'));
    await waitFor(() => {
      expect(toast.error).toHaveBeenCalled();
    });
  });

  it('opens edit form with pre-filled data', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => screen.getAllByTestId('maintenance-row'));

    // Click edit on first row
    const editBtns = document.querySelectorAll('[data-testid="icon-pencil"]');
    const firstEditBtn = editBtns[0]?.closest('button');
    if (firstEditBtn) fireEvent.click(firstEditBtn);

    await waitFor(() => {
      expect(screen.getByTestId('maintenance-form')).toBeInTheDocument();
      expect(screen.getByTestId('form-lot')).toHaveValue('lot-1');
      expect(screen.getByTestId('form-reason')).toHaveValue('Elevator repair');
    });
  });

  it('submits edit form with PUT method', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => screen.getAllByTestId('maintenance-row'));

    const editBtns = document.querySelectorAll('[data-testid="icon-pencil"]');
    const firstEditBtn = editBtns[0]?.closest('button');
    if (firstEditBtn) fireEvent.click(firstEditBtn);

    await waitFor(() => screen.getByTestId('maintenance-form'));
    fireEvent.change(screen.getByTestId('form-reason'), { target: { value: 'Updated reason' } });
    fireEvent.click(screen.getByTestId('form-submit'));

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/v1/admin/maintenance/m1',
        expect.objectContaining({ method: 'PUT' }),
      );
    });
  });

  it('deletes a maintenance window', async () => {
    const toast = (await import('react-hot-toast')).default;
    render(<AdminMaintenancePage />);
    await waitFor(() => screen.getAllByTestId('maintenance-row'));

    const trashBtns = document.querySelectorAll('[data-testid="icon-trash"]');
    const firstTrashBtn = trashBtns[0]?.closest('button');
    if (firstTrashBtn) fireEvent.click(firstTrashBtn);

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/v1/admin/maintenance/m1',
        expect.objectContaining({ method: 'DELETE' }),
      );
    });
  });

  it('cancels create form', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => screen.getByTestId('create-btn'));
    fireEvent.click(screen.getByTestId('create-btn'));
    await waitFor(() => screen.getByTestId('maintenance-form'));

    fireEvent.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(screen.queryByTestId('maintenance-form')).not.toBeInTheDocument();
    });
  });

  it('handles submit error from API', async () => {
    const toast = (await import('react-hot-toast')).default;
    global.fetch = vi.fn((url: string, opts?: any) => {
      if (url.includes('/maintenance/active')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (url.includes('/admin/maintenance') && !opts?.method) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleWindows }) } as Response);
      }
      if (url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      if (opts?.method === 'POST') {
        return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Overlap' } }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
    }) as any;

    render(<AdminMaintenancePage />);
    await waitFor(() => screen.getByTestId('create-btn'));
    fireEvent.click(screen.getByTestId('create-btn'));
    await waitFor(() => screen.getByTestId('maintenance-form'));

    fireEvent.change(screen.getByTestId('form-lot'), { target: { value: 'lot-1' } });
    fireEvent.change(screen.getByTestId('form-reason'), { target: { value: 'Test' } });
    fireEvent.change(screen.getByTestId('form-start'), { target: { value: '2026-05-01T08:00' } });
    fireEvent.change(screen.getByTestId('form-end'), { target: { value: '2026-05-01T12:00' } });
    fireEvent.click(screen.getByTestId('form-submit'));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('Overlap');
    });
  });

  it('handles submit network error', async () => {
    const toast = (await import('react-hot-toast')).default;
    global.fetch = vi.fn((url: string, opts?: any) => {
      if (url.includes('/maintenance/active')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (url.includes('/admin/maintenance') && !opts?.method) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleWindows }) } as Response);
      }
      if (url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      if (opts?.method === 'POST') {
        return Promise.reject(new Error('Network'));
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
    }) as any;

    render(<AdminMaintenancePage />);
    await waitFor(() => screen.getByTestId('create-btn'));
    fireEvent.click(screen.getByTestId('create-btn'));
    await waitFor(() => screen.getByTestId('maintenance-form'));

    fireEvent.change(screen.getByTestId('form-lot'), { target: { value: 'lot-1' } });
    fireEvent.change(screen.getByTestId('form-reason'), { target: { value: 'Test' } });
    fireEvent.change(screen.getByTestId('form-start'), { target: { value: '2026-05-01T08:00' } });
    fireEvent.change(screen.getByTestId('form-end'), { target: { value: '2026-05-01T12:00' } });
    fireEvent.click(screen.getByTestId('form-submit'));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('Error');
    });
  });

  it('handles delete error', async () => {
    const toast = (await import('react-hot-toast')).default;
    global.fetch = vi.fn((url: string, opts?: any) => {
      if (url.includes('/maintenance/active')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (url.includes('/admin/maintenance') && !opts?.method) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleWindows }) } as Response);
      }
      if (url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      if (opts?.method === 'DELETE') {
        return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Cannot delete' } }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
    }) as any;

    render(<AdminMaintenancePage />);
    await waitFor(() => screen.getAllByTestId('maintenance-row'));

    const trashBtns = document.querySelectorAll('[data-testid="icon-trash"]');
    const firstTrashBtn = trashBtns[0]?.closest('button');
    if (firstTrashBtn) fireEvent.click(firstTrashBtn);
    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('Cannot delete');
    });
  });

  it('submits with specific slot IDs when all_slots unchecked', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => screen.getByTestId('create-btn'));
    fireEvent.click(screen.getByTestId('create-btn'));
    await waitFor(() => screen.getByTestId('maintenance-form'));

    fireEvent.change(screen.getByTestId('form-lot'), { target: { value: 'lot-1' } });
    fireEvent.change(screen.getByTestId('form-reason'), { target: { value: 'Slot repair' } });
    fireEvent.change(screen.getByTestId('form-start'), { target: { value: '2026-05-01T08:00' } });
    fireEvent.change(screen.getByTestId('form-end'), { target: { value: '2026-05-01T12:00' } });

    // Uncheck all_slots
    const checkbox = document.querySelector('input[type="checkbox"]') as HTMLInputElement;
    fireEvent.click(checkbox);

    // Slot IDs input should appear
    await waitFor(() => {
      const slotInput = screen.getByPlaceholderText('s1, s2, s3');
      fireEvent.change(slotInput, { target: { value: 'slot-a, slot-b' } });
    });

    fireEvent.click(screen.getByTestId('form-submit'));
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/v1/admin/maintenance',
        expect.objectContaining({
          body: expect.stringContaining('slot-a'),
        }),
      );
    });
  });

  it('shows slot count for specific slot windows', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => {
      // sampleWindows[1] has specific slots with 2 slot_ids
      expect(screen.getByText(/2 slots/)).toBeInTheDocument();
    });
  });

  it('shows "all slots" for windows affecting all', async () => {
    render(<AdminMaintenancePage />);
    await waitFor(() => {
      expect(screen.getAllByTestId('maintenance-row')).toHaveLength(2);
    });
    // Check the text content includes "all slots"
    const rows = screen.getAllByTestId('maintenance-row');
    expect(rows[0].textContent).toContain('All slots');
  });
});
