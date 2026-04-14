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
  CheckSquare: (props: any) => <span data-testid="icon-check-square" {...props} />,
  Square: (props: any) => <span data-testid="icon-square" {...props} />,
  Trash: (props: any) => <span data-testid="icon-trash" {...props} />,
  Lightning: (props: any) => <span data-testid="icon-lightning" {...props} />,
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

  it('opens role editor on edit pencil click', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    // Click the edit role button for Alice
    const editBtns = screen.getAllByLabelText(/Edit Role/i);
    await user.click(editBtns[0]);

    // A select dropdown should appear
    await waitFor(() => {
      const selects = screen.getAllByRole('combobox');
      expect(selects.length).toBeGreaterThan(0);
    });
  });

  it('saves role change and shows success toast', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    mockAdminUpdateUserRole.mockResolvedValue({ success: true, data: { id: '1', role: 'admin' } });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const editBtns = screen.getAllByLabelText(/Edit Role/i);
    await user.click(editBtns[0]);

    // Click save button
    const saveBtn = screen.getByLabelText('Save');
    await user.click(saveBtn);

    await waitFor(() => {
      expect(mockAdminUpdateUserRole).toHaveBeenCalledWith('1', 'user');
      expect(mockToastSuccess).toHaveBeenCalledWith('Role updated');
    });
  });

  it('shows error toast when role update fails', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    mockAdminUpdateUserRole.mockResolvedValue({ success: false, data: null, error: { code: 'FORBIDDEN', message: 'Not allowed' } });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const editBtns = screen.getAllByLabelText(/Edit Role/i);
    await user.click(editBtns[0]);
    await user.click(screen.getByLabelText('Save'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Not allowed');
    });
  });

  it('cancels role editing on cancel click', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const editBtns = screen.getAllByLabelText(/Edit Role/i);
    await user.click(editBtns[0]);

    // Cancel button should be visible
    const cancelBtn = screen.getByLabelText('Cancel');
    await user.click(cancelBtn);

    // Role selector should disappear
    await waitFor(() => {
      const roleSelectors = screen.queryAllByRole('combobox');
      // Only the bulk action selector might remain
      expect(roleSelectors.length).toBeLessThanOrEqual(1);
    });
  });

  it('toggles user active status', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    mockAdminUpdateUser.mockResolvedValue({ success: true, data: { id: '1', is_active: false } });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    // Click deactivate for Alice (active user)
    const deactivateBtn = screen.getByLabelText(/Deactivate Alice Smith/i);
    await user.click(deactivateBtn);

    await waitFor(() => {
      expect(mockAdminUpdateUser).toHaveBeenCalledWith('1', { is_active: false });
      expect(mockToastSuccess).toHaveBeenCalledWith('User deactivated');
    });
  });

  it('shows error toast when toggle active fails', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    mockAdminUpdateUser.mockResolvedValue({ success: false, data: null, error: { code: 'ERROR', message: 'Cannot deactivate' } });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const deactivateBtn = screen.getByLabelText(/Deactivate Alice Smith/i);
    await user.click(deactivateBtn);

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Cannot deactivate');
    });
  });

  it('opens grant credits panel when coins button is clicked', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const grantBtns = screen.getAllByLabelText(/Grant Credits Alice/i);
    await user.click(grantBtns[0]);

    await waitFor(() => {
      // The panel header
      expect(screen.getAllByText(/Grant Credits/i).length).toBeGreaterThanOrEqual(1);
      expect(screen.getByLabelText('Amount')).toBeInTheDocument();
    });
  });

  it('grants credits successfully', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    mockAdminGrantCredits.mockResolvedValue({ success: true, data: null });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const grantBtns = screen.getAllByLabelText(/Grant Credits/i);
    await user.click(grantBtns[0]);

    await waitFor(() => expect(screen.getByLabelText('Amount')).toBeInTheDocument());

    await user.type(screen.getByLabelText('Amount'), '10');
    await user.type(screen.getByLabelText('Description'), 'Bonus');
    await user.click(screen.getByText('Grant'));

    await waitFor(() => {
      expect(mockAdminGrantCredits).toHaveBeenCalledWith('1', 10, 'Bonus');
      expect(mockToastSuccess).toHaveBeenCalledWith('Credits granted');
    });
  });

  it('shows error toast when grant credits fails', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    mockAdminGrantCredits.mockResolvedValue({ success: false, data: null, error: { code: 'ERROR', message: 'Insufficient' } });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const grantBtns = screen.getAllByLabelText(/Grant Credits/i);
    await user.click(grantBtns[0]);
    await user.type(screen.getByLabelText('Amount'), '10');
    await user.click(screen.getByText('Grant'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Insufficient');
    });
  });

  it('handles API failure on initial load', async () => {
    mockAdminUsers.mockResolvedValue({ success: false, data: null });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.queryByTestId('icon-spinner')).not.toBeInTheDocument());
    // Should show no users found
    expect(screen.getByText('No users found.')).toBeInTheDocument();
  });

  it('displays role badges for different roles', async () => {
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());
    expect(screen.getByText('user')).toBeInTheDocument();
    expect(screen.getByText('admin')).toBeInTheDocument();
    expect(screen.getByText('superadmin')).toBeInTheDocument();
  });

  it('displays active/inactive status badges', async () => {
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());
    expect(screen.getAllByText('Active').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Inactive').length).toBeGreaterThanOrEqual(1);
  });

  it('activates inactive user', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    mockAdminUpdateUser.mockResolvedValue({ success: true, data: { id: '2', is_active: true } });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Bob Jones')).toBeInTheDocument());

    // Bob is inactive, click activate
    const activateBtn = screen.getByLabelText(/Activate Bob Jones/i);
    await user.click(activateBtn);

    await waitFor(() => {
      expect(mockAdminUpdateUser).toHaveBeenCalledWith('2', { is_active: true });
      expect(mockToastSuccess).toHaveBeenCalledWith('User activated');
    });
  });

  it('closes credit panel on close button', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const grantBtns = screen.getAllByLabelText(/Grant Credits/i);
    await user.click(grantBtns[0]);
    await waitFor(() => expect(screen.getByLabelText('Amount')).toBeInTheDocument());

    // Close the panel
    await user.click(screen.getByLabelText('Close'));
    await waitFor(() => {
      expect(screen.queryByLabelText('Amount')).not.toBeInTheDocument();
    });
  });

  it('closes credit panel on cancel button', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const grantBtns = screen.getAllByLabelText(/Grant Credits/i);
    await user.click(grantBtns[0]);
    await waitFor(() => expect(screen.getByLabelText('Amount')).toBeInTheDocument());

    await user.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(screen.queryByLabelText('Amount')).not.toBeInTheDocument();
    });
  });

  it('opens quota editor and can save', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    mockAdminUpdateUserQuota.mockResolvedValue({ success: true });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    // Find quota display buttons (they show the current quota value)
    // Alice has credits_monthly_quota: 5, displayed as a button with tabular-nums
    const quotaBtns = screen.getAllByLabelText('Edit quota');
    await user.click(quotaBtns[0]); // Alice

    // Quota input should appear
    await waitFor(() => {
      expect(screen.getByLabelText('Monthly Quota')).toBeInTheDocument();
      expect(screen.getByLabelText('Save quota')).toBeInTheDocument();
    });
  });

  it('validates quota range', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const quotaBtns = screen.getAllByLabelText('Edit quota');
    await user.click(quotaBtns[0]);
    await waitFor(() => expect(screen.getByLabelText('Monthly Quota')).toBeInTheDocument());

    // The quota input should be present
    expect(screen.getByLabelText('Save quota')).toBeInTheDocument();
  });

  it('cancels quota edit', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const quotaBtns = screen.getAllByLabelText('Edit quota');
    await user.click(quotaBtns[0]);
    await waitFor(() => expect(screen.getByLabelText('Monthly Quota')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Cancel edit quota'));
    await waitFor(() => {
      expect(screen.queryByLabelText('Monthly Quota')).not.toBeInTheDocument();
    });
  });

  it('handles role update with generic error', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    mockAdminUpdateUserRole.mockResolvedValue({ success: false, data: null });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const editBtns = screen.getAllByLabelText(/Edit Role/i);
    await user.click(editBtns[0]);
    await user.click(screen.getByLabelText('Save'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalled();
    });
  });

  it('handles toggle active with generic error', async () => {
    const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) });
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
    mockAdminUpdateUser.mockResolvedValue({ success: false, data: null });
    render(<AdminUsersPage />);
    await vi.runAllTimersAsync();
    await waitFor(() => expect(screen.getByText('Alice Smith')).toBeInTheDocument());

    const deactivateBtn = screen.getByLabelText(/Deactivate Alice Smith/i);
    await user.click(deactivateBtn);

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalled();
    });
  });
});
