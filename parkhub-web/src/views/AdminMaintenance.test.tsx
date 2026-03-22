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
});
