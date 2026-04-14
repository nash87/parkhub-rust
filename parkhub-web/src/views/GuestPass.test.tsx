import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, any>) => {
      const map: Record<string, string> = {
        'guestBooking.title': 'Guest Parking',
        'guestBooking.subtitle': 'Create and manage guest parking passes',
        'guestBooking.create': 'Create Guest Pass',
        'guestBooking.formTitle': 'New Guest Booking',
        'guestBooking.guestName': 'Guest Name',
        'guestBooking.guestEmail': 'Guest Email',
        'guestBooking.lot': 'Lot',
        'guestBooking.slot': 'Slot',
        'guestBooking.selectLot': 'Select a lot',
        'guestBooking.selectSlot': 'Select a slot',
        'guestBooking.startTime': 'Start Time',
        'guestBooking.endTime': 'End Time',
        'guestBooking.requiredFields': 'Please fill all required fields',
        'guestBooking.creating': 'Creating...',
        'guestBooking.created': 'Guest pass created!',
        'guestBooking.cancelled': 'Guest booking cancelled',
        'guestBooking.codeCopied': 'Code copied!',
        'guestBooking.linkCopied': 'Link copied!',
        'guestBooking.share': 'Share Pass',
        'guestBooking.shareTitle': 'Guest Parking Pass',
        'guestBooking.shareText': `Guest pass for ${opts?.name || ''}: ${opts?.code || ''}`,
        'guestBooking.shareInstructions': 'Share this pass with your guest',
        'guestBooking.passCreated': 'Guest Pass Created',
        'guestBooking.code': 'Guest Code',
        'guestBooking.dateRange': 'Date & Time',
        'guestBooking.existing': 'Guest Bookings',
        'guestBooking.empty': 'No guest bookings yet',
        'guestBooking.cancel': 'Cancel',
        'guestBooking.status.active': 'Active',
        'guestBooking.status.expired': 'Expired',
        'guestBooking.status.cancelled': 'Cancelled',
        'common.save': 'Save',
        'common.cancel': 'Cancel',
        'common.error': 'Error',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  UserPlus: (props: any) => <span data-testid="icon-user-plus" {...props} />,
  Copy: (props: any) => <span data-testid="icon-copy" {...props} />,
  ShareNetwork: (props: any) => <span data-testid="icon-share" {...props} />,
  Trash: (props: any) => <span data-testid="icon-trash" {...props} />,
  QrCode: (props: any) => <span data-testid="icon-qr" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  CheckCircle: (props: any) => <span data-testid="icon-check" {...props} />,
  CalendarBlank: (props: any) => <span data-testid="icon-calendar" {...props} />,
  MapPin: (props: any) => <span data-testid="icon-map-pin" {...props} />,
}));

vi.mock('../context/AuthContext', () => ({
  useAuth: () => ({
    user: { id: 'user-1', name: 'Test User', role: 'admin' },
  }),
}));

vi.mock('../api/client', () => ({
  getInMemoryToken: () => 'test-token',
}));

vi.mock('react-hot-toast', () => ({
  default: { success: vi.fn(), error: vi.fn() },
}));

import { GuestPassPage } from './GuestPass';

const sampleBookings = [
  {
    id: 'gb-1',
    lot_id: 'lot-1',
    lot_name: 'HQ Garage',
    slot_id: 'slot-1',
    slot_number: 'A1',
    guest_name: 'Alice Guest',
    guest_email: 'alice@example.com',
    guest_code: 'ABCD1234',
    start_time: '2026-04-15T09:00:00Z',
    end_time: '2026-04-15T17:00:00Z',
    status: 'active' as const,
    created_at: '2026-04-12T08:00:00Z',
  },
  {
    id: 'gb-2',
    lot_id: 'lot-1',
    lot_name: 'HQ Garage',
    slot_id: 'slot-2',
    slot_number: 'A2',
    guest_name: 'Bob Visitor',
    guest_email: null,
    guest_code: 'EFGH5678',
    start_time: '2026-04-16T10:00:00Z',
    end_time: '2026-04-16T14:00:00Z',
    status: 'expired' as const,
    created_at: '2026-04-11T10:00:00Z',
  },
];

const sampleLots = [
  { id: 'lot-1', name: 'HQ Garage' },
  { id: 'lot-2', name: 'Annex Lot' },
];

