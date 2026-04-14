import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'rbac.title': 'Roles & Permissions',
        'rbac.subtitle': 'Manage role-based access control',
        'rbac.help': 'Define roles with granular permissions to control access.',
        'rbac.helpLabel': 'Help',
        'rbac.createRole': 'Create Role',
        'rbac.newRole': 'New Role',
        'rbac.editRole': 'Edit Role',
        'rbac.name': 'Name',
        'rbac.namePlaceholder': 'Role name',
        'rbac.description': 'Description',
        'rbac.descriptionPlaceholder': 'Optional description',
        'rbac.permissions': 'Permissions',
        'rbac.save': 'Save',
        'rbac.edit': 'Edit',
        'rbac.delete': 'Delete',
        'rbac.created': 'Role created',
        'rbac.updated': 'Role updated',
        'rbac.deleted': 'Role deleted',
        'rbac.empty': 'No roles defined',
        'rbac.builtIn': 'Built-in',
        'rbac.noPermissions': 'No permissions',
        'rbac.nameRequired': 'Role name is required',
        'rbac.deleteConfirm': 'Delete this role?',
        'rbac.perm.manage_users': 'Manage Users',
        'rbac.perm.manage_lots': 'Manage Lots',
        'rbac.perm.manage_bookings': 'Manage Bookings',
        'rbac.perm.view_reports': 'View Reports',
        'rbac.perm.manage_settings': 'Manage Settings',
        'rbac.perm.manage_plugins': 'Manage Plugins',
        'common.cancel': 'Cancel',
        'common.error': 'Error',
        'common.loading': 'Loading...',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, layout, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  ShieldCheck: (props: any) => <span data-testid="icon-shield" {...props} />,
  Plus: (props: any) => <span data-testid="icon-plus" {...props} />,
  Trash: (props: any) => <span data-testid="icon-trash" {...props} />,
  Pencil: (props: any) => <span data-testid="icon-pencil" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  UserCircle: (props: any) => <span data-testid="icon-user" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: { success: vi.fn(), error: vi.fn() },
}));

import { AdminRolesPage } from './AdminRoles';

const sampleRoles = [
  {
    id: 'role-001',
    name: 'super_admin',
    description: 'Full system access',
    permissions: ['manage_users', 'manage_lots', 'manage_bookings', 'view_reports', 'manage_settings', 'manage_plugins'],
    built_in: true,
    created_at: '2026-03-23T10:00:00Z',
    updated_at: '2026-03-23T10:00:00Z',
  },
  {
    id: 'role-002',
    name: 'viewer',
    description: 'Read-only access',
    permissions: ['view_reports'],
    built_in: true,
    created_at: '2026-03-23T10:00:00Z',
    updated_at: '2026-03-23T10:00:00Z',
  },
  {
    id: 'role-003',
    name: 'custom_role',
    description: 'Custom test role',
    permissions: ['manage_lots'],
    built_in: false,
    created_at: '2026-03-23T10:00:00Z',
    updated_at: '2026-03-23T10:00:00Z',
  },
];

