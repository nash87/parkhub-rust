import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallbackOrOpts?: string | Record<string, any>) => {
      const map: Record<string, string> = {
        'updates.title': 'System Updates',
        'updates.subtitle': 'Manage application updates and versioning',
        'updates.currentVersion': 'Current Version',
        'updates.version': 'Version',
        'updates.buildInfo': 'Build',
        'updates.uptime': 'Uptime',
        'updates.checkTitle': 'Check for Updates',
        'updates.checkButton': 'Check for Updates',
        'updates.upToDate': "You're up to date!",
        'updates.newVersion': 'New version available',
        'updates.releaseNotes': 'Release Notes',
        'updates.applyButton': 'Update Now',
        'updates.autoUpdateLabel': 'Enable automatic updates',
        'updates.autoUpdateDesc': 'Automatically install minor and patch updates.',
        'updates.channelTitle': 'Update Channel',
        'updates.channelStable': 'Stable',
        'updates.channelBeta': 'Beta',
        'updates.historyTitle': 'Update History',
        'updates.noHistory': 'No update history yet',
        'updates.autoEnabled': 'Auto-updates enabled',
        'updates.autoDisabled': 'Auto-updates disabled',
        'common.error': 'Error',
      };
      return map[key] || (typeof fallbackOrOpts === 'string' ? fallbackOrOpts : key);
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  ArrowsClockwise: (props: any) => <span data-testid="icon-arrows-clockwise" {...props} />,
  CheckCircle: (props: any) => <span data-testid="icon-check" {...props} />,
  Warning: (props: any) => <span data-testid="icon-warning" {...props} />,
  CloudArrowDown: (props: any) => <span data-testid="icon-cloud" {...props} />,
  Info: (props: any) => <span data-testid="icon-info" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  Spinner: (props: any) => <span data-testid="icon-spinner" {...props} />,
  ArrowRight: (props: any) => <span data-testid="icon-arrow-right" {...props} />,
}));

vi.mock('../api/client', () => ({
  getInMemoryToken: () => 'test-token',
}));

import { AdminUpdatesPage } from './AdminUpdates';

const mockVersion = {
  version: '4.8.0',
  build_hash: 'abc12345def',
  build_date: '2026-04-10T12:00:00Z',
  uptime_seconds: 86400,
};

const mockHistory = [
  {
    id: 'u1',
    from_version: '4.7.0',
    to_version: '4.8.0',
    status: 'success',
    applied_at: '2026-04-10T12:00:00Z',
  },
  {
    id: 'u2',
    from_version: '4.6.0',
    to_version: '4.7.0',
    status: 'failed',
    applied_at: '2026-04-01T10:00:00Z',
    error: 'Timeout',
  },
];

const mockSettings = {
  auto_update: false,
  update_channel: 'stable',
};

function mockFetch(overrides: Record<string, any> = {}) {
  return vi.fn((url: string, opts?: any) => {
    if (url.includes('/system/version')) {
      return Promise.resolve({
        json: () => Promise.resolve(overrides.version ?? { success: true, data: mockVersion }),
      } as Response);
    }
    if (url.includes('/updates/history')) {
      return Promise.resolve({
        json: () => Promise.resolve(overrides.history ?? { success: true, data: mockHistory }),
      } as Response);
    }
    if (url.includes('/updates/check')) {
      return Promise.resolve({
        json: () => Promise.resolve(overrides.check ?? { success: true, data: { available: false, current_version: '4.8.0', latest_version: '4.8.0' } }),
      } as Response);
    }
    if (url.includes('/updates/apply')) {
      return Promise.resolve({
        json: () => Promise.resolve(overrides.apply ?? { success: true, data: { version: '4.9.0' } }),
      } as Response);
    }
    if (url.includes('/admin/settings') && opts?.method === 'POST') {
      return Promise.resolve({
        json: () => Promise.resolve({ success: true }),
      } as Response);
    }
    if (url.includes('/admin/settings')) {
      return Promise.resolve({
        json: () => Promise.resolve(overrides.settings ?? { success: true, data: mockSettings }),
      } as Response);
    }
    return Promise.resolve({
      json: () => Promise.resolve({ success: true, data: null }),
    } as Response);
  });
}

