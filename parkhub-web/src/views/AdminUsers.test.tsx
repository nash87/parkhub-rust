import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

const mockAdminUsers = vi.fn();
const mockAdminUpdateUserRole = vi.fn();
const mockAdminUpdateUser = vi.fn();
const mockAdminGrantCredits = vi.fn();
const mockAdminUpdateUserQuota = vi.fn();
const mockAdminBulkUpdate = vi.fn();
const mockAdminBulkDelete = vi.fn();
const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    adminUsers: (...a: any[]) => mockAdminUsers(...a),
    adminUpdateUserRole: (...a: any[]) => mockAdminUpdateUserRole(...a),
    adminUpdateUser: (...a: any[]) => mockAdminUpdateUser(...a),
    adminGrantCredits: (...a: any[]) => mockAdminGrantCredits(...a),
    adminUpdateUserQuota: (...a: any[]) => mockAdminUpdateUserQuota(...a),
    adminBulkUpdate: (...a: any[]) => mockAdminBulkUpdate(...a),
    adminBulkDelete: (...a: any[]) => mockAdminBulkDelete(...a),
  },
}));

vi.mock('react-hot-toast', () => ({
  default: {
    success: (...a: any[]) => mockToastSuccess(...a),
    error: (...a: any[]) => mockToastError(...a),
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
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  MagnifyingGlass: (props: any) => <span data-testid="icon-search" {...props} />,
  Coins: (props: any) => <span data-testid="icon-coins" {...props} />,
  PencilSimple: (props: any) => <span data-testid="icon-pencil" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  UserMinus: (props: any) => <span data-testid="icon-user-minus" {...props} />,
  UserPlus: (props: any) => <span data-testid="icon-user-plus" {...props} />,
  Lightning: (props: any) => <span data-testid="icon-lightning" {...props} />,
}));

vi.mock('../components/ui/DataTable', () => ({
  DataTable: ({ data, columns, searchValue, emptyMessage }: any) => {
    return (
      <div data-testid="data-table">
        {data.length === 0 ? <p>{emptyMessage}</p> : (
          <table>
            <tbody>
              {data.map((row: any) => (
                <tr key={row.id} data-testid={`user-row-${row.id}`}>
                  {columns.map((col: any, i: number) => {
                    if (col.cell && col.accessorKey) {
                      const value = row[col.accessorKey];
                      const info = {
                        getValue: () => value,
                        row: { original: row },
                      };
                      return <td key={i}>{typeof col.cell === 'function' ? col.cell(info) : String(value)}</td>;
                    }
                    if (col.id === 'actions' && col.cell) {
                      const info = { row: { original: row } };
                      return <td key={i}>{col.cell(info)}</td>;
                    }
                    return <td key={i}>{row[col.accessorKey] ?? ''}</td>;
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    );
  },
}));

vi.mock('../components/ui/ConfirmDialog', () => ({
  ConfirmDialog: ({ open, onConfirm, onCancel, title, message }: any) =>
    open ? (
      <div data-testid="confirm-dialog">
        <p>{title}</p>
        <p>{message}</p>
        <button onClick={onConfirm}>Confirm</button>
        <button onClick={onCancel}>CancelDialog</button>
      </div>
    ) : null,
}));

import { AdminUsersPage } from './AdminUsers';

const sampleUsers = [
  { id: 'u-1', name: 'Alice', email: 'alice@test.com', role: 'admin', credits_balance: 10, credits_monthly_quota: 5, is_active: true },
  { id: 'u-2', name: 'Bob', email: 'bob@test.com', role: 'user', credits_balance: 3, credits_monthly_quota: 5, is_active: false },
];

describe('AdminUsersPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAdminUsers.mockResolvedValue({ success: true, data: sampleUsers });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading spinner initially', () => {
    mockAdminUsers.mockReturnValue(new Promise(() => {}));
    render(<AdminUsersPage />);
    expect(screen.getByRole('status')).toBeInTheDocument();
  });

  it('renders user list after loading', async () => {
    render(<AdminUsersPage />);
    await waitFor(() => {
      expect(screen.getByText('Alice')).toBeInTheDocument();
      expect(screen.getByText('Bob')).toBeInTheDocument();
    });
  });

  it('shows user count', async () => {
    render(<AdminUsersPage />);
    await waitFor(() => {
      expect(screen.getByText('(2)')).toBeInTheDocument();
    });
  });

  it('search input is rendered', async () => {
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());
    expect(screen.getByLabelText('Search users...')).toBeInTheDocument();
  });

  it('clear search button works', async () => {
    const user = userEvent.setup();
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    const searchInput = screen.getByLabelText('Search users...');
    await user.type(searchInput, 'alice');

    const clearBtn = screen.getByLabelText('Clear search');
    await user.click(clearBtn);

    expect(searchInput).toHaveValue('');
  });

  it('starts role editing when edit button is clicked', async () => {
    const user = userEvent.setup();
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    const editBtn = screen.getByLabelText('Edit role Alice');
    await user.click(editBtn);

    await waitFor(() => {
      expect(screen.getByDisplayValue('admin')).toBeInTheDocument();
    });
  });

  it('saves role successfully', async () => {
    const user = userEvent.setup();
    mockAdminUpdateUserRole.mockResolvedValue({ success: true });
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Edit role Alice'));
    await waitFor(() => expect(screen.getByDisplayValue('admin')).toBeInTheDocument());

    await user.selectOptions(screen.getByDisplayValue('admin'), 'user');
    await user.click(screen.getByLabelText('Save'));

    await waitFor(() => {
      expect(mockAdminUpdateUserRole).toHaveBeenCalledWith('u-1', 'user');
      expect(mockToastSuccess).toHaveBeenCalled();
    });
  });

  it('shows error toast on role save failure', async () => {
    const user = userEvent.setup();
    mockAdminUpdateUserRole.mockResolvedValue({ success: false, error: { message: 'Forbidden' } });
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Edit role Alice'));
    await waitFor(() => expect(screen.getByDisplayValue('admin')).toBeInTheDocument());
    await user.click(screen.getByLabelText('Save'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Forbidden');
    });
  });

  it('cancels role editing', async () => {
    const user = userEvent.setup();
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Edit role Alice'));
    await waitFor(() => expect(screen.getByDisplayValue('admin')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Cancel'));
    await waitFor(() => {
      expect(screen.queryByDisplayValue('admin')).not.toBeInTheDocument();
    });
  });

  it('toggles user active status (deactivate)', async () => {
    const user = userEvent.setup();
    mockAdminUpdateUser.mockResolvedValue({ success: true });
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Deactivate Alice'));

    await waitFor(() => {
      expect(mockAdminUpdateUser).toHaveBeenCalledWith('u-1', { is_active: false });
      expect(mockToastSuccess).toHaveBeenCalled();
    });
  });

  it('shows error on toggle active failure', async () => {
    const user = userEvent.setup();
    mockAdminUpdateUser.mockResolvedValue({ success: false, error: { message: 'Denied' } });
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Deactivate Alice'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Denied');
    });
  });

  it('activates inactive user', async () => {
    const user = userEvent.setup();
    mockAdminUpdateUser.mockResolvedValue({ success: true });
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Bob')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Activate Bob'));

    await waitFor(() => {
      expect(mockAdminUpdateUser).toHaveBeenCalledWith('u-2', { is_active: true });
    });
  });

  it('opens credit grant modal', async () => {
    const user = userEvent.setup();
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Grant Credits Alice'));
    await waitFor(() => {
      expect(screen.getByText('Grant Credits')).toBeInTheDocument();
      expect(screen.getByLabelText('Amount')).toBeInTheDocument();
    });
  });

  it('grants credits successfully', async () => {
    const user = userEvent.setup();
    mockAdminGrantCredits.mockResolvedValue({ success: true });
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Grant Credits Alice'));
    await waitFor(() => expect(screen.getByLabelText('Amount')).toBeInTheDocument());

    await user.type(screen.getByLabelText('Amount'), '10');
    await user.type(screen.getByLabelText('Description (optional)'), 'Bonus');
    await user.click(screen.getByText('Grant'));

    await waitFor(() => {
      expect(mockAdminGrantCredits).toHaveBeenCalledWith('u-1', 10, 'Bonus');
      expect(mockToastSuccess).toHaveBeenCalled();
    });
  });

  it('shows error on credit grant failure', async () => {
    const user = userEvent.setup();
    mockAdminGrantCredits.mockResolvedValue({ success: false, error: { message: 'Insufficient' } });
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Grant Credits Alice'));
    await user.type(screen.getByLabelText('Amount'), '10');
    await user.click(screen.getByText('Grant'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Insufficient');
    });
  });

  it('closes credit modal on cancel', async () => {
    const user = userEvent.setup();
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Grant Credits Alice'));
    await waitFor(() => expect(screen.getByLabelText('Amount')).toBeInTheDocument());

    await user.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(screen.queryByLabelText('Amount')).not.toBeInTheDocument();
    });
  });

  it('closes credit modal on X button', async () => {
    const user = userEvent.setup();
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Grant Credits Alice'));
    await waitFor(() => expect(screen.getByLabelText('Amount')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Close'));
    await waitFor(() => {
      expect(screen.queryByLabelText('Amount')).not.toBeInTheDocument();
    });
  });

  it('handles API failure on load gracefully', async () => {
    mockAdminUsers.mockResolvedValue({ success: false, data: null });
    render(<AdminUsersPage />);
    await waitFor(() => {
      expect(screen.getByTestId('data-table')).toBeInTheDocument();
    });
  });

  it('grants credits without description', async () => {
    const user = userEvent.setup();
    mockAdminGrantCredits.mockResolvedValue({ success: true });
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Grant Credits Alice'));
    await user.type(screen.getByLabelText('Amount'), '5');
    await user.click(screen.getByText('Grant'));

    await waitFor(() => {
      expect(mockAdminGrantCredits).toHaveBeenCalledWith('u-1', 5, undefined);
    });
  });

  it('shows role badges with correct text', async () => {
    mockAdminUsers.mockResolvedValue({
      success: true,
      data: [
        { id: 'u-3', name: 'Super', email: 's@test.com', role: 'superadmin', credits_balance: 0, credits_monthly_quota: 5, is_active: true },
        ...sampleUsers,
      ],
    });
    render(<AdminUsersPage />);
    await waitFor(() => {
      expect(screen.getByText('Super')).toBeInTheDocument();
      expect(screen.getByText('superadmin')).toBeInTheDocument();
    });
  });

  it('starts quota editing and saves valid value', async () => {
    const user = userEvent.setup();
    mockAdminUpdateUserQuota.mockResolvedValue({ success: true });
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    // Click quota value to start editing
    const quotaBtn = screen.getByLabelText('Edit quota for Alice');
    await user.click(quotaBtn);

    await waitFor(() => {
      expect(screen.getByLabelText('Monthly quota')).toBeInTheDocument();
    });

    const quotaInput = screen.getByLabelText('Monthly quota');
    await user.clear(quotaInput);
    await user.type(quotaInput, '10');
    await user.click(screen.getByLabelText('Save quota'));

    await waitFor(() => {
      expect(mockAdminUpdateUserQuota).toHaveBeenCalledWith('u-1', 10);
      expect(mockToastSuccess).toHaveBeenCalled();
    });
  });

  it('quota edit rejects invalid range (>999)', async () => {
    const user = userEvent.setup();
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Edit quota for Alice'));
    const quotaInput = screen.getByLabelText('Monthly quota');
    await user.clear(quotaInput);
    await user.type(quotaInput, '1500');
    await user.click(screen.getByLabelText('Save quota'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalled();
    });
    expect(mockAdminUpdateUserQuota).not.toHaveBeenCalled();
  });

  it('quota edit shows error on API failure', async () => {
    const user = userEvent.setup();
    mockAdminUpdateUserQuota.mockResolvedValue({ success: false, error: { message: 'Failed' } });
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Edit quota for Alice'));
    const quotaInput = screen.getByLabelText('Monthly quota');
    await user.clear(quotaInput);
    await user.type(quotaInput, '10');
    await user.click(screen.getByLabelText('Save quota'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Failed');
    });
  });

  it('quota edit can be cancelled', async () => {
    const user = userEvent.setup();
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Edit quota for Alice'));
    await waitFor(() => expect(screen.getByLabelText('Monthly quota')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Cancel editing quota'));
    await waitFor(() => {
      expect(screen.queryByLabelText('Monthly quota')).not.toBeInTheDocument();
    });
  });

  it('active badge shows Active for active users', async () => {
    render(<AdminUsersPage />);
    await waitFor(() => {
      expect(screen.getByText('Active')).toBeInTheDocument();
      expect(screen.getByText('Inactive')).toBeInTheDocument();
    });
  });

  it('grant button is disabled when amount is empty', async () => {
    const user = userEvent.setup();
    render(<AdminUsersPage />);
    await waitFor(() => expect(screen.getByText('Alice')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Grant Credits Alice'));
    await waitFor(() => expect(screen.getByLabelText('Amount')).toBeInTheDocument());

    const grantBtn = screen.getByText('Grant').closest('button')!;
    expect(grantBtn).toBeDisabled();
  });
});
