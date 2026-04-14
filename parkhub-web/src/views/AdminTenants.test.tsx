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

  it('creates a tenant successfully', async () => {
    const user = userEvent.setup();
    render(<AdminTenantsPage />);
    await waitFor(() => expect(screen.getByText('Tenants')).toBeInTheDocument());

    await user.click(screen.getByText('Create Tenant'));
    await user.type(screen.getByTestId('tenant-name-input'), 'New Corp');
    await user.type(screen.getByTestId('tenant-domain-input'), 'newcorp.com');
    await user.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(mockCreateTenant).toHaveBeenCalledWith(expect.objectContaining({
        name: 'New Corp',
        domain: 'newcorp.com',
      }));
    });
  });

  it('does not save when name is empty', async () => {
    const user = userEvent.setup();
    render(<AdminTenantsPage />);
    await waitFor(() => expect(screen.getByText('Tenants')).toBeInTheDocument());

    await user.click(screen.getByText('Create Tenant'));
    // Name is empty, Save button should be disabled
    const saveBtn = screen.getByText('Save');
    expect(saveBtn).toBeDisabled();
  });

  it('opens edit modal for existing tenant', async () => {
    const user = userEvent.setup();
    render(<AdminTenantsPage />);
    await waitFor(() => expect(screen.getByText('Acme Corp')).toBeInTheDocument());

    // Click edit on first tenant
    const editBtns = screen.getAllByLabelText('Edit');
    await user.click(editBtns[0]);

    await waitFor(() => {
      expect(screen.getByText('Edit Tenant')).toBeInTheDocument();
      expect(screen.getByTestId('tenant-name-input')).toHaveValue('Acme Corp');
      expect(screen.getByTestId('tenant-domain-input')).toHaveValue('acme.com');
    });
  });

  it('updates a tenant successfully', async () => {
    const user = userEvent.setup();
    mockUpdateTenant.mockResolvedValue({ success: true, data: { ...sampleTenants[0], name: 'Updated' } });
    render(<AdminTenantsPage />);
    await waitFor(() => expect(screen.getByText('Acme Corp')).toBeInTheDocument());

    const editBtns = screen.getAllByLabelText('Edit');
    await user.click(editBtns[0]);

    const nameInput = screen.getByTestId('tenant-name-input');
    await user.clear(nameInput);
    await user.type(nameInput, 'Updated');
    await user.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(mockUpdateTenant).toHaveBeenCalledWith('t-1', expect.objectContaining({ name: 'Updated' }));
    });
  });

  it('closes modal on cancel', async () => {
    const user = userEvent.setup();
    render(<AdminTenantsPage />);
    await waitFor(() => expect(screen.getByText('Tenants')).toBeInTheDocument());

    await user.click(screen.getByText('Create Tenant'));
    await waitFor(() => expect(screen.getByTestId('tenant-name-input')).toBeInTheDocument());

    await user.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(screen.queryByTestId('tenant-name-input')).not.toBeInTheDocument();
    });
  });

  it('closes modal on X button', async () => {
    const user = userEvent.setup();
    render(<AdminTenantsPage />);
    await waitFor(() => expect(screen.getByText('Tenants')).toBeInTheDocument());

    await user.click(screen.getByText('Create Tenant'));
    await waitFor(() => expect(screen.getByTestId('tenant-name-input')).toBeInTheDocument());

    // Click X icon button
    const closeBtn = screen.getByTestId('icon-x').closest('button')!;
    await user.click(closeBtn);
    await waitFor(() => {
      expect(screen.queryByTestId('tenant-name-input')).not.toBeInTheDocument();
    });
  });

  it('handles API error on load', async () => {
    mockListTenants.mockRejectedValue(new Error('Network'));
    render(<AdminTenantsPage />);
    await waitFor(() => {
      expect(screen.getByText('Tenants')).toBeInTheDocument();
    });
  });

  it('handles API error on create', async () => {
    const user = userEvent.setup();
    mockCreateTenant.mockRejectedValue(new Error('Server error'));
    render(<AdminTenantsPage />);
    await waitFor(() => expect(screen.getByText('Tenants')).toBeInTheDocument());

    await user.click(screen.getByText('Create Tenant'));
    await user.type(screen.getByTestId('tenant-name-input'), 'Failing');
    await user.click(screen.getByText('Save'));

    // Should not crash
    await waitFor(() => {
      expect(screen.getByText('Tenants')).toBeInTheDocument();
    });
  });

  it('shows user and lot counts for tenants', async () => {
    render(<AdminTenantsPage />);
    await waitFor(() => {
      expect(screen.getByText('5 users')).toBeInTheDocument();
      expect(screen.getByText('2 lots')).toBeInTheDocument();
    });
  });

  it('creates tenant with brand color', async () => {
    const user = userEvent.setup();
    render(<AdminTenantsPage />);
    await waitFor(() => expect(screen.getByText('Tenants')).toBeInTheDocument());

    await user.click(screen.getByText('Create Tenant'));
    await user.type(screen.getByTestId('tenant-name-input'), 'Colored');

    // Brand color input is a color input - we can just verify it renders
    expect(screen.getByDisplayValue('#6366f1')).toBeInTheDocument();

    await user.click(screen.getByText('Save'));
    await waitFor(() => {
      expect(mockCreateTenant).toHaveBeenCalledWith(expect.objectContaining({
        name: 'Colored',
        branding: undefined, // no color typed so default
      }));
    });
  });

  it('handles tenant without branding gracefully', async () => {
    render(<AdminTenantsPage />);
    await waitFor(() => {
      // Beta Inc has no branding
      expect(screen.getByText('Beta Inc')).toBeInTheDocument();
    });
  });

  it('handles tenant without domain', async () => {
    render(<AdminTenantsPage />);
    await waitFor(() => {
      expect(screen.getByText('Beta Inc')).toBeInTheDocument();
      // Beta Inc has no domain, so no domain text
      expect(screen.queryByText('null')).not.toBeInTheDocument();
    });
  });
});
