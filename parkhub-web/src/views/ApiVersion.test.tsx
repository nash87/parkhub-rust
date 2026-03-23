import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'apiVersion.title': 'API Version',
        'apiVersion.version': 'Version',
        'apiVersion.prefix': 'Prefix',
        'apiVersion.status': 'Status',
        'apiVersion.deprecations': 'Deprecations',
        'apiVersion.tooltip': 'Current API version',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('@phosphor-icons/react', () => ({
  Code: (props: any) => <span data-testid="icon-code" {...props} />,
  Info: (props: any) => <span data-testid="icon-info" {...props} />,
}));

import { ApiVersionBadge, ApiVersionAdmin } from './ApiVersion';

const sampleVersionInfo = {
  version: '4.1.0',
  api_prefix: '/api/v1',
  status: 'stable',
  deprecations: [
    {
      endpoint: '/api/v1/lots/{id}/slots',
      method: 'GET',
      severity: 'info',
      message: 'Use /api/v1/lots/{id}/display instead',
      sunset_date: '2027-01-01',
      replacement: '/api/v1/lots/{id}/display',
    },
  ],
  supported_versions: ['v1'],
};

describe('ApiVersionBadge', () => {
  beforeEach(() => {
    global.fetch = vi.fn(() =>
      Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleVersionInfo }) } as Response)
    ) as any;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders version badge after loading', async () => {
    render(<ApiVersionBadge />);
    await waitFor(() => {
      expect(screen.getByTestId('api-version-badge')).toBeInTheDocument();
      expect(screen.getByText(/API v4\.1\.0/)).toBeInTheDocument();
    });
  });
});

describe('ApiVersionAdmin', () => {
  beforeEach(() => {
    global.fetch = vi.fn(() =>
      Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleVersionInfo }) } as Response)
    ) as any;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders version info panel', async () => {
    render(<ApiVersionAdmin />);
    await waitFor(() => {
      expect(screen.getByTestId('api-version-admin')).toBeInTheDocument();
    });
  });

  it('displays version value', async () => {
    render(<ApiVersionAdmin />);
    await waitFor(() => {
      expect(screen.getByTestId('version-value')).toHaveTextContent('4.1.0');
    });
  });

  it('shows deprecation notices', async () => {
    render(<ApiVersionAdmin />);
    await waitFor(() => {
      expect(screen.getByTestId('deprecation-list')).toBeInTheDocument();
      expect(screen.getByText(/lots.*slots/)).toBeInTheDocument();
    });
  });
});