describe('AdminUpdatesPage', () => {
  beforeEach(() => {
    global.fetch = mockFetch();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the title and header', async () => {
    render(<AdminUpdatesPage />);
    await waitFor(() => {
      expect(screen.getByText('System Updates')).toBeInTheDocument();
    });
  });

  it('displays the current version', async () => {
    render(<AdminUpdatesPage />);
    await waitFor(() => {
      expect(screen.getByTestId('current-version')).toHaveTextContent('4.8.0');
    });
  });

  it('shows version card with build info', async () => {
    render(<AdminUpdatesPage />);
    await waitFor(() => {
      expect(screen.getByTestId('version-card')).toBeInTheDocument();
    });
  });

  it('shows check for updates button', async () => {
    render(<AdminUpdatesPage />);
    await waitFor(() => {
      expect(screen.getByTestId('check-btn')).toBeInTheDocument();
    });
  });

  it('shows up-to-date message after check when no update', async () => {
    render(<AdminUpdatesPage />);
    await waitFor(() => expect(screen.getByTestId('check-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('check-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('up-to-date')).toBeInTheDocument();
    });
  });

  it('shows update available banner when update exists', async () => {
    global.fetch = mockFetch({
      check: {
        success: true,
        data: {
          available: true,
          latest_version: '4.9.0',
          current_version: '4.8.0',
          release_url: 'https://github.com/nash87/parkhub-rust/releases/v4.9.0',
          release_notes: 'Bug fixes and improvements',
          published_at: '2026-04-12T00:00:00Z',
        },
      },
    });

    render(<AdminUpdatesPage />);
    await waitFor(() => expect(screen.getByTestId('check-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('check-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('update-available')).toBeInTheDocument();
      expect(screen.getByText('4.9.0')).toBeInTheDocument();
      expect(screen.getByText('Bug fixes and improvements')).toBeInTheDocument();
    });
  });

  it('renders auto-update toggle', async () => {
    render(<AdminUpdatesPage />);
    await waitFor(() => {
      expect(screen.getByTestId('auto-update-toggle')).toBeInTheDocument();
      expect(screen.getByTestId('auto-update-toggle')).toHaveAttribute('aria-checked', 'false');
    });
  });

  it('renders update history list', async () => {
    render(<AdminUpdatesPage />);
    await waitFor(() => {
      const rows = screen.getAllByTestId('history-row');
      expect(rows).toHaveLength(2);
    });
  });

  it('shows empty state for update history when none', async () => {
    global.fetch = mockFetch({
      history: { success: true, data: [] },
    });
    render(<AdminUpdatesPage />);
    await waitFor(() => {
      expect(screen.getByText('No update history yet')).toBeInTheDocument();
    });
  });

  it('renders channel selector with stable selected by default', async () => {
    render(<AdminUpdatesPage />);
    await waitFor(() => {
      expect(screen.getByTestId('channel-stable')).toBeInTheDocument();
      expect(screen.getByTestId('channel-beta')).toBeInTheDocument();
    });
  });

  it('toggles auto-update and calls settings API', async () => {
    const fetchMock = mockFetch();
    global.fetch = fetchMock;

    render(<AdminUpdatesPage />);
    await waitFor(() => expect(screen.getByTestId('auto-update-toggle')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('auto-update-toggle'));

    await waitFor(() => {
      const settingsCalls = fetchMock.mock.calls.filter(
        ([url, opts]: [string, any]) => url.includes('/admin/settings') && opts?.method === 'POST'
      );
      expect(settingsCalls.length).toBeGreaterThan(0);
    });
  });
});
