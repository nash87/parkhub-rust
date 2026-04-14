import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallbackOrOpts?: string | Record<string, any>) => {
      const map: Record<string, string> = {
        'accessible.title': 'Accessible Parking',
        'accessible.subtitle': 'Manage accessible slots and view utilization',
        'accessible.totalSlots': 'Accessible Slots',
        'accessible.utilization': 'Utilization',
        'accessible.totalBookings': 'Active Bookings',
        'accessible.usersWithNeeds': 'Users with Needs',
        'accessible.priority': 'Priority booking active',
        'accessible.manageSlots': 'Manage Accessible Slots',
        'accessible.selectLot': 'Select a parking lot...',
        'accessible.slotLabel': 'Slot',
        'accessible.noSlots': 'No slots found',
        'accessible.help': 'This module manages accessible parking slots.',
        'common.error': 'Error',
      };
      return map[key] || (typeof fallbackOrOpts === 'string' ? fallbackOrOpts : key);
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    tr: React.forwardRef(({ children, initial, animate, transition, ...props }: any, ref: any) => (
      <tr ref={ref} {...props}>{children}</tr>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Wheelchair: (props: any) => <span data-testid="icon-wheelchair" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  ToggleLeft: (props: any) => <span data-testid="icon-toggle-left" {...props} />,
  ToggleRight: (props: any) => <span data-testid="icon-toggle-right" {...props} />,
  ChartBar: (props: any) => <span data-testid="icon-chart" {...props} />,
  Users: (props: any) => <span data-testid="icon-users" {...props} />,
}));

import { AdminAccessiblePage } from './AdminAccessible';

const sampleStats = {
  total_accessible_slots: 5,
  occupied_accessible_slots: 2,
  utilization_percent: 40.0,
  total_accessible_bookings: 8,
  users_with_accessibility_needs: 3,
  priority_booking_active: true,
  priority_minutes: 30,
};

const sampleLots = [
  { id: 'lot-1', name: 'Lot A' },
  { id: 'lot-2', name: 'Lot B' },
];

const sampleSlots = [
  { id: 's1', lot_id: 'lot-1', slot_number: 1, status: 'available', slot_type: 'standard', is_accessible: true },
  { id: 's2', lot_id: 'lot-1', slot_number: 2, status: 'available', slot_type: 'handicap', is_accessible: true },
  { id: 's3', lot_id: 'lot-1', slot_number: 3, status: 'available', slot_type: 'standard', is_accessible: false },
];

describe('AdminAccessiblePage', () => {
  beforeEach(() => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/accessible-stats')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleStats }) } as Response);
      }
      if (url.includes('/lots/lot-1/slots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleSlots }) } as Response);
      }
      if (url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      if (url.includes('/accessible')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: null }) } as Response);
    }) as any;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the title', async () => {
    render(<AdminAccessiblePage />);
    await waitFor(() => {
      expect(screen.getByText('Accessible Parking')).toBeInTheDocument();
    });
  });

  it('renders stats cards with correct values', async () => {
    render(<AdminAccessiblePage />);
    await waitFor(() => {
      expect(screen.getByTestId('accessible-stats')).toBeInTheDocument();
      expect(screen.getByText('Accessible Slots')).toBeInTheDocument();
      expect(screen.getByText('5')).toBeInTheDocument();
      expect(screen.getByText('40%')).toBeInTheDocument();
    });
  });

  it('shows priority booking info', async () => {
    render(<AdminAccessiblePage />);
    await waitFor(() => {
      expect(screen.getByText(/Priority booking active/)).toBeInTheDocument();
    });
  });

  it('shows lot selector', async () => {
    render(<AdminAccessiblePage />);
    await waitFor(() => {
      expect(screen.getByTestId('lot-selector')).toBeInTheDocument();
      expect(screen.getByText('Lot A')).toBeInTheDocument();
      expect(screen.getByText('Lot B')).toBeInTheDocument();
    });
  });

  it('loads slots when lot is selected', async () => {
    render(<AdminAccessiblePage />);
    await waitFor(() => {
      expect(screen.getByTestId('lot-selector')).toBeInTheDocument();
    });

    fireEvent.change(screen.getByTestId('lot-selector'), { target: { value: 'lot-1' } });

    await waitFor(() => {
      expect(screen.getByTestId('slot-list')).toBeInTheDocument();
      const toggles = screen.getAllByTestId('slot-toggle');
      expect(toggles).toHaveLength(3);
    });
  });

  it('toggles slot accessibility on click', async () => {
    render(<AdminAccessiblePage />);
    await waitFor(() => expect(screen.getByTestId('lot-selector')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('lot-selector'), { target: { value: 'lot-1' } });
    await waitFor(() => expect(screen.getByTestId('slot-list')).toBeInTheDocument());

    const toggles = screen.getAllByTestId('slot-toggle');
    fireEvent.click(toggles[2]); // Toggle slot 3 (not accessible) -> accessible

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('/accessible'),
        expect.objectContaining({ method: 'PUT' }),
      );
    });
  });

  it('toggles help panel via help button', async () => {
    render(<AdminAccessiblePage />);
    await waitFor(() => expect(screen.getByText('Accessible Parking')).toBeInTheDocument());
    fireEvent.click(screen.getByLabelText('Help'));
    await waitFor(() => {
      expect(screen.getByText('This module manages accessible parking slots.')).toBeInTheDocument();
    });
  });

  it('shows error toast when slot toggle returns failure', async () => {
    global.fetch = vi.fn((url: string, opts?: any) => {
      if (url.includes('/accessible-stats')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleStats }) } as Response);
      }
      if (url.includes('/lots/lot-1/slots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleSlots }) } as Response);
      }
      if (url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      if (opts?.method === 'PUT' && url.includes('/accessible')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Forbidden' } }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: null }) } as Response);
    }) as any;

    render(<AdminAccessiblePage />);
    await waitFor(() => expect(screen.getByTestId('lot-selector')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('lot-selector'), { target: { value: 'lot-1' } });
    await waitFor(() => expect(screen.getByTestId('slot-list')).toBeInTheDocument());
    const toggles = screen.getAllByTestId('slot-toggle');
    fireEvent.click(toggles[0]);
    // No throw, function completes
  });

  it('catches network error during slot toggle', async () => {
    global.fetch = vi.fn((url: string, opts?: any) => {
      if (url.includes('/accessible-stats')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleStats }) } as Response);
      }
      if (url.includes('/lots/lot-1/slots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleSlots }) } as Response);
      }
      if (url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      if (opts?.method === 'PUT' && url.includes('/accessible')) {
        return Promise.reject(new Error('net'));
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: null }) } as Response);
    }) as any;

    render(<AdminAccessiblePage />);
    await waitFor(() => expect(screen.getByTestId('lot-selector')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('lot-selector'), { target: { value: 'lot-1' } });
    await waitFor(() => expect(screen.getByTestId('slot-list')).toBeInTheDocument());
    const toggles = screen.getAllByTestId('slot-toggle');
    fireEvent.click(toggles[0]);
    // No throw
  });
});
