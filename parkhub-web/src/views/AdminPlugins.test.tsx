import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'plugins.title': 'Plugins',
        'plugins.subtitle': 'Extend ParkHub with plugins',
        'plugins.help': 'Extend ParkHub with plugins. Enable built-in plugins or configure custom integrations.',
        'plugins.helpLabel': 'Help',
        'plugins.totalPlugins': 'Total Plugins',
        'plugins.enabledPlugins': 'Enabled',
        'plugins.disabledPlugins': 'Disabled',
        'plugins.by': 'by',
        'plugins.enable': 'Enable',
        'plugins.disable': 'Disable',
        'plugins.configure': 'Configure',
        'plugins.routes': 'routes',
        'plugins.empty': 'No plugins available',
        'plugins.toggled': 'Plugin toggled',
        'plugins.configTitle': 'Plugin Configuration',
        'plugins.configSaved': 'Configuration saved',
        'common.error': 'Error',
        'common.cancel': 'Cancel',
        'common.save': 'Save',
        'common.saving': 'Saving...',
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
  PuzzlePiece: (props: any) => <span data-testid="icon-puzzle" {...props} />,
  ToggleLeft: (props: any) => <span data-testid="icon-toggle-left" {...props} />,
  ToggleRight: (props: any) => <span data-testid="icon-toggle-right" {...props} />,
  Gear: (props: any) => <span data-testid="icon-gear" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  Lightning: (props: any) => <span data-testid="icon-lightning" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: { success: (...a: any[]) => mockToastSuccess(...a), error: (...a: any[]) => mockToastError(...a) },
}));

import { AdminPluginsPage } from './AdminPlugins';

const samplePlugins = {
  plugins: [
    {
      id: 'slack-notifier',
      name: 'Slack Notifier',
      version: '1.0.0',
      description: 'Sends notifications to Slack',
      author: 'ParkHub',
      status: 'disabled',
      subscribed_events: ['booking_created', 'booking_cancelled', 'lot_full'],
      routes: [{ path: '/api/v1/plugins/slack-notifier/test', method: 'POST', description: 'Test' }],
      config: { webhook_url: '', channel: '#parking' },
    },
    {
      id: 'auto-assign-preferred',
      name: 'Auto-Assign Preferred Spot',
      version: '1.0.0',
      description: 'Assigns preferred spots automatically',
      author: 'ParkHub',
      status: 'enabled',
      subscribed_events: ['booking_created'],
      routes: [],
      config: { fallback_to_any: true },
    },
  ],
  total: 2,
  enabled: 1,
};