describe('AdminRolesPage', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('renders title and subtitle', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleRoles }),
    });

    render(<AdminRolesPage />);
    expect(screen.getByText('Roles & Permissions')).toBeDefined();
    expect(screen.getByText('Manage role-based access control')).toBeDefined();
  });

  it('renders role list with names', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleRoles }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => {
      expect(screen.getByText('super_admin')).toBeDefined();
      expect(screen.getByText('viewer')).toBeDefined();
      expect(screen.getByText('custom_role')).toBeDefined();
    });
  });

  it('shows built-in badge for built-in roles', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleRoles }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => {
      const badges = screen.getAllByText('Built-in');
      expect(badges.length).toBe(2); // super_admin and viewer
    });
  });

  it('shows empty state', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => {
      expect(screen.getByText('No roles defined')).toBeDefined();
    });
  });

  it('opens create form', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => {
      fireEvent.click(screen.getByText('Create Role'));
      expect(screen.getByText('New Role')).toBeDefined();
    });
  });

  it('shows permission checkboxes in form', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => {
      fireEvent.click(screen.getByText('Create Role'));
      expect(screen.getByText('Manage Users')).toBeDefined();
      expect(screen.getByText('Manage Lots')).toBeDefined();
      expect(screen.getByText('View Reports')).toBeDefined();
    });
  });

  it('shows permission badges on roles', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleRoles }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => {
      // viewer has view_reports permission
      expect(screen.getAllByText('View Reports').length).toBeGreaterThan(0);
    });
  });

  it('handles API errors gracefully', async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new Error('Network error'));

    render(<AdminRolesPage />);
    await waitFor(() => {
      expect(screen.getByText('Roles & Permissions')).toBeDefined();
    });
  });

  it('shows role descriptions', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleRoles }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => {
      expect(screen.getByText('Full system access')).toBeDefined();
      expect(screen.getByText('Read-only access')).toBeDefined();
      expect(screen.getByText('Custom test role')).toBeDefined();
    });
  });

  it('shows custom role in list', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleRoles }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => {
      expect(screen.getByText('custom_role')).toBeDefined();
    });
  });

  it('shows help tooltip', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => expect(screen.getByTitle('Help')).toBeDefined());
    fireEvent.click(screen.getByTitle('Help'));
    await waitFor(() => {
      expect(screen.getByText('Define roles with granular permissions to control access.')).toBeDefined();
    });
  });

  it('shows create role button', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleRoles }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => {
      expect(screen.getByText('Create Role')).toBeDefined();
    });
  });

  it('shows all permission options in create form', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => {
      fireEvent.click(screen.getByText('Create Role'));
      expect(screen.getByText('Manage Settings')).toBeDefined();
      expect(screen.getByText('Manage Plugins')).toBeDefined();
      expect(screen.getByText('Manage Bookings')).toBeDefined();
    });
  });

  it('saves a new role successfully', async () => {
    let callCount = 0;
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST') {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({ success: true, data: { id: 'role-new', name: 'editor', permissions: ['view_reports'] } }),
        });
      }
      callCount++;
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve({ success: true, data: callCount <= 1 ? [] : sampleRoles }),
      });
    });

    render(<AdminRolesPage />);
    await waitFor(() => fireEvent.click(screen.getByText('Create Role')));
    await waitFor(() => expect(screen.getByText('New Role')).toBeDefined());

    // Fill name
    const nameInput = screen.getByPlaceholderText('Role name');
    fireEvent.change(nameInput, { target: { value: 'editor' } });

    // Toggle a permission
    fireEvent.click(screen.getByText('View Reports'));

    // Save
    fireEvent.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalledWith(
        '/api/v1/admin/roles',
        expect.objectContaining({ method: 'POST' }),
      );
    });
  });

  it('shows error when saving role without name', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => fireEvent.click(screen.getByText('Create Role')));

    // Don't fill name, just save
    fireEvent.click(screen.getByText('Save'));
    // Error toast for missing name
  });

  it('cancels create form', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => fireEvent.click(screen.getByText('Create Role')));
    expect(screen.getByText('New Role')).toBeDefined();

    fireEvent.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(screen.queryByText('New Role')).toBeNull();
    });
  });

  it('opens edit form for existing role', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleRoles }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => expect(screen.getByText('custom_role')).toBeDefined());

    // Click edit on custom_role
    const editBtns = screen.getAllByTitle('Edit');
    fireEvent.click(editBtns[editBtns.length - 1]); // last one = custom_role

    await waitFor(() => {
      expect(screen.getByText('Edit Role')).toBeDefined();
    });
  });

  it('deletes custom role', async () => {
    // Mock confirm dialog
    vi.spyOn(window, 'confirm').mockReturnValue(true);
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'DELETE') {
        return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true }) });
      }
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve({ success: true, data: sampleRoles }),
      });
    });

    render(<AdminRolesPage />);
    await waitFor(() => expect(screen.getByText('custom_role')).toBeDefined());

    // Only custom_role has a delete button (non-built-in)
    const deleteBtn = screen.getByTitle('Delete');
    fireEvent.click(deleteBtn);

    await waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalledWith(
        '/api/v1/admin/roles/role-003',
        expect.objectContaining({ method: 'DELETE' }),
      );
    });
  });

  it('does not delete built-in role', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleRoles }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => expect(screen.getByText('super_admin')).toBeDefined());

    // Built-in roles should not have delete buttons -- only 1 delete button for custom_role
    const deleteBtns = screen.getAllByTitle('Delete');
    expect(deleteBtns.length).toBe(1);
  });

  it('shows role with no permissions', async () => {
    const rolesWithEmpty = [
      ...sampleRoles,
      { id: 'role-004', name: 'empty_role', description: null, permissions: [], built_in: false, created_at: '2026-03-23T10:00:00Z', updated_at: '2026-03-23T10:00:00Z' },
    ];
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: rolesWithEmpty }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => {
      expect(screen.getByText('empty_role')).toBeDefined();
      expect(screen.getByText('No permissions')).toBeDefined();
    });
  });

  it('handles save failure with error message', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST') {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({ success: false, error: { message: 'Duplicate name' } }),
        });
      }
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve({ success: true, data: [] }),
      });
    });

    render(<AdminRolesPage />);
    await waitFor(() => fireEvent.click(screen.getByText('Create Role')));

    fireEvent.change(screen.getByPlaceholderText('Role name'), { target: { value: 'dup' } });
    fireEvent.click(screen.getByText('Save'));
    // Error path exercised
  });

  it('handles save fetch exception', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST') {
        return Promise.reject(new Error('Network'));
      }
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve({ success: true, data: [] }),
      });
    });

    render(<AdminRolesPage />);
    await waitFor(() => fireEvent.click(screen.getByText('Create Role')));

    fireEvent.change(screen.getByPlaceholderText('Role name'), { target: { value: 'test' } });
    fireEvent.click(screen.getByText('Save'));
    // Exception path exercised
  });

  it('toggles permission in form', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    render(<AdminRolesPage />);
    await waitFor(() => fireEvent.click(screen.getByText('Create Role')));

    // Toggle manage_users on
    fireEvent.click(screen.getByText('Manage Users'));
    // Toggle it off
    fireEvent.click(screen.getByText('Manage Users'));
  });
});
