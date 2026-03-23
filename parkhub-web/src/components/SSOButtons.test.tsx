import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => {
      const map: Record<string, string> = {
        'sso.continueWith': `Continue with ${opts?.provider || 'SSO'}`,
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('@phosphor-icons/react', () => ({
  ShieldCheck: (props: any) => <span data-testid="icon-shield" {...props} />,
}));

import { SSOButtons } from './SSOButtons';

describe('SSOButtons', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('renders nothing when no providers are configured', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ data: { providers: [] } }),
    });

    const { container } = render(<SSOButtons />);
    await waitFor(() => {
      expect(container.innerHTML).toBe('');
    });
  });

  it('renders SSO buttons for configured providers', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () =>
        Promise.resolve({
          data: {
            providers: [
              { slug: 'okta', display_name: 'Okta', enabled: true },
              { slug: 'azure-ad', display_name: 'Azure AD', enabled: true },
            ],
          },
        }),
    });

    render(<SSOButtons />);
    await waitFor(() => {
      expect(screen.getByText('Continue with Okta')).toBeDefined();
      expect(screen.getByText('Continue with Azure AD')).toBeDefined();
    });
  });

  it('handles API failure gracefully', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
    });

    const { container } = render(<SSOButtons />);
    await waitFor(() => {
      expect(container.innerHTML).toBe('');
    });
  });

  it('initiates SSO login on button click', async () => {
    const mockFetch = vi.fn();
    mockFetch
      .mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({
            data: {
              providers: [{ slug: 'okta', display_name: 'Okta', enabled: true }],
            },
          }),
      })
      .mockResolvedValueOnce({
        ok: true,
        json: () =>
          Promise.resolve({ redirect_url: 'https://okta.example.com/sso?SAMLRequest=abc' }),
      });

    globalThis.fetch = mockFetch;

    render(<SSOButtons />);
    await waitFor(() => {
      expect(screen.getByText('Continue with Okta')).toBeDefined();
    });

    fireEvent.click(screen.getByText('Continue with Okta'));
    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledTimes(2);
    });
  });

  it('renders shield icon for each provider', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () =>
        Promise.resolve({
          data: {
            providers: [{ slug: 'saml', display_name: 'Corporate SSO', enabled: true }],
          },
        }),
    });

    render(<SSOButtons />);
    await waitFor(() => {
      expect(screen.getByTestId('icon-shield')).toBeDefined();
    });
  });

  it('handles network error on provider fetch', async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new Error('Network error'));

    const { container } = render(<SSOButtons />);
    await waitFor(() => {
      expect(container.innerHTML).toBe('');
    });
  });
});
