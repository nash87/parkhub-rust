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
});