describe('AdminPluginsPage', () => {
  beforeEach(() => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/config')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { webhook_url: '', channel: '#parking' } }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: samplePlugins }) } as Response);
    }) as any;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the title after loading', async () => {
    render(<AdminPluginsPage />);
    await waitFor(() => {
      expect(screen.getByText('Plugins')).toBeInTheDocument();
    });
  });

  it('renders stats cards', async () => {
    render(<AdminPluginsPage />);
    await waitFor(() => {
      expect(screen.getByTestId('plugins-stats')).toBeInTheDocument();
      expect(screen.getByText('Total Plugins')).toBeInTheDocument();
    });
  });

  it('renders plugin cards', async () => {
    render(<AdminPluginsPage />);
    await waitFor(() => {
      const cards = screen.getAllByTestId('plugin-card');
      expect(cards).toHaveLength(2);
    });
  });

  it('shows plugin names and descriptions', async () => {
    render(<AdminPluginsPage />);
    await waitFor(() => {
      expect(screen.getByText('Slack Notifier')).toBeInTheDocument();
      expect(screen.getByText('Sends notifications to Slack')).toBeInTheDocument();
      expect(screen.getByText('Auto-Assign Preferred Spot')).toBeInTheDocument();
    });
  });

  it('shows event badges', async () => {
    render(<AdminPluginsPage />);
    await waitFor(() => {
      const bookingBadges = screen.getAllByText('booking created');
      expect(bookingBadges.length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText('lot full')).toBeInTheDocument();
    });
  });

  it('shows help text when help button clicked', async () => {
    render(<AdminPluginsPage />);
    await waitFor(() => {
      expect(screen.getByTestId('plugins-help-btn')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId('plugins-help-btn'));
    await waitFor(() => {
      expect(screen.getByTestId('plugins-help')).toBeInTheDocument();
    });
  });

  it('shows empty state when no plugins', async () => {
    global.fetch = vi.fn(() =>
      Promise.resolve({ json: () => Promise.resolve({ success: true, data: { plugins: [], total: 0, enabled: 0 } }) } as Response)
    ) as any;

    render(<AdminPluginsPage />);
    await waitFor(() => {
      expect(screen.getByTestId('plugins-empty')).toBeInTheDocument();
    });
  });

  it('opens config dialog when configure button clicked', async () => {
    render(<AdminPluginsPage />);
    await waitFor(() => {
      expect(screen.getByTestId('config-slack-notifier')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId('config-slack-notifier'));
    await waitFor(() => {
      expect(screen.getByTestId('config-dialog')).toBeInTheDocument();
    });
  });

  it('toggles plugin', async () => {
    render(<AdminPluginsPage />);
    await waitFor(() => {
      expect(screen.getByTestId('toggle-slack-notifier')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId('toggle-slack-notifier'));
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(expect.stringContaining('/toggle'), expect.objectContaining({ method: 'PUT' }));
    });
  });

  it('toggle plugin network error', async () => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/toggle')) return Promise.reject(new Error('net'));
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: samplePlugins }) } as Response);
    }) as any;

    render(<AdminPluginsPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('toggle-slack-notifier')));
    await waitFor(() => expect(mockToastError).toHaveBeenCalled());
  });

  it('opens config and edits boolean field', async () => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/auto-assign-preferred/config')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { fallback_to_any: true } }) } as Response);
      }
      if (url.includes('/config')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { webhook_url: '', channel: '#parking' } }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: samplePlugins }) } as Response);
    }) as any;
    render(<AdminPluginsPage />);
    await waitFor(() => expect(screen.getByTestId('config-auto-assign-preferred')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('config-auto-assign-preferred'));
    await waitFor(() => {
      expect(screen.getByTestId('config-dialog')).toBeInTheDocument();
    });
    // Toggle the boolean field
    const boolBtn = screen.getByTestId('config-field-fallback_to_any');
    fireEvent.click(boolBtn);
  });

  it('opens config and edits text field', async () => {
    render(<AdminPluginsPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('config-slack-notifier')));
    await waitFor(() => {
      expect(screen.getByTestId('config-dialog')).toBeInTheDocument();
    });
    const textField = screen.getByTestId('config-field-webhook_url');
    fireEvent.change(textField, { target: { value: 'https://new.com' } });
    expect((textField as HTMLInputElement).value).toBe('https://new.com');
  });

  it('saves config', async () => {
    render(<AdminPluginsPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('config-slack-notifier')));
    await waitFor(() => expect(screen.getByTestId('save-config-btn')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('save-config-btn'));
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('/config'),
        expect.objectContaining({ method: 'PUT' }),
      );
    });
  });

  it('save config network error', async () => {
    global.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'PUT' && url.includes('/config')) return Promise.reject(new Error('net'));
      if (url.includes('/config')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { webhook_url: '', channel: '#parking' } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: samplePlugins }) } as Response);
    }) as any;

    render(<AdminPluginsPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('config-slack-notifier')));
    await waitFor(() => fireEvent.click(screen.getByTestId('save-config-btn')));
    await waitFor(() => expect(mockToastError).toHaveBeenCalled());
  });

  it('cancels config dialog', async () => {
    render(<AdminPluginsPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('config-slack-notifier')));
    await waitFor(() => expect(screen.getByTestId('config-dialog')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Cancel'));
    await waitFor(() => expect(screen.queryByTestId('config-dialog')).not.toBeInTheDocument());
  });

  it('open config network error', async () => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/config')) return Promise.reject(new Error('net'));
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: samplePlugins }) } as Response);
    }) as any;

    render(<AdminPluginsPage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('config-slack-notifier')));
    await waitFor(() => expect(mockToastError).toHaveBeenCalled());
  });

  it('shows routes count when present', async () => {
    render(<AdminPluginsPage />);
    await waitFor(() => {
      expect(screen.getByText('1 routes')).toBeInTheDocument();
    });
  });

  it('load plugins network error', async () => {
    global.fetch = vi.fn(() => Promise.reject(new Error('net'))) as any;

    render(<AdminPluginsPage />);
    await waitFor(() => expect(mockToastError).toHaveBeenCalled());
  });
});
