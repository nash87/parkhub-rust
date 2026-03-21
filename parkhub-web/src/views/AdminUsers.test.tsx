import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockAdminUsers = vi.fn();
const mockAdminUpdateUserRole = vi.fn();
const mockAdminUpdateUser = vi.fn();
const mockAdminGrantCredits = vi.fn();
const mockAdminUpdateUserQuota = vi.fn();
const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    adminUsers: (...args: any[]) => mockAdminUsers(...args),
    adminUpdateUserRole: (...args: any[]) => mockAdminUpdateUserRole(...args),
    adminUpdateUser: (...args: any[]) => mockAdminUpdateUser(...args),
    adminGrantCredits: (...args: any[]) => mockAdminGrantCredits(...args),
    adminUpdateUserQuota: (...args: any[]) => mockAdminUpdateUserQuota(...args),
  },
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    tr: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <tr ref={ref} {...props}>{children}</tr>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Users: (props: any) => <span data-testid="icon-users" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  MagnifyingGlass: (props: any) => <span data-testid="icon-search" {...props} />,
  Coins: (props: any) => <span data-testid="icon-coins" {...props} />,
  PencilSimple: (props: any) => <span data-testid="icon-pencil" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  Gauge: (props: any) => <span data-testid="icon-gauge" {...props} />,
  UserMinus: (props: any) => <span data-testid="icon-user-minus" {...props} />,
  UserPlus: (props: any) => <span data-testid="icon-user-plus" {...props} />,
  CaretUp: (props: any) => <span data-testid="icon-caret-up" {...props} />,
  CaretDown: (props: any) => <span data-testid="icon-caret-down" {...props} />,
  DownloadSimple: (props: any) => <span data-testid="icon-download" {...props} />,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'admin.users': 'Users',
        'admin.searchUsers': 'Search users',
        'admin.clearSearch': 'Clear search',
        'admin.noUsersMatch': 'No users match your search.',
        'admin.noUsersFound': 'No users found.',
        'admin.editRole': 'Edit Role',
        'admin.credits': 'Credits',
        'admin.monthlyQuota': 'Monthly Quota',
        'admin.status': 'Status',
        'admin.active': 'Active',
        'admin.inactive': 'Inactive',
        'admin.grantCredits': 'Grant Credits',
        'admin.grantCreditsFor': 'Grant credits for user',
        'admin.grantingTo': 'Granting to',
        'admin.amount': 'Amount',
        'admin.description': 'Description',
        'admin.grant': 'Grant',
        'admin.roleUpdated': 'Role updated',
        'admin.roleUpdateFailed': 'Role update failed',
        'admin.userDeactivated': 'User deactivated',
        'admin.userActivated': 'User activated',
        'admin.userUpdateFailed': 'User update failed',
        'admin.creditsGranted': 'Credits granted',
        'admin.creditsGrantFailed': 'Credits grant failed',
        'admin.quotaUpdated': 'Quota updated',
        'admin.quotaUpdateFailed': 'Quota update failed',
        'admin.quotaRange': 'Quota must be 0-999',
        'admin.saveQuota': 'Save quota',
        'admin.cancelEditQuota': 'Cancel edit quota',
        'admin.editQuota': 'Edit quota',
        'admin.deactivate': 'Deactivate',
        'admin.activate': 'Activate',
        'common.loading': 'Loading',
        'common.save': 'Save',
        'common.cancel': 'Cancel',
        'common.close': 'Close',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('react-hot-toast', () => ({
  default: {
    success: (...args: any[]) => mockToastSuccess(...args),
    error: (...args: any[]) => mockToastError(...args),
  },
}));

const sampleUsers = [
  {
    id: '1',
    name: 'Alice Smith',
    email: 'alice@example.com',
    username: 'alice',
    role: 'user',
    is_active: true,
    credits_balance: 10,
    credits_monthly_quota: 5,
  },
  {
    id: '2',
    name: 'Bob Jones',
    email: 'bob@example.com',
    username: 'bjones',
    role: 'admin',
    is_active: false,
    credits_balance: 3,
    credits_monthly_quota: 10,
  },
  {
    id: '3',
    name: 'Carol Admin',
    email: 'carol@acme.org',
    username: 'carol',
    role: 'superadmin',
    is_active: true,
    credits_balance: 0,
    credits_monthly_quota: 20,
  },
];

