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

vi.mock('react-hot-toast', () => ({
  default: { success: vi.fn(), error: vi.fn() },
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
    (global as any).fetch = vi.fn((url: string) => {
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
    (global as any).fetch = vi.fn(() =>
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
    (global as any).fetch = vi.fn(() =>
      Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response)
    );
    render(<AdminVisitorsPage />);
    await waitFor(() => expect(screen.getByText('No visitors registered')).toBeTruthy());
  });

  it('shows help tooltip when clicked', async () => {
    render(<AdminVisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    fireEvent.click(screen.getByLabelText('Help'));
    expect(screen.getByText('Pre-register visitors with their details.')).toBeTruthy();
  });

  it('filters by search input', async () => {
    render(<AdminVisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    const searchInput = screen.getByPlaceholderText('Search visitors...');
    fireEvent.change(searchInput, { target: { value: 'bob' } });
    // loadData should be called again since search is a dependency of loadData
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(expect.stringContaining('search=bob'));
    });
  });

  it('filters by status dropdown', async () => {
    render(<AdminVisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    const statusSelect = screen.getByDisplayValue('visitors.allStatuses');
    fireEvent.change(statusSelect, { target: { value: 'pending' } });
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(expect.stringContaining('status=pending'));
    });
  });

  it('shows correct stats for visitors', async () => {
    render(<AdminVisitorsPage />);
    await waitFor(() => {
      // total = 2, pending = 1, checked_in = 1
      expect(screen.getByText('2')).toBeTruthy();
      expect(screen.getByText('1', { selector: '.text-2xl.font-bold.text-amber-600' })).toBeTruthy();
    });
  });
});

