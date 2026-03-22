import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'auth.continueWithGoogle': 'Continue with Google',
        'auth.continueWithGitHub': 'Continue with GitHub',
        'auth.orContinueWith': 'or',
      };
      return map[key] || key;
    },
  }),
}));

import { OAuthButtons } from './OAuthButtons';

describe('OAuthButtons', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders both buttons when both providers are available', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ data: { google: true, github: true } }),
    }) as any;

    render(<OAuthButtons />);

    await waitFor(() => {
      expect(screen.getByTestId('oauth-google')).toBeDefined();
      expect(screen.getByTestId('oauth-github')).toBeDefined();
    });

    expect(screen.getByText('Continue with Google')).toBeDefined();
    expect(screen.getByText('Continue with GitHub')).toBeDefined();
  });

  it('renders only Google button when only Google is configured', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ data: { google: true, github: false } }),
    }) as any;

    render(<OAuthButtons />);

    await waitFor(() => {
      expect(screen.getByTestId('oauth-google')).toBeDefined();
    });

    expect(screen.queryByTestId('oauth-github')).toBeNull();
  });

  it('renders only GitHub button when only GitHub is configured', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ data: { google: false, github: true } }),
    }) as any;

    render(<OAuthButtons />);

    await waitFor(() => {
      expect(screen.getByTestId('oauth-github')).toBeDefined();
    });

    expect(screen.queryByTestId('oauth-google')).toBeNull();
  });

  it('renders nothing when no providers are configured', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ data: { google: false, github: false } }),
    }) as any;

    const { container } = render(<OAuthButtons />);

    // Wait for fetch to resolve
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalled();
    });

    // Should render nothing
    expect(container.innerHTML).toBe('');
  });

  it('renders nothing when fetch fails', async () => {
    global.fetch = vi.fn().mockRejectedValue(new Error('Network error')) as any;

    const { container } = render(<OAuthButtons />);

    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalled();
    });

    expect(container.innerHTML).toBe('');
  });

  it('renders correct OAuth redirect URLs', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ data: { google: true, github: true } }),
    }) as any;

    render(<OAuthButtons />);

    await waitFor(() => {
      expect(screen.getByTestId('oauth-google')).toBeDefined();
    });

    const googleLink = screen.getByTestId('oauth-google');
    const githubLink = screen.getByTestId('oauth-github');

    expect(googleLink.getAttribute('href')).toBe('/api/v1/auth/oauth/google');
    expect(githubLink.getAttribute('href')).toBe('/api/v1/auth/oauth/github');
  });

  it('shows divider text between OAuth buttons and form', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ data: { google: true, github: false } }),
    }) as any;

    render(<OAuthButtons />);

    await waitFor(() => {
      expect(screen.getByText('or')).toBeDefined();
    });
  });
});
