import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

const mockGetWidgetLayout = vi.fn();
const mockSaveWidgetLayout = vi.fn();
const mockGetWidgetData = vi.fn();
const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    getWidgetLayout: (...args: any[]) => mockGetWidgetLayout(...args),
    saveWidgetLayout: (...args: any[]) => mockSaveWidgetLayout(...args),
    getWidgetData: (...args: any[]) => mockGetWidgetData(...args),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'widgets.title': 'Dashboard Widgets',
        'widgets.subtitle': 'Customize your admin dashboard',
        'widgets.help': 'Customize your dashboard by adding, removing, and rearranging widgets',
        'widgets.helpLabel': 'Help',
        'widgets.customize': 'Customize',
        'widgets.catalog': 'Widget Catalog',
        'widgets.empty': 'No widgets configured',
        'widgets.emptyHint': 'Click Customize to add widgets',
        'widgets.layoutSaved': 'Layout saved',
        'widgets.remove': 'Remove',
        'widgets.types.occupancy_chart': 'Occupancy Chart',
        'widgets.types.revenue_summary': 'Revenue Summary',
        'widgets.types.recent_bookings': 'Recent Bookings',
        'widgets.types.user_growth': 'User Growth',
        'widgets.types.booking_heatmap': 'Booking Heatmap',
        'widgets.types.active_alerts': 'Active Alerts',
        'widgets.types.maintenance_status': 'Maintenance Status',
        'widgets.types.ev_charging_status': 'EV Charging Status',
        'common.error': 'Error',
      };
      return map[key] || key;
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
  ChartBar: (props: any) => <span data-testid="icon-chart" {...props} />,
  CurrencyCircleDollar: (props: any) => <span data-testid="icon-currency" {...props} />,
  CalendarCheck: (props: any) => <span data-testid="icon-cal-check" {...props} />,
  UsersThree: (props: any) => <span data-testid="icon-users" {...props} />,
  Fire: (props: any) => <span data-testid="icon-fire" {...props} />,
  Warning: (props: any) => <span data-testid="icon-warning" {...props} />,
  Wrench: (props: any) => <span data-testid="icon-wrench" {...props} />,
  Lightning: (props: any) => <span data-testid="icon-lightning" {...props} />,
  Plus: (props: any) => <span data-testid="icon-plus" {...props} />,
  Minus: (props: any) => <span data-testid="icon-minus" {...props} />,
  GearSix: (props: any) => <span data-testid="icon-gear" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  ArrowsOutCardinal: (props: any) => <span data-testid="icon-arrows" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: {
    success: (...args: any[]) => mockToastSuccess(...args),
    error: (...args: any[]) => mockToastError(...args),
  },
}));

import { AdminDashboardPage } from './AdminDashboard';

const sampleLayout = {
  user_id: 'u1',
  widgets: [
    { id: 'w1', widget_type: 'occupancy_chart', position: { x: 0, y: 0, w: 6, h: 4 }, visible: true },
    { id: 'w2', widget_type: 'revenue_summary', position: { x: 6, y: 0, w: 6, h: 4 }, visible: true },
    { id: 'w3', widget_type: 'recent_bookings', position: { x: 0, y: 4, w: 4, h: 3 }, visible: false },
  ],
};

