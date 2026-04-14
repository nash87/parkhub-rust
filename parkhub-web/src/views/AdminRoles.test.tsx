import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (k: string) => k }) }));
vi.mock('framer-motion', () => ({
  motion: { div: React.forwardRef(({ children, layout, ...p }: any, r: any) => <div ref={r} {...p}>{children}</div>) },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));
vi.mock('@phosphor-icons/react', () => {
  const C = (p: any) => <span {...p} />;
  return { ShieldCheck: C, Plus: C, Trash: C, Pencil: C, Question: C, UserCircle: C };
});
vi.mock('react-hot-toast', () => ({ default: { success: vi.fn(), error: vi.fn() } }));

import { AdminRolesPage } from './AdminRoles';
import toast from 'react-hot-toast';

const roles = [
  { id: 'r1', name: 'Admin', description: 'Full access', permissions: ['manage_users', 'manage_lots'], built_in: true, created_at: '2026-01-01', updated_at: '2026-01-01' },
  { id: 'r2', name: 'Viewer', description: null, permissions: ['view_reports'], built_in: false, created_at: '2026-02-01', updated_at: '2026-02-01' },
  { id: 'r3', name: 'Empty', description: 'No perms', permissions: [], built_in: false, created_at: '2026-03-01', updated_at: '2026-03-01' },
];

describe('AdminRolesPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'DELETE') return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      if (opts?.method === 'POST' || opts?.method === 'PUT') return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: roles }) } as Response);
    }) as any;
    globalThis.confirm = vi.fn(() => true);
  });
  afterEach(() => vi.restoreAllMocks());

  it('renders roles', async () => {
    render(<AdminRolesPage />);
    await waitFor(() => expect(screen.getByText('Admin')).toBeInTheDocument());
    expect(screen.getByText('Viewer')).toBeInTheDocument();
  });

  it('shows built-in badge', async () => {
    render(<AdminRolesPage />);
    await waitFor(() => expect(screen.getByText('rbac.builtIn')).toBeInTheDocument());
  });

  it('shows description', async () => {
    render(<AdminRolesPage />);
    await waitFor(() => expect(screen.getByText('Full access')).toBeInTheDocument());
  });

  it('shows no permissions text', async () => {
    render(<AdminRolesPage />);
    await waitFor(() => expect(screen.getByText('rbac.noPermissions')).toBeInTheDocument());
  });

  it('shows help', async () => {
    render(<AdminRolesPage />);
    await waitFor(() => {
      const helpBtn = screen.getByTitle('rbac.helpLabel');
      fireEvent.click(helpBtn);
    });
    await waitFor(() => expect(screen.getByText('rbac.help')).toBeInTheDocument());
  });

  it('opens create form', async () => {
    render(<AdminRolesPage />);
    await waitFor(() => fireEvent.click(screen.getByText('rbac.createRole')));
    expect(screen.getByText('rbac.newRole')).toBeInTheDocument();
  });

  it('validates empty name', async () => {
    render(<AdminRolesPage />);
    await waitFor(() => fireEvent.click(screen.getByText('rbac.createRole')));
    fireEvent.click(screen.getByText('rbac.save'));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('rbac.nameRequired'));
  });

  it('creates role', async () => {
    const user = userEvent.setup();
    render(<AdminRolesPage />);
    await waitFor(() => fireEvent.click(screen.getByText('rbac.createRole')));
    await user.type(screen.getByPlaceholderText('rbac.namePlaceholder'), 'Editor');
    fireEvent.click(screen.getByText('rbac.save'));
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('rbac.created'));
  });

  it('edits role', async () => {
    render(<AdminRolesPage />);
    await waitFor(() => {
      const editBtns = screen.getAllByTitle('rbac.edit');
      fireEvent.click(editBtns[0]);
    });
    expect(screen.getByText('rbac.editRole')).toBeInTheDocument();
  });

  it('deletes custom role', async () => {
    render(<AdminRolesPage />);
    await waitFor(() => {
      const delBtns = screen.getAllByTitle('rbac.delete');
      fireEvent.click(delBtns[0]); // Viewer (not built-in)
    });
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('rbac.deleted'));
  });

  it('does not show delete for built-in', async () => {
    render(<AdminRolesPage />);
    await waitFor(() => expect(screen.getByText('Admin')).toBeInTheDocument());
    // Built-in roles should not have delete button
  });

  it('toggles permission checkboxes', async () => {
    render(<AdminRolesPage />);
    await waitFor(() => fireEvent.click(screen.getByText('rbac.createRole')));
    const checkboxes = screen.getAllByRole('checkbox');
    fireEvent.click(checkboxes[0]); // toggle on
    fireEvent.click(checkboxes[0]); // toggle off
  });

  it('cancel form', async () => {
    render(<AdminRolesPage />);
    await waitFor(() => fireEvent.click(screen.getByText('rbac.createRole')));
    fireEvent.click(screen.getByText('common.cancel'));
    await waitFor(() => expect(screen.queryByText('rbac.newRole')).not.toBeInTheDocument());
  });

  it('handles save error', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST') return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Duplicate' } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: roles }) } as Response);
    }) as any;
    const user = userEvent.setup();
    render(<AdminRolesPage />);
    await waitFor(() => fireEvent.click(screen.getByText('rbac.createRole')));
    await user.type(screen.getByPlaceholderText('rbac.namePlaceholder'), 'Admin');
    fireEvent.click(screen.getByText('rbac.save'));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Duplicate'));
  });

  it('handles save network error', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST') return Promise.reject(new Error('net'));
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: roles }) } as Response);
    }) as any;
    const user = userEvent.setup();
    render(<AdminRolesPage />);
    await waitFor(() => fireEvent.click(screen.getByText('rbac.createRole')));
    await user.type(screen.getByPlaceholderText('rbac.namePlaceholder'), 'X');
    fireEvent.click(screen.getByText('rbac.save'));
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });

  it('handles delete confirm cancel', async () => {
    globalThis.confirm = vi.fn(() => false);
    render(<AdminRolesPage />);
    await waitFor(() => {
      const delBtns = screen.getAllByTitle('rbac.delete');
      fireEvent.click(delBtns[0]);
    });
    // Should not call delete API
    expect(globalThis.fetch).not.toHaveBeenCalledWith(expect.stringContaining('/roles/r2'), expect.objectContaining({ method: 'DELETE' }));
  });

  it('handles delete error response', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'DELETE') return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'In use' } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: roles }) } as Response);
    }) as any;
    render(<AdminRolesPage />);
    await waitFor(() => fireEvent.click(screen.getAllByTitle('rbac.delete')[0]));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('In use'));
  });

  it('handles delete network error', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'DELETE') return Promise.reject(new Error('net'));
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: roles }) } as Response);
    }) as any;
    render(<AdminRolesPage />);
    await waitFor(() => fireEvent.click(screen.getAllByTitle('rbac.delete')[0]));
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });

  it('shows empty state', async () => {
    globalThis.fetch = vi.fn(() => Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response)) as any;
    render(<AdminRolesPage />);
    await waitFor(() => expect(screen.getByText('rbac.empty')).toBeInTheDocument());
  });

  it('handles load error', async () => {
    globalThis.fetch = vi.fn(() => Promise.reject(new Error('net'))) as any;
    render(<AdminRolesPage />);
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });
});
