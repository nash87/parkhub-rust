import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

const mockListTenants = vi.fn();
const mockCreateTenant = vi.fn();
const mockUpdateTenant = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    listTenants: (...args: any[]) => mockListTenants(...args),
    createTenant: (...args: any[]) => mockCreateTenant(...args),
    updateTenant: (...args: any[]) => mockUpdateTenant(...args),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'tenants.title': 'Tenants',
        'tenants.create': 'Create Tenant',
        'tenants.empty': 'No tenants configured.',
        'tenants.name': 'Name',
        'tenants.domain': 'Domain',
        'tenants.brandColor': 'Brand Color',
        'tenants.users': 'users',
        'tenants.lots': 'lots',
        'tenants.editTitle': 'Edit Tenant',
        'tenants.created': 'Tenant created',
        'common.save': 'Save',
        'common.cancel': 'Cancel',
      };
      return map[key] || fallback || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Buildings: (props: any) => <span data-testid="icon-buildings" {...props} />,
  Plus: (props: any) => <span data-testid="icon-plus" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  PencilSimple: (props: any) => <span data-testid="icon-pencil" {...props} />,
}));

import { AdminTenantsPage } from './AdminTenants';

const sampleTenants = [
  { id: 't-1', name: 'Acme Corp', domain: 'acme.com', branding: { primary_color: '#FF5733' }, created_at: '2026-01-01', updated_at: '2026-01-01', user_count: 5, lot_count: 2 },
  { id: 't-2', name: 'Beta Inc', domain: null, branding: null, created_at: '2026-02-01', updated_at: '2026-02-01', user_count: 3, lot_count: 1 },
];

describe('AdminTenantsPage', () => {
  beforeEach(() => {
    mockListTenants.mockClear();
    mockCreateTenant.mockClear();
    mockUpdateTenant.mockClear();
    mockListTenants.mockResolvedValue({ success: true, data: sampleTenants });
    mockCreateTenant.mockResolvedValue({ success: true, data: { id: 't-3', name: 'New', domain: null, branding: null, created_at: '', updated_at: '', user_count: 0, lot_count: 0 } });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the title after loading', async () => {
    render(<AdminTenantsPage />);
    await waitFor(() => {
      expect(screen.getByText('Tenants')).toBeInTheDocument();
    });
  });

  it('renders tenant list', async () => {
    render(<AdminTenantsPage />);
    await waitFor(() => {
      expect(screen.getByText('Acme Corp')).toBeInTheDocument();
      expect(screen.getByText('Beta Inc')).toBeInTheDocument();
    });
  });

  it('shows domain for tenants that have one', async () => {
    render(<AdminTenantsPage />);
    await waitFor(() => {
      expect(screen.getByText('acme.com')).toBeInTheDocument();
    });
  });

  it('shows empty state when no tenants', async () => {
    mockListTenants.mockResolvedValue({ success: true, data: [] });
    render(<AdminTenantsPage />);
    await waitFor(() => {
      expect(screen.getByText('No tenants configured.')).toBeInTheDocument();
    });
  });

  it('opens create modal on button click', async () => {
    const user = userEvent.setup();
    render(<AdminTenantsPage />);

    await waitFor(() => {
      expect(screen.getByText('Tenants')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Create Tenant'));

    await waitFor(() => {
      expect(screen.getByTestId('tenant-name-input')).toBeInTheDocument();
      expect(screen.getByTestId('tenant-domain-input')).toBeInTheDocument();
    });
  });
});