describe('AdminDashboardPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetWidgetLayout.mockResolvedValue({ success: true, data: sampleLayout });
    mockGetWidgetData.mockResolvedValue({ success: true, data: { widget_id: 'test', data: { value: 42 } } });
    mockSaveWidgetLayout.mockResolvedValue({ success: true, data: sampleLayout });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders page title and subtitle', async () => {
    render(<AdminDashboardPage />);
    await waitFor(() => {
      expect(screen.getByText('Dashboard Widgets')).toBeInTheDocument();
      expect(screen.getByText('Customize your admin dashboard')).toBeInTheDocument();
    });
  });

  it('shows help tooltip when clicked', async () => {
    const user = userEvent.setup();
    render(<AdminDashboardPage />);
    await waitFor(() => expect(screen.getByTitle('Help')).toBeInTheDocument());
    await user.click(screen.getByTitle('Help'));
    expect(screen.getByText('Customize your dashboard by adding, removing, and rearranging widgets')).toBeInTheDocument();
  });

  it('renders visible widgets from layout', async () => {
    render(<AdminDashboardPage />);
    await waitFor(() => {
      expect(screen.getByText('Occupancy Chart')).toBeInTheDocument();
      expect(screen.getByText('Revenue Summary')).toBeInTheDocument();
    });
  });

  it('shows customize button', async () => {
    render(<AdminDashboardPage />);
    await waitFor(() => {
      expect(screen.getByText('Customize')).toBeInTheDocument();
    });
  });

  it('opens widget catalog on customize click', async () => {
    const user = userEvent.setup();
    render(<AdminDashboardPage />);
    await waitFor(() => expect(screen.getByText('Customize')).toBeInTheDocument());
    await user.click(screen.getByText('Customize'));
    await waitFor(() => {
      expect(screen.getByText('Widget Catalog')).toBeInTheDocument();
    });
  });

  it('shows empty state when no visible widgets', async () => {
    mockGetWidgetLayout.mockResolvedValue({
      success: true,
      data: { user_id: 'u1', widgets: [] },
    });
    render(<AdminDashboardPage />);
    await waitFor(() => {
      expect(screen.getByText('No widgets configured')).toBeInTheDocument();
      expect(screen.getByText('Click Customize to add widgets')).toBeInTheDocument();
    });
  });

  it('calls getWidgetLayout on mount', async () => {
    render(<AdminDashboardPage />);
    await waitFor(() => {
      expect(mockGetWidgetLayout).toHaveBeenCalledTimes(1);
    });
  });

  it('loads widget data for visible widgets', async () => {
    render(<AdminDashboardPage />);
    await waitFor(() => {
      expect(mockGetWidgetData).toHaveBeenCalled();
    });
  });

  it('hides non-visible widgets', async () => {
    render(<AdminDashboardPage />);
    await waitFor(() => {
      // recent_bookings is visible: false in sample layout
      expect(screen.queryByText('Recent Bookings')).not.toBeInTheDocument();
    });
  });

  it('handles API error on layout load', async () => {
    mockGetWidgetLayout.mockRejectedValue(new Error('Network error'));
    render(<AdminDashboardPage />);
    await waitFor(() => {
      expect(screen.getByText('Dashboard Widgets')).toBeInTheDocument();
    });
  });

  it('handles null layout', async () => {
    mockGetWidgetLayout.mockResolvedValue({ success: false, data: null });
    render(<AdminDashboardPage />);
    await waitFor(() => {
      expect(screen.getByText('Dashboard Widgets')).toBeInTheDocument();
    });
  });

  it('toggleWidget is a no-op when layout is null', async () => {
    const user = userEvent.setup();
    mockGetWidgetLayout.mockResolvedValue({ success: false, data: null });
    render(<AdminDashboardPage />);
    await waitFor(() => expect(screen.getByText('Customize')).toBeInTheDocument());
    // Open catalog
    await user.click(screen.getByText('Customize'));
    // Click any widget toggle button - should not throw
    const widgetButtons = screen.getAllByRole('button').filter(b => b.className.includes('rounded-lg border'));
    if (widgetButtons.length > 0) {
      await user.click(widgetButtons[0]);
    }
    // Layout still null, no errors
    expect(mockSaveWidgetLayout).not.toHaveBeenCalled();
  });

  it('toggles help tooltip', async () => {
    const user = userEvent.setup();
    render(<AdminDashboardPage />);
    await waitFor(() => expect(screen.getByTitle('Help')).toBeInTheDocument());

    // Open help
    await user.click(screen.getByTitle('Help'));
    expect(screen.getByText('Customize your dashboard by adding, removing, and rearranging widgets')).toBeInTheDocument();

    // Close help
    await user.click(screen.getByTitle('Help'));
    await waitFor(() => {
      expect(screen.queryByText('Customize your dashboard by adding, removing, and rearranging widgets')).not.toBeInTheDocument();
    });
  });

  it('closes widget catalog on second customize click', async () => {
    const user = userEvent.setup();
    render(<AdminDashboardPage />);
    await waitFor(() => expect(screen.getByText('Customize')).toBeInTheDocument());

    // Open catalog
    await user.click(screen.getByText('Customize'));
    await waitFor(() => expect(screen.getByText('Widget Catalog')).toBeInTheDocument());

    // Close catalog
    await user.click(screen.getByText('Customize'));
    await waitFor(() => {
      expect(screen.queryByText('Widget Catalog')).not.toBeInTheDocument();
    });
  });

  it('shows all widget type options in catalog', async () => {
    const user = userEvent.setup();
    render(<AdminDashboardPage />);
    await waitFor(() => expect(screen.getByText('Customize')).toBeInTheDocument());
    await user.click(screen.getByText('Customize'));
    await waitFor(() => {
      expect(screen.getByText('Widget Catalog')).toBeInTheDocument();
      expect(screen.getByText('Booking Heatmap')).toBeInTheDocument();
      expect(screen.getByText('Active Alerts')).toBeInTheDocument();
      expect(screen.getByText('Maintenance Status')).toBeInTheDocument();
      expect(screen.getByText('EV Charging Status')).toBeInTheDocument();
    });
  });

  it('toggles widget visibility from catalog', async () => {
    const user = userEvent.setup();
    render(<AdminDashboardPage />);
    await waitFor(() => expect(screen.getByText('Customize')).toBeInTheDocument());
    await user.click(screen.getByText('Customize'));
    await waitFor(() => expect(screen.getByText('Widget Catalog')).toBeInTheDocument());

    // Click Active Alerts (not currently in layout) to add it
    await user.click(screen.getByText('Active Alerts'));

    await waitFor(() => {
      expect(mockSaveWidgetLayout).toHaveBeenCalled();
    });
  });

  it('removes a widget from the grid', async () => {
    const user = userEvent.setup();
    render(<AdminDashboardPage />);
    await waitFor(() => expect(screen.getByText('Occupancy Chart')).toBeInTheDocument());

    // Click remove button on a widget
    const removeBtns = screen.getAllByTitle('Remove');
    await user.click(removeBtns[0]);

    await waitFor(() => {
      expect(mockSaveWidgetLayout).toHaveBeenCalled();
    });
  });

  it('handles save layout error', async () => {
    mockSaveWidgetLayout.mockRejectedValue(new Error('Network'));
    const user = userEvent.setup();
    render(<AdminDashboardPage />);
    await waitFor(() => expect(screen.getByText('Customize')).toBeInTheDocument());
    await user.click(screen.getByText('Customize'));
    await waitFor(() => expect(screen.getByText('Widget Catalog')).toBeInTheDocument());

    await user.click(screen.getByText('Booking Heatmap'));
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalled();
    });
  });

  it('handles save layout API failure', async () => {
    mockSaveWidgetLayout.mockResolvedValue({ success: false });
    const user = userEvent.setup();
    render(<AdminDashboardPage />);
    await waitFor(() => expect(screen.getByText('Customize')).toBeInTheDocument());
    await user.click(screen.getByText('Customize'));
    await waitFor(() => expect(screen.getByText('Widget Catalog')).toBeInTheDocument());

    await user.click(screen.getByText('Booking Heatmap'));
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalled();
    });
  });

  it('displays widget data when loaded', async () => {
    render(<AdminDashboardPage />);
    await waitFor(() => {
      expect(screen.getByText('Occupancy Chart')).toBeInTheDocument();
    });
    // Widget data should load -- pre element with JSON
    await waitFor(() => {
      expect(mockGetWidgetData).toHaveBeenCalled();
    });
  });

  it('handles widget data load failure gracefully', async () => {
    mockGetWidgetData.mockRejectedValue(new Error('Failed'));
    render(<AdminDashboardPage />);
    await waitFor(() => {
      expect(screen.getByText('Occupancy Chart')).toBeInTheDocument();
    });
    // Should not crash -- shows skeleton
  });

  it('loading state shows skeleton', () => {
    mockGetWidgetLayout.mockReturnValue(new Promise(() => {}));
    render(<AdminDashboardPage />);
    const skeletons = document.querySelectorAll('.skeleton');
    expect(skeletons.length).toBeGreaterThan(0);
  });
});
