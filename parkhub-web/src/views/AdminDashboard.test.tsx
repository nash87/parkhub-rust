import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
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
});
