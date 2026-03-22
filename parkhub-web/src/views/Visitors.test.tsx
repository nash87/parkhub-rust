import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallbackOrOpts?: string | Record<string, any>) => {
      const map: Record<string, string> = {
        'visitors.title': 'Visitor Pre-Registration',
        'visitors.subtitle': 'Pre-register visitors for easy check-in',
        'visitors.register': 'Register Visitor',
        'visitors.empty': 'No visitors registered',
        'visitors.name': 'Name',
        'visitors.email': 'Email',
        'visitors.plate': 'Vehicle Plate',
        'visitors.date': 'Visit Date',
        'visitors.purpose': 'Purpose',
        'visitors.registerTitle': 'Register a Visitor',
        'visitors.registered': 'Visitor registered',
        'visitors.checkedIn': 'Visitor checked in',
        'visitors.cancelled': 'Visitor cancelled',
        'visitors.requiredFields': 'Please fill required fields',
        'visitors.searchPlaceholder': 'Search visitors...',
        'visitors.help': 'Pre-register visitors with their details.',
        'visitors.aboutTitle': 'About Visitor Pre-Registration',
        'visitors.myVisitors': 'My Visitors',
        'visitors.allVisitors': 'All Visitors',
        'visitors.showQr': 'Show QR',
        'visitors.checkIn': 'Check In',
        'visitors.cancelVisitor': 'Cancel',
        'visitors.qrTitle': 'Visitor QR Code',
        'visitors.status.pending': 'Pending',
        'visitors.status.checked_in': 'Checked In',
        'visitors.status.expired': 'Expired',
        'visitors.status.cancelled': 'Cancelled',
        'visitors.purposePlaceholder': 'e.g. Business meeting',
        'common.save': 'Save',
        'common.cancel': 'Cancel',
        'common.close': 'Close',
        'common.error': 'Error',
        'common.status': 'Status',
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
  UserPlus: (props: any) => <span data-testid="icon-user-plus" {...props} />,
  QrCode: (props: any) => <span data-testid="icon-qr" {...props} />,
  Trash: (props: any) => <span data-testid="icon-trash" {...props} />,
  CheckCircle: (props: any) => <span data-testid="icon-check" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  MagnifyingGlass: (props: any) => <span data-testid="icon-search" {...props} />,
  CalendarBlank: (props: any) => <span data-testid="icon-calendar" {...props} />,
  Envelope: (props: any) => <span data-testid="icon-envelope" {...props} />,
}));

vi.mock('../context/AuthContext', () => ({
  useAuth: () => ({
    user: { id: 'user-1', name: 'Test User', role: 'admin' },
  }),
}));

import { VisitorsPage, AdminVisitorsPage } from './Visitors';

const sampleVisitors = [
  {
    id: 'v1',
    host_user_id: 'user-1',
    name: 'Alice Smith',
    email: 'alice@example.com',
    vehicle_plate: 'ABC-123',
    visit_date: '2026-04-15T09:00:00Z',
    purpose: 'Business meeting',
    status: 'pending' as const,
    qr_code: 'data:image/png;base64,abc',
    pass_url: '/visitor-pass/v1',
    checked_in_at: null,
    created_at: '2026-04-10T08:00:00Z',
  },
  {
    id: 'v2',
    host_user_id: 'user-1',
    name: 'Bob Jones',
    email: 'bob@example.com',
    vehicle_plate: null,
    visit_date: '2026-04-16T14:00:00Z',
    purpose: null,
    status: 'checked_in' as const,
    qr_code: null,
    pass_url: null,
    checked_in_at: '2026-04-16T14:05:00Z',
    created_at: '2026-04-12T10:00:00Z',
  },
];

describe('VisitorsPage', () => {
  beforeEach(() => {
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/api/v1/visitors')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleVisitors }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the visitors page with title', async () => {
    render(<VisitorsPage />);
    await waitFor(() => expect(screen.getByText('Visitor Pre-Registration')).toBeTruthy());
  });

  it('displays visitors after loading', async () => {
    render(<VisitorsPage />);
    await waitFor(() => {
      expect(screen.getByText('Alice Smith')).toBeTruthy();
      expect(screen.getByText('Bob Jones')).toBeTruthy();
    });
  });

  it('shows register form when clicking register button', async () => {
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    fireEvent.click(screen.getByText('Register Visitor'));
    await waitFor(() => expect(screen.getByText('Register a Visitor')).toBeTruthy());
  });

  it('filters visitors by search', async () => {
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    const input = screen.getByPlaceholderText('Search visitors...');
    fireEvent.change(input, { target: { value: 'alice' } });
    await waitFor(() => {
      expect(screen.getByText('Alice Smith')).toBeTruthy();
      expect(screen.queryByText('Bob Jones')).toBeNull();
    });
  });
});

describe('AdminVisitorsPage', () => {
  beforeEach(() => {
    global.fetch = vi.fn(() =>
      Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleVisitors }) } as Response)
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders admin visitor overview with stats', async () => {
    render(<AdminVisitorsPage />);
    await waitFor(() => {
      expect(screen.getByText('Alice Smith')).toBeTruthy();
      // Stats cards
      expect(screen.getByText('2')).toBeTruthy(); // total
    });
  });

  it('shows empty state when no visitors', async () => {
    global.fetch = vi.fn(() =>
      Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response)
    );
    render(<AdminVisitorsPage />);
    await waitFor(() => expect(screen.getByText('No visitors registered')).toBeTruthy());
  });
});