import { AdminUsersPage } from './AdminUsers';

describe('AdminUsersPage', () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    mockAdminUsers.mockClear();
    mockAdminUpdateUserRole.mockClear();
    mockAdminUpdateUser.mockClear();
    mockAdminGrantCredits.mockClear();
    mockAdminUpdateUserQuota.mockClear();
    mockToastSuccess.mockClear();
    mockToastError.mockClear();
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('renders loading spinner initially', () => {
    mockAdminUsers.mockReturnValue(new Promise(() => {}));
    render(<AdminUsersPage />);
    expect(screen.getByTestId('icon-spinner')).toBeInTheDocument();
  });

  it('renders user list after load', async () => {
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => {
      expect(screen.getByText('Alice Smith')).toBeInTheDocument();
    });
    expect(screen.getByText('Bob Jones')).toBeInTheDocument();
    expect(screen.getByText('Carol Admin')).toBeInTheDocument();
  });

  it('renders search input', async () => {
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.queryByTestId('icon-spinner')).not.toBeInTheDocument());
    expect(screen.getByRole('textbox', { name: /search users/i })).toBeInTheDocument();
  });

  it('debounces search filter — does not filter mid-typing', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const input = screen.getByRole('textbox', { name: /search users/i });
    // Type 'bob' quickly — debounce not yet fired
    await user.type(input, 'bob');
    // Before 200 ms: all users still visible
    expect(screen.getByText('Alice Smith')).toBeInTheDocument();

    // Fire the debounce
    act(() => { vi.advanceTimersByTime(200); });
    await waitFor(() => {
      expect(screen.queryByText('Alice Smith')).not.toBeInTheDocument();
      expect(screen.getByText('Bob Jones')).toBeInTheDocument();
    });
  });

  it('filters by name (DataTable global filter on column accessors)', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const input = screen.getByRole('textbox', { name: /search users/i });
    await user.type(input, 'Carol');
    act(() => { vi.advanceTimersByTime(200); });
    await waitFor(() => {
      expect(screen.getByText('Carol Admin')).toBeInTheDocument();
      expect(screen.queryByText('Alice Smith')).not.toBeInTheDocument();
    });
  });

  it('filters by role', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const input = screen.getByRole('textbox', { name: /search users/i });
    await user.type(input, 'superadmin');
    act(() => { vi.advanceTimersByTime(200); });
    await waitFor(() => {
      expect(screen.getByText('Carol Admin')).toBeInTheDocument();
      expect(screen.queryByText('Alice Smith')).not.toBeInTheDocument();
    });
  });

  it('shows "No users match your search." when filter is empty', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const input = screen.getByRole('textbox', { name: /search users/i });
    await user.type(input, 'zzznomatch');
    act(() => { vi.advanceTimersByTime(200); });
    await waitFor(() => {
      expect(screen.getByText('No users match your search.')).toBeInTheDocument();
    });
  });

  it('shows clear button when search has text', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.queryByTestId('icon-spinner')).not.toBeInTheDocument());

    const input = screen.getByRole('textbox', { name: /search users/i });
    expect(screen.queryByRole('button', { name: /clear search/i })).not.toBeInTheDocument();

    await user.type(input, 'alice');
    expect(screen.getByRole('button', { name: /clear search/i })).toBeInTheDocument();
  });

  it('clear button resets search and shows all users', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const input = screen.getByRole('textbox', { name: /search users/i });
    await user.type(input, 'bob');
    act(() => { vi.advanceTimersByTime(200); });
    await waitFor(() => expect(screen.queryByText('Alice Smith')).not.toBeInTheDocument());

    const clearBtn = screen.getByRole('button', { name: /clear search/i });
    await user.click(clearBtn);
    act(() => { vi.advanceTimersByTime(200); });
    await waitFor(() => {
      expect(screen.getByText('Alice Smith')).toBeInTheDocument();
      expect(screen.getByText('Bob Jones')).toBeInTheDocument();
    });
  });

  it('shows user count in header', async () => {
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText(`(${sampleUsers.length})`)).toBeInTheDocument());
  });
});
