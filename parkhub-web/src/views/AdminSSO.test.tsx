import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => {
      const map: Record<string, string> = {
        'sso.title': 'SSO Configuration',
        'sso.subtitle': 'Configure SAML/SSO enterprise authentication providers',
        'sso.help': 'Add SAML 2.0 identity providers for enterprise single sign-on.',
        'sso.helpLabel': 'Help',
        'sso.addProvider': 'Add Provider',
        'sso.newProvider': 'New Provider',
        'sso.editProvider': 'Edit Provider',
        'sso.slug': 'Slug',
        'sso.displayName': 'Display Name',
        'sso.entityId': 'Entity ID',
        'sso.ssoUrl': 'SSO URL',
        'sso.metadataUrl': 'Metadata URL',
        'sso.certificate': 'Certificate',
        'sso.save': 'Save',
        'sso.edit': 'Edit',
        'sso.delete': 'Delete',
        'sso.created': 'Provider created',
        'sso.updated': 'Provider updated',
        'sso.deleted': 'Provider deleted',
        'sso.empty': 'No SSO providers configured',
        'sso.requiredFields': 'Please fill all required fields',
        'sso.continueWith': `Continue with ${opts?.provider || 'SSO'}`,
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
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
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
  ToggleLeft: (props: any) => <span data-testid="icon-toggle-off" {...props} />,
  ToggleRight: (props: any) => <span data-testid="icon-toggle-on" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: { success: vi.fn(), error: vi.fn() },
}));

import { AdminSSOPage } from './AdminSSO';

const sampleProviders = {
  providers: [
    {
      slug: 'okta',
      display_name: 'Okta',
      entity_id: 'https://okta.example.com',
      metadata_url: 'https://okta.example.com/metadata',
      sso_url: 'https://okta.example.com/sso',
      certificate: 'MIIC...',
      enabled: true,
      created_at: '2026-03-20T10:00:00Z',
      updated_at: '2026-03-23T08:00:00Z',
    },
  ],
};

describe('AdminSSOPage', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('renders title and subtitle', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleProviders }),
    });

    render(<AdminSSOPage />);
    expect(screen.getByText('SSO Configuration')).toBeDefined();
    expect(screen.getByText('Configure SAML/SSO enterprise authentication providers')).toBeDefined();
  });

  it('renders providers list from API', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleProviders }),
    });

    render(<AdminSSOPage />);
    await waitFor(() => {
      expect(screen.getByText('Okta')).toBeDefined();
    });
  });

  it('shows empty state when no providers', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: { providers: [] } }),
    });

    render(<AdminSSOPage />);
    await waitFor(() => {
      expect(screen.getByText('No SSO providers configured')).toBeDefined();
    });
  });

  it('opens form when Add Provider is clicked', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: { providers: [] } }),
    });

    render(<AdminSSOPage />);
    await waitFor(() => {
      fireEvent.click(screen.getByText('Add Provider'));
      expect(screen.getByText('New Provider')).toBeDefined();
    });
  });

  it('shows cancel button in form', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: { providers: [] } }),
    });

    render(<AdminSSOPage />);
    await waitFor(() => {
      fireEvent.click(screen.getByText('Add Provider'));
      expect(screen.getByText('Cancel')).toBeDefined();
    });
  });

  it('sends PUT request on save', async () => {
    const mockFetch = vi.fn();
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve({ success: true, data: { providers: [] } }),
    });
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve({ success: true, data: { slug: 'test', display_name: 'Test', enabled: true } }),
    });
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () => Promise.resolve({ success: true, data: { providers: [] } }),
    });

    globalThis.fetch = mockFetch;

    render(<AdminSSOPage />);
    await waitFor(() => {
      fireEvent.click(screen.getByText('Add Provider'));
    });

    // Fill in form fields
    const inputs = screen.getAllByRole('textbox');
    fireEvent.change(inputs[0], { target: { value: 'test-sso' } }); // slug
    fireEvent.change(inputs[1], { target: { value: 'Test SSO' } }); // display name
    fireEvent.change(inputs[2], { target: { value: 'https://idp.test' } }); // entity id
    fireEvent.change(inputs[3], { target: { value: 'https://idp.test/sso' } }); // sso url
    fireEvent.change(inputs[4], { target: { value: 'https://idp.test/metadata' } }); // metadata url
    fireEvent.change(inputs[5], { target: { value: 'MIIC...' } }); // certificate

    fireEvent.click(screen.getByText('Save'));
    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/admin/sso/'),
        expect.objectContaining({ method: 'PUT' }),
      );
    });
  });

  it('handles API errors gracefully', async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new Error('Network error'));

    render(<AdminSSOPage />);
    // Should not crash
    await waitFor(() => {
      expect(screen.getByText('SSO Configuration')).toBeDefined();
    });
  });
});