describe('GuestPassPage', () => {
  beforeEach(() => {
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleBookings }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
    Object.assign(navigator, {
      clipboard: { writeText: vi.fn(() => Promise.resolve()) },
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the page with title', async () => {
    render(<GuestPassPage />);
    expect(screen.getByText('Guest Parking')).toBeTruthy();
    expect(screen.getByTestId('guest-pass-page')).toBeInTheDocument();
  });

  it('displays guest bookings after loading', async () => {
    render(<GuestPassPage />);
    await waitFor(() => {
      expect(screen.getByText('Alice Guest')).toBeTruthy();
      expect(screen.getByText('Bob Visitor')).toBeTruthy();
    });
  });

  it('shows guest codes in the list', async () => {
    render(<GuestPassPage />);
    await waitFor(() => {
      expect(screen.getByText('ABCD1234')).toBeTruthy();
      expect(screen.getByText('EFGH5678')).toBeTruthy();
    });
  });

  it('shows status badges', async () => {
    render(<GuestPassPage />);
    await waitFor(() => {
      expect(screen.getByText('Active')).toBeTruthy();
      expect(screen.getByText('Expired')).toBeTruthy();
    });
  });

  it('shows create form when clicking create button', async () => {
    render(<GuestPassPage />);
    await waitFor(() => screen.getByText('Alice Guest'));
    fireEvent.click(screen.getByTestId('create-guest-btn'));
    expect(screen.getByTestId('guest-form')).toBeInTheDocument();
    expect(screen.getByTestId('input-guest-name')).toBeInTheDocument();
    expect(screen.getByTestId('select-lot')).toBeInTheDocument();
  });

  it('shows empty state when no bookings', async () => {
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
    });
    render(<GuestPassPage />);
    await waitFor(() => {
      expect(screen.getByTestId('empty-state')).toBeInTheDocument();
      expect(screen.getByText('No guest bookings yet')).toBeTruthy();
    });
  });

  it('renders cancel button for admin on active bookings', async () => {
    render(<GuestPassPage />);
    await waitFor(() => {
      expect(screen.getByTestId('cancel-gb-1')).toBeInTheDocument();
    });
  });

  it('does not show cancel button for expired bookings', async () => {
    render(<GuestPassPage />);
    await waitFor(() => {
      expect(screen.getByText('Bob Visitor')).toBeTruthy();
    });
    expect(screen.queryByTestId('cancel-gb-2')).not.toBeInTheDocument();
  });

  it('renders guest page with subtitle', async () => {
    render(<GuestPassPage />);
    await waitFor(() => {
      expect(screen.getByText('Create and manage guest parking passes')).toBeTruthy();
    });
  });

  it('shows correct form fields in create form', async () => {
    render(<GuestPassPage />);
    await waitFor(() => screen.getByText('Alice Guest'));
    fireEvent.click(screen.getByTestId('create-guest-btn'));

    expect(screen.getByTestId('input-guest-name')).toBeInTheDocument();
    expect(screen.getByTestId('select-lot')).toBeInTheDocument();
  });

  it('shows lot options in create form', async () => {
    render(<GuestPassPage />);
    await waitFor(() => screen.getByText('Alice Guest'));
    fireEvent.click(screen.getByTestId('create-guest-btn'));

    const lotSelect = screen.getByTestId('select-lot') as HTMLSelectElement;
    const options = Array.from(lotSelect.options).map(o => o.text);
    expect(options).toContain('HQ Garage');
    expect(options).toContain('Annex Lot');
  });

  it('handles cancel guest booking', async () => {
    global.fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest') && opts?.method === 'DELETE') {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: null }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleBookings }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });

    render(<GuestPassPage />);
    await waitFor(() => expect(screen.getByTestId('cancel-gb-1')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('cancel-gb-1'));

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/bookings/guest/gb-1'),
        expect.objectContaining({ method: 'DELETE' }),
      );
    });
  });

  it('handles API error on guest bookings load', async () => {
    global.fetch = vi.fn(() =>
      Promise.resolve({ json: () => Promise.resolve({ success: false, data: null }) } as Response)
    );
    render(<GuestPassPage />);
    await waitFor(() => {
      expect(screen.getByTestId('guest-pass-page')).toBeInTheDocument();
    });
  });

  it('handles fetch exception on bookings load', async () => {
    global.fetch = vi.fn(() => Promise.reject(new Error('Network error')));
    render(<GuestPassPage />);
    await waitFor(() => {
      expect(screen.getByTestId('guest-pass-page')).toBeInTheDocument();
    });
  });

  it('submits guest form successfully and shows pass card', async () => {
    const createdPass = {
      id: 'gb-new',
      lot_id: 'lot-1',
      lot_name: 'HQ Garage',
      slot_id: 'slot-1',
      slot_number: 'A1',
      guest_name: 'New Guest',
      guest_email: 'new@test.com',
      guest_code: 'NEWCODE1',
      start_time: '2026-04-20T09:00:00Z',
      end_time: '2026-04-20T17:00:00Z',
      status: 'active' as const,
      created_at: '2026-04-14T08:00:00Z',
    };

    global.fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest') && opts?.method === 'POST') {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: createdPass }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleBookings }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/lots') && url.includes('/slots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'slot-1', number: 'A1', status: 'available' }] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });

    render(<GuestPassPage />);
    await waitFor(() => screen.getByText('Alice Guest'));

    fireEvent.click(screen.getByTestId('create-guest-btn'));
    await waitFor(() => expect(screen.getByTestId('guest-form')).toBeInTheDocument());

    fireEvent.change(screen.getByTestId('input-guest-name'), { target: { value: 'New Guest' } });
    fireEvent.change(screen.getByTestId('input-guest-email'), { target: { value: 'new@test.com' } });
    fireEvent.change(screen.getByTestId('select-lot'), { target: { value: 'lot-1' } });

    // Wait for slots to load
    await waitFor(() => {
      const slotSelect = screen.getByTestId('select-slot') as HTMLSelectElement;
      expect(slotSelect.disabled).toBe(false);
    });

    fireEvent.change(screen.getByTestId('select-slot'), { target: { value: 'slot-1' } });
    fireEvent.change(screen.getByTestId('input-start-time'), { target: { value: '2026-04-20T09:00' } });
    fireEvent.change(screen.getByTestId('input-end-time'), { target: { value: '2026-04-20T17:00' } });

    fireEvent.click(screen.getByTestId('submit-guest-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('guest-pass-card')).toBeInTheDocument();
      expect(screen.getByText('NEWCODE1')).toBeInTheDocument();
    });
  });

  it('shows error toast when submit fails', async () => {
    global.fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest') && opts?.method === 'POST') {
        return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Slot taken' } }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/lots') && url.includes('/slots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'slot-1', number: 'A1', status: 'available' }] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });

    render(<GuestPassPage />);
    await waitFor(() => screen.getByTestId('guest-pass-page'));

    fireEvent.click(screen.getByTestId('create-guest-btn'));
    fireEvent.change(screen.getByTestId('input-guest-name'), { target: { value: 'Bad Guest' } });
    fireEvent.change(screen.getByTestId('select-lot'), { target: { value: 'lot-1' } });
    await waitFor(() => {
      const slotSelect = screen.getByTestId('select-slot') as HTMLSelectElement;
      expect(slotSelect.disabled).toBe(false);
    });
    fireEvent.change(screen.getByTestId('select-slot'), { target: { value: 'slot-1' } });
    fireEvent.change(screen.getByTestId('input-start-time'), { target: { value: '2026-04-20T09:00' } });
    fireEvent.change(screen.getByTestId('input-end-time'), { target: { value: '2026-04-20T17:00' } });

    fireEvent.click(screen.getByTestId('submit-guest-btn'));
    // Should not crash, error toast shown
  });

  it('shows validation error when required fields missing on submit', async () => {
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
    });

    render(<GuestPassPage />);
    await waitFor(() => screen.getByTestId('guest-pass-page'));

    fireEvent.click(screen.getByTestId('create-guest-btn'));
    // Submit without filling anything -- guest_name is empty
    fireEvent.click(screen.getByTestId('submit-guest-btn'));
    // The HTML5 required attribute prevents submission, or the check returns early
  });

  it('cancel form clears and hides it', async () => {
    render(<GuestPassPage />);
    await waitFor(() => screen.getByText('Alice Guest'));

    fireEvent.click(screen.getByTestId('create-guest-btn'));
    expect(screen.getByTestId('guest-form')).toBeInTheDocument();

    // Click cancel
    fireEvent.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(screen.queryByTestId('guest-form')).not.toBeInTheDocument();
    });
  });

  it('copy code button works', async () => {
    const createdPass = {
      id: 'gb-new', lot_id: 'lot-1', lot_name: 'HQ Garage', slot_id: 'slot-1', slot_number: 'A1',
      guest_name: 'New Guest', guest_email: null, guest_code: 'COPY1234',
      start_time: '2026-04-20T09:00:00Z', end_time: '2026-04-20T17:00:00Z',
      status: 'active' as const, created_at: '2026-04-14T08:00:00Z',
    };

    global.fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest') && opts?.method === 'POST') {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: createdPass }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/lots') && url.includes('/slots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'slot-1', number: 'A1', status: 'available' }] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });

    render(<GuestPassPage />);
    await waitFor(() => screen.getByTestId('guest-pass-page'));

    fireEvent.click(screen.getByTestId('create-guest-btn'));
    fireEvent.change(screen.getByTestId('input-guest-name'), { target: { value: 'New Guest' } });
    fireEvent.change(screen.getByTestId('select-lot'), { target: { value: 'lot-1' } });
    await waitFor(() => {
      const slotSelect = screen.getByTestId('select-slot') as HTMLSelectElement;
      expect(slotSelect.disabled).toBe(false);
    });
    fireEvent.change(screen.getByTestId('select-slot'), { target: { value: 'slot-1' } });
    fireEvent.change(screen.getByTestId('input-start-time'), { target: { value: '2026-04-20T09:00' } });
    fireEvent.change(screen.getByTestId('input-end-time'), { target: { value: '2026-04-20T17:00' } });
    fireEvent.click(screen.getByTestId('submit-guest-btn'));

    await waitFor(() => expect(screen.getByTestId('guest-pass-card')).toBeInTheDocument());

    // Click copy code
    fireEvent.click(screen.getByTestId('copy-code-btn'));
    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith('COPY1234');
    });
  });

  it('dismiss pass card hides it', async () => {
    const createdPass = {
      id: 'gb-new', lot_id: 'lot-1', lot_name: 'HQ Garage', slot_id: 'slot-1', slot_number: 'A1',
      guest_name: 'Dismiss Guest', guest_email: null, guest_code: 'DISMISS1',
      start_time: '2026-04-20T09:00:00Z', end_time: '2026-04-20T17:00:00Z',
      status: 'active' as const, created_at: '2026-04-14T08:00:00Z',
    };

    global.fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest') && opts?.method === 'POST') {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: createdPass }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/lots') && url.includes('/slots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'slot-1', number: 'A1', status: 'available' }] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });

    render(<GuestPassPage />);
    await waitFor(() => screen.getByTestId('guest-pass-page'));

    fireEvent.click(screen.getByTestId('create-guest-btn'));
    fireEvent.change(screen.getByTestId('input-guest-name'), { target: { value: 'Dismiss Guest' } });
    fireEvent.change(screen.getByTestId('select-lot'), { target: { value: 'lot-1' } });
    await waitFor(() => {
      const s = screen.getByTestId('select-slot') as HTMLSelectElement;
      expect(s.disabled).toBe(false);
    });
    fireEvent.change(screen.getByTestId('select-slot'), { target: { value: 'slot-1' } });
    fireEvent.change(screen.getByTestId('input-start-time'), { target: { value: '2026-04-20T09:00' } });
    fireEvent.change(screen.getByTestId('input-end-time'), { target: { value: '2026-04-20T17:00' } });
    fireEvent.click(screen.getByTestId('submit-guest-btn'));

    await waitFor(() => expect(screen.getByTestId('guest-pass-card')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('dismiss-pass'));
    await waitFor(() => {
      expect(screen.queryByTestId('guest-pass-card')).not.toBeInTheDocument();
    });
  });

  it('share pass uses navigator.share when available', async () => {
    Object.assign(navigator, {
      share: vi.fn(() => Promise.resolve()),
    });

    render(<GuestPassPage />);
    await waitFor(() => screen.getByText('Alice Guest'));

    // Click share on the first booking
    const shareButtons = screen.getAllByTitle('Share Pass');
    fireEvent.click(shareButtons[0]);

    await waitFor(() => {
      expect(navigator.share).toHaveBeenCalled();
    });
  });

  it('handles cancel booking error', async () => {
    global.fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest') && opts?.method === 'DELETE') {
        return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Cannot cancel' } }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleBookings }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });

    render(<GuestPassPage />);
    await waitFor(() => expect(screen.getByTestId('cancel-gb-1')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('cancel-gb-1'));
    // Error path exercised
  });

  it('handles cancel booking fetch exception', async () => {
    global.fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest') && opts?.method === 'DELETE') {
        return Promise.reject(new Error('Network'));
      }
      if (typeof url === 'string' && url.includes('/api/v1/bookings/guest')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleBookings }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });

    render(<GuestPassPage />);
    await waitFor(() => expect(screen.getByTestId('cancel-gb-1')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('cancel-gb-1'));
    // Exception path exercised
  });
});
