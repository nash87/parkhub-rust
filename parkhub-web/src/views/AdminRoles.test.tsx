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
});