describe('VisitorsPage - extended', () => {
  beforeEach(() => {
    (global as any).fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/api/v1/visitors') && !url.includes('admin')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleVisitors }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/api/v1/admin/visitors')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleVisitors }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/register')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { id: 'v-new' } }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/check-in')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      }
      if (opts?.method === 'DELETE') {
        return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows help tooltip when help button clicked', async () => {
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    fireEvent.click(screen.getByLabelText('Help'));
    expect(screen.getByText('Pre-register visitors with their details.')).toBeTruthy();
  });

  it('switches view mode to admin when admin user clicks All Visitors', async () => {
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    fireEvent.click(screen.getByText('All Visitors'));
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith('/api/v1/admin/visitors');
    });
  });

  it('switches back to My Visitors view', async () => {
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    fireEvent.click(screen.getByText('All Visitors'));
    fireEvent.click(screen.getByText('My Visitors'));
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith('/api/v1/visitors');
    });
  });

  it('submits registration form successfully', async () => {
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    fireEvent.click(screen.getByText('Register Visitor'));
    await waitFor(() => screen.getByText('Register a Visitor'));

    // Fill form fields using the form structure
    const form = screen.getByText('Register a Visitor').closest('div');
    const textInputs = form!.querySelectorAll('input[type="text"]');
    const emailInput = form!.querySelector('input[type="email"]');
    const dateInput = form!.querySelector('input[type="datetime-local"]');

    // name
    fireEvent.change(textInputs[0]!, { target: { value: 'Test Visitor' } });
    // email
    if (emailInput) fireEvent.change(emailInput, { target: { value: 'test@example.com' } });
    // date
    if (dateInput) fireEvent.change(dateInput, { target: { value: '2026-04-20T10:00' } });
    // vehicle plate (optional)
    if (textInputs[1]) fireEvent.change(textInputs[1]!, { target: { value: 'XY-999' } });
    // purpose (optional)
    if (textInputs[2]) fireEvent.change(textInputs[2]!, { target: { value: 'Meeting' } });

    fireEvent.click(screen.getByText('Save'));
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/v1/visitors/register',
        expect.objectContaining({ method: 'POST' }),
      );
    });
  });

  it('shows validation error when required fields missing', async () => {
    const toast = await import('react-hot-toast');
    const errSpy = vi.spyOn(toast.default, 'error');
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    fireEvent.click(screen.getByText('Register Visitor'));
    await waitFor(() => screen.getByText('Register a Visitor'));
    // Submit form directly bypassing HTML required validation
    const form = document.querySelector('form');
    if (form) fireEvent.submit(form);
    await waitFor(() => {
      expect(errSpy).toHaveBeenCalledWith('Please fill required fields');
    });
    errSpy.mockRestore();
  });

  it('shows error when registration server returns failure', async () => {
    const toast = await import('react-hot-toast');
    const errSpy = vi.spyOn(toast.default, 'error');
    (global as any).fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/visitors/register')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Server fail' } }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleVisitors }) } as Response);
    }) as any;

    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    fireEvent.click(screen.getByText('Register Visitor'));
    await waitFor(() => screen.getByText('Register a Visitor'));

    const form = screen.getByText('Register a Visitor').closest('div')!;
    const textInputs = form.querySelectorAll('input[type="text"]');
    const emailInput = form.querySelector('input[type="email"]');
    const dateInput = form.querySelector('input[type="datetime-local"]');
    fireEvent.change(textInputs[0]!, { target: { value: 'X' } });
    if (emailInput) fireEvent.change(emailInput, { target: { value: 'x@x.com' } });
    if (dateInput) fireEvent.change(dateInput, { target: { value: '2026-04-20T10:00' } });
    fireEvent.click(screen.getByText('Save'));
    await waitFor(() => {
      expect(errSpy).toHaveBeenCalledWith('Server fail');
    });
    errSpy.mockRestore();
  });

  it('catches network errors during registration', async () => {
    const toast = await import('react-hot-toast');
    const errSpy = vi.spyOn(toast.default, 'error');
    (global as any).fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/visitors/register')) {
        return Promise.reject(new Error('boom'));
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleVisitors }) } as Response);
    }) as any;

    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    fireEvent.click(screen.getByText('Register Visitor'));
    await waitFor(() => screen.getByText('Register a Visitor'));

    const form = screen.getByText('Register a Visitor').closest('div')!;
    const textInputs = form.querySelectorAll('input[type="text"]');
    const emailInput = form.querySelector('input[type="email"]');
    const dateInput = form.querySelector('input[type="datetime-local"]');
    fireEvent.change(textInputs[0]!, { target: { value: 'X' } });
    if (emailInput) fireEvent.change(emailInput, { target: { value: 'x@x.com' } });
    if (dateInput) fireEvent.change(dateInput, { target: { value: '2026-04-20T10:00' } });
    fireEvent.click(screen.getByText('Save'));
    await waitFor(() => {
      expect(errSpy).toHaveBeenCalled();
    });
    errSpy.mockRestore();
  });

  it('cancels registration form', async () => {
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    fireEvent.click(screen.getByText('Register Visitor'));
    await waitFor(() => screen.getByText('Register a Visitor'));
    fireEvent.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(screen.queryByText('Register a Visitor')).toBeNull();
    });
  });

  it('opens QR modal when QR button clicked', async () => {
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    // Alice has qr_code
    const qrBtns = screen.getAllByTitle('Show QR');
    fireEvent.click(qrBtns[0]!);
    expect(screen.getByText('Visitor QR Code')).toBeTruthy();
  });

  it('closes QR modal when clicking backdrop', async () => {
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    const qrBtns = screen.getAllByTitle('Show QR');
    fireEvent.click(qrBtns[0]!);
    expect(screen.getByText('Visitor QR Code')).toBeTruthy();
    // Click backdrop
    const backdrop = screen.getByText('Visitor QR Code').closest('.fixed');
    if (backdrop) fireEvent.click(backdrop);
    await waitFor(() => {
      expect(screen.queryByText('Visitor QR Code')).toBeNull();
    });
  });

  it('closes QR modal when clicking Close button', async () => {
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    const qrBtns = screen.getAllByTitle('Show QR');
    fireEvent.click(qrBtns[0]!);
    fireEvent.click(screen.getByText('Close'));
    await waitFor(() => {
      expect(screen.queryByText('Visitor QR Code')).toBeNull();
    });
  });

  it('checks in a pending visitor', async () => {
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    const checkInBtns = screen.getAllByTitle('Check In');
    fireEvent.click(checkInBtns[0]!);
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/v1/visitors/v1/check-in',
        expect.objectContaining({ method: 'PUT' }),
      );
    });
  });

  it('cancels a pending visitor', async () => {
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    const cancelBtns = screen.getAllByTitle('Cancel');
    fireEvent.click(cancelBtns[0]!);
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/v1/visitors/v1',
        expect.objectContaining({ method: 'DELETE' }),
      );
    });
  });

  it('shows empty state when no visitors match search', async () => {
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    const searchInput = screen.getByPlaceholderText('Search visitors...');
    fireEvent.change(searchInput, { target: { value: 'zzz-no-match' } });
    await waitFor(() => {
      expect(screen.getByText('No visitors registered')).toBeTruthy();
    });
  });

  it('filters by vehicle plate', async () => {
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    const searchInput = screen.getByPlaceholderText('Search visitors...');
    fireEvent.change(searchInput, { target: { value: 'ABC' } });
    await waitFor(() => {
      expect(screen.getByText('Alice Smith')).toBeTruthy();
      expect(screen.queryByText('Bob Jones')).toBeNull();
    });
  });

  it('handles check-in error', async () => {
    (global as any).fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/api/v1/visitors') && !opts?.method) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleVisitors }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/check-in')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Already checked in' } }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    const checkInBtns = screen.getAllByTitle('Check In');
    fireEvent.click(checkInBtns[0]!);
    // Error toast would be shown
  });

  it('handles cancel error', async () => {
    (global as any).fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/api/v1/visitors') && !opts?.method) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleVisitors }) } as Response);
      }
      if (opts?.method === 'DELETE') {
        return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Cannot cancel' } }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    const cancelBtns = screen.getAllByTitle('Cancel');
    fireEvent.click(cancelBtns[0]!);
    // Error toast would be shown
  });

  it('handles network failure on check-in', async () => {
    (global as any).fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/api/v1/visitors') && !opts?.method) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleVisitors }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/check-in')) {
        return Promise.reject(new Error('Network error'));
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    const checkInBtns = screen.getAllByTitle('Check In');
    fireEvent.click(checkInBtns[0]!);
    // Should not crash
  });

  it('handles network failure on cancel', async () => {
    (global as any).fetch = vi.fn((url: string, opts?: any) => {
      if (typeof url === 'string' && url.includes('/api/v1/visitors') && !opts?.method) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleVisitors }) } as Response);
      }
      if (opts?.method === 'DELETE') {
        return Promise.reject(new Error('Network error'));
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
    render(<VisitorsPage />);
    await waitFor(() => screen.getByText('Alice Smith'));
    const cancelBtns = screen.getAllByTitle('Cancel');
    fireEvent.click(cancelBtns[0]!);
    // Should not crash
  });

  it('shows loading state initially', () => {
    (global as any).fetch = vi.fn(() => new Promise(() => {})); // Never resolves
    render(<VisitorsPage />);
    expect(screen.getByText('Visitor Pre-Registration')).toBeTruthy();
  });

  it('displays visitor purpose and vehicle plate info', async () => {
    render(<VisitorsPage />);
    await waitFor(() => {
      expect(screen.getByText('Business meeting')).toBeTruthy();
      expect(screen.getByText('ABC-123')).toBeTruthy();
    });
  });
});
