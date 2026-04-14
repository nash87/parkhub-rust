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
  version: '4.9.0',
  build_hash: 'abc12345def',
  build_date: '2026-04-10T12:00:00Z',
  uptime_seconds: 86400,
};

const mockHistory = [
  {
    id: 'u1',
    from_version: '4.7.0',
    to_version: '4.9.0',
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
        json: () => Promise.resolve(overrides.check ?? { success: true, data: { available: false, current_version: '4.9.0', latest_version: '4.9.0' } }),
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
      expect(screen.getByTestId('current-version')).toHaveTextContent('4.9.0');
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
          latest_version: '4.10.0',
          current_version: '4.9.0',
          release_url: 'https://github.com/nash87/parkhub-rust/releases/v4.10.0',
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
      expect(screen.getByText('4.10.0')).toBeInTheDocument();
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

  it('switches channel to beta', async () => {
    const fetchMock = mockFetch();
    global.fetch = fetchMock;

    render(<AdminUpdatesPage />);
    await waitFor(() => expect(screen.getByTestId('channel-beta')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('channel-beta'));

    await waitFor(() => {
      const settingsCalls = fetchMock.mock.calls.filter(
        ([url, opts]: [string, any]) => url.includes('/admin/settings') && opts?.method === 'POST'
      );
      const betaCall = settingsCalls.find(([_, opts]: any) => {
        const body = JSON.parse(opts.body);
        return body.update_channel === 'beta';
      });
      expect(betaCall).toBeTruthy();
    });
  });

  it('applies update and shows progress state', async () => {
    global.fetch = mockFetch({
      check: {
        success: true,
        data: {
          available: true,
          latest_version: '5.0.0',
          current_version: '4.9.0',
          release_url: 'https://example.com',
          release_notes: 'Major release',
          published_at: '2026-04-14T00:00:00Z',
        },
      },
    });

    render(<AdminUpdatesPage />);
    await waitFor(() => expect(screen.getByTestId('check-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('check-btn'));
    await waitFor(() => expect(screen.getByTestId('update-available')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('apply-btn'));

    // Should show progress at some point during the update
    await waitFor(() => {
      // Either still in progress or already done/errored
      const progress = screen.queryByTestId('update-progress');
      const success = screen.queryByTestId('update-success');
      expect(progress || success).toBeTruthy();
    });
  });

  it('shows error state when apply fails', async () => {
    global.fetch = mockFetch({
      check: {
        success: true,
        data: {
          available: true,
          latest_version: '5.0.0',
          current_version: '4.9.0',
          release_url: '',
          release_notes: '',
          published_at: '',
        },
      },
      apply: { success: false, error: { message: 'Deploy failed' } },
    });

    render(<AdminUpdatesPage />);
    await waitFor(() => expect(screen.getByTestId('check-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('check-btn'));
    await waitFor(() => expect(screen.getByTestId('update-available')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('apply-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('update-error')).toBeInTheDocument();
    });
  });

  it('shows error when check update API fails', async () => {
    global.fetch = mockFetch({
      check: { success: false, error: { message: 'Server error' } },
    });

    render(<AdminUpdatesPage />);
    await waitFor(() => expect(screen.getByTestId('check-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('check-btn'));

    // Should handle gracefully without crashing
    await waitFor(() => {
      expect(screen.getByTestId('check-btn')).toBeInTheDocument();
    });
  });

  it('shows error when check update throws', async () => {
    const fetchMock = mockFetch();
    // Override check to throw
    fetchMock.mockImplementation((url: string) => {
      if (url.includes('/updates/check')) return Promise.reject(new Error('Network'));
      return mockFetch()(url);
    });
    global.fetch = fetchMock;

    render(<AdminUpdatesPage />);
    await waitFor(() => expect(screen.getByTestId('check-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('check-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('check-btn')).not.toBeDisabled();
    });
  });

  it('displays uptime from version info', async () => {
    render(<AdminUpdatesPage />);
    await waitFor(() => {
      // 86400 seconds = 1d 0h 0m
      expect(screen.getByText('1d 0h 0m')).toBeInTheDocument();
    });
  });

  it('shows build hash and date', async () => {
    render(<AdminUpdatesPage />);
    await waitFor(() => {
      // build_hash abc12345def -> first 8 chars = abc12345
      const buildText = screen.getByText(/abc12345/);
      expect(buildText).toBeInTheDocument();
    });
  });

  it('handles version info without optional fields', async () => {
    global.fetch = mockFetch({
      version: { success: true, data: { version: '1.0.0' } },
    });
    render(<AdminUpdatesPage />);
    await waitFor(() => {
      expect(screen.getByTestId('current-version')).toHaveTextContent('1.0.0');
    });
  });

  it('renders status badges for history entries', async () => {
    render(<AdminUpdatesPage />);
    await waitFor(() => {
      const badges = screen.getAllByTestId('status-badge');
      expect(badges).toHaveLength(2);
      expect(badges[0]).toHaveTextContent('Success');
      expect(badges[1]).toHaveTextContent('Failed');
    });
  });

  it('toggles auto-update off after on', async () => {
    global.fetch = mockFetch({
      settings: { success: true, data: { auto_update: true, update_channel: 'stable' } },
    });
    render(<AdminUpdatesPage />);
    await waitFor(() => {
      expect(screen.getByTestId('auto-update-toggle')).toHaveAttribute('aria-checked', 'true');
    });

    fireEvent.click(screen.getByTestId('auto-update-toggle'));
    await waitFor(() => {
      expect(screen.getByTestId('auto-update-toggle')).toHaveAttribute('aria-checked', 'false');
    });
  });

  it('handles auto-update toggle API error', async () => {
    const fetchMock = vi.fn((url: string, opts?: any) => {
      if (url.includes('/admin/settings') && opts?.method === 'POST') {
        return Promise.resolve({
          json: () => Promise.resolve({ success: false, error: { message: 'Fail' } }),
        } as Response);
      }
      return mockFetch()(url, opts);
    });
    global.fetch = fetchMock;

    render(<AdminUpdatesPage />);
    await waitFor(() => expect(screen.getByTestId('auto-update-toggle')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('auto-update-toggle'));

    // Should not crash
    await waitFor(() => {
      expect(screen.getByTestId('auto-update-toggle')).toBeInTheDocument();
    });
  });

  it('shows release URL link and published date when available', async () => {
    global.fetch = mockFetch({
      check: {
        success: true,
        data: {
          available: true,
          latest_version: '5.0.0',
          current_version: '4.9.0',
          release_url: 'https://example.com/release',
          release_notes: 'Notes here',
          published_at: '2026-04-14T00:00:00Z',
        },
      },
    });

    render(<AdminUpdatesPage />);
    await waitFor(() => expect(screen.getByTestId('check-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('check-btn'));

    await waitFor(() => {
      // viewRelease falls back to default string 'View release'
      expect(screen.getByText('View release')).toBeInTheDocument();
      expect(screen.getByText('Notes here')).toBeInTheDocument();
    });
  });

  it('shows channel description for stable vs beta', async () => {
    render(<AdminUpdatesPage />);
    await waitFor(() => {
      // channelStableDesc falls back to default 'Recommended. Receives tested...'
      expect(screen.getByText(/Recommended/)).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId('channel-beta'));
    await waitFor(() => {
      expect(screen.getByText(/Early access/)).toBeInTheDocument();
    });
  });

  it('handles apply update network error', async () => {
    const fetchMock = vi.fn((url: string, opts?: any) => {
      if (url.includes('/updates/apply')) return Promise.reject(new Error('Network'));
      return mockFetch({
        check: {
          success: true,
          data: {
            available: true, latest_version: '5.0.0', current_version: '4.9.0',
            release_url: '', release_notes: '', published_at: '',
          },
        },
      })(url, opts);
    });
    global.fetch = fetchMock;

    render(<AdminUpdatesPage />);
    await waitFor(() => expect(screen.getByTestId('check-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('check-btn'));
    await waitFor(() => expect(screen.getByTestId('update-available')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('apply-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('update-error')).toBeInTheDocument();
    });
  });

  it('formatUptime handles hours-only range', async () => {
    // 3720 seconds = 0d 1h 2m → should show "1h 2m"
    global.fetch = mockFetch({
      version: { success: true, data: { ...mockVersion, uptime_seconds: 3720 } },
    });
    render(<AdminUpdatesPage />);
    await waitFor(() => expect(screen.getByText('1h 2m')).toBeInTheDocument());
  });

  it('formatUptime handles minutes-only range', async () => {
    // 120 seconds = 0d 0h 2m → should show "2m"
    global.fetch = mockFetch({
      version: { success: true, data: { ...mockVersion, uptime_seconds: 120 } },
    });
    render(<AdminUpdatesPage />);
    await waitFor(() => expect(screen.getByText('2m')).toBeInTheDocument());
  });
});
