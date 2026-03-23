import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

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
});
