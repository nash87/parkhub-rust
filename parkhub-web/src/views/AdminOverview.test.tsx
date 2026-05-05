import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

const mockAdminStats = vi.fn();
const mockGetBookings = vi.fn();
const mockGetLots = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    adminStats: (...args: any[]) => mockAdminStats(...args),
    getBookings: (...args: any[]) => mockGetBookings(...args),
    getLots: (...args: any[]) => mockGetLots(...args),
  },
}));

vi.mock('@phosphor-icons/react', () => ({
  ArrowRight: (props: any) => <span data-testid="icon-arrow" {...props} />,
  Buildings: (props: any) => <span data-testid="icon-buildings" {...props} />,
  CalendarCheck: (props: any) => <span data-testid="icon-calendar" {...props} />,
  ChartLine: (props: any) => <span data-testid="icon-chart" {...props} />,
  GearSix: (props: any) => <span data-testid="icon-gear" {...props} />,
  Lightning: (props: any) => <span data-testid="icon-lightning" {...props} />,
  Megaphone: (props: any) => <span data-testid="icon-megaphone" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  Users: (props: any) => <span data-testid="icon-users" {...props} />,
  WarningCircle: (props: any) => <span data-testid="icon-warning" {...props} />,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'common.loading': 'Loading',
        'admin.overview': 'Overview',
        'admin.overviewSubtitle': 'Instance status, operating priorities, and direct administration paths.',
        'admin.totalUsers': 'Total Users',
        'admin.totalLots': 'Parking Lots',
        'admin.totalBookings': 'Total Bookings',
        'admin.activeBookings': 'Active Bookings',
        'admin.slotsOccupied': 'slots occupied',
        'admin.overviewUsersHelper': 'Known user accounts',
        'admin.overviewBookingsHelper': 'All recorded reservations',
        'admin.overviewActiveHelper': 'Currently in effect',
        'admin.operatingSnapshot': 'Operating snapshot',
        'admin.utilizationRate': 'Utilization Rate',
        'admin.activeBookingRate': 'Active Booking Rate',
        'admin.avgBookingsPerUser': 'Avg. Bookings per User',
        'admin.parkingCapacity': 'Parking Capacity',
        'admin.quickActions': 'Quick actions',
        'admin.settings': 'Settings',
        'admin.users': 'Users',
        'admin.announcements': 'Announcements',
        'admin.reports': 'Reports',
        'admin.noLotsConfigured': 'No parking lots configured',
      };
      return map[key] || fallback || key;
    },
  }),
}));

import { AdminOverviewPage } from './AdminOverview';

function renderOverview() {
  return render(
    <MemoryRouter>
      <AdminOverviewPage />
    </MemoryRouter>,
  );
}

describe('AdminOverviewPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAdminStats.mockResolvedValue({
      success: true,
      data: { total_users: 50, total_lots: 2, total_bookings: 100, active_bookings: 10 },
    });
    mockGetBookings.mockResolvedValue({ success: true, data: [{ id: 'b1' }, { id: 'b2' }] });
    mockGetLots.mockResolvedValue({
      success: true,
      data: [
        { id: 'l1', name: 'North', total_slots: 20, available_slots: 5 },
        { id: 'l2', name: 'South', total_slots: 10, available_slots: 10 },
      ],
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading spinner initially', () => {
    mockAdminStats.mockReturnValue(new Promise(() => {}));
    mockGetBookings.mockReturnValue(new Promise(() => {}));
    mockGetLots.mockReturnValue(new Promise(() => {}));

    renderOverview();

    expect(screen.getByTestId('icon-spinner')).toBeInTheDocument();
  });

  it('renders overview metrics and operating snapshot', async () => {
    renderOverview();

    await waitFor(() => expect(screen.getByText('Overview')).toBeInTheDocument());

    expect(screen.getByText('Total Users')).toBeInTheDocument();
    expect(screen.getByText('50')).toBeInTheDocument();
    expect(screen.getByText('Parking Lots')).toBeInTheDocument();
    expect(screen.getByText('2')).toBeInTheDocument();
    expect(screen.getByText('Total Bookings')).toBeInTheDocument();
    expect(screen.getByText('100')).toBeInTheDocument();
    expect(screen.getByText('Active Bookings')).toBeInTheDocument();
    expect(screen.getByText('10')).toBeInTheDocument();
    expect(screen.getByText('Utilization Rate')).toBeInTheDocument();
    expect(screen.getByText('50%')).toBeInTheDocument();
    expect(screen.getByText('Active Booking Rate')).toBeInTheDocument();
    expect(screen.getByText('10%')).toBeInTheDocument();
    expect(screen.getByText('Avg. Bookings per User')).toBeInTheDocument();
    expect(screen.getByText('2.0')).toBeInTheDocument();
  });

  it('links to high-value admin routes instead of duplicating the reports page', async () => {
    renderOverview();

    await waitFor(() => expect(screen.getByText('Quick actions')).toBeInTheDocument());

    expect(screen.getByRole('link', { name: /settings/i })).toHaveAttribute('href', '/admin/settings');
    expect(screen.getByRole('link', { name: /users/i })).toHaveAttribute('href', '/admin/users');
    expect(screen.getByRole('link', { name: /reports/i })).toHaveAttribute('href', '/admin/reports');
  });

  it('surfaces an explicit setup warning when no lots exist', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [] });

    renderOverview();

    await waitFor(() => {
      expect(screen.getByText('No parking lots configured')).toBeInTheDocument();
    });
  });
});
