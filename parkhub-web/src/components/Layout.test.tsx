import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockNavigate = vi.fn();
const mockLogout = vi.fn();
const mockSetTheme = vi.fn();
let mockUser: any = {
  id: '1',
  username: 'testuser',
  email: 'test@example.com',
  name: 'Alice Smith',
  role: 'user',
};

vi.mock('react-router-dom', () => ({
  Outlet: () => <div data-testid="outlet">Page content</div>,
  NavLink: ({ to, children, end, onClick, className, ...props }: any) => {
    const cls = typeof className === 'function' ? className({ isActive: to === '/' && end }) : className;
    return <a href={to} onClick={onClick} className={cls} {...props}>{children}</a>;
  },
  useNavigate: () => mockNavigate,
}));

vi.mock('../context/AuthContext', () => ({
  useAuth: () => ({
    user: mockUser,
    loading: false,
    login: vi.fn(),
    logout: mockLogout,
    refreshUser: vi.fn(),
  }),
}));

vi.mock('../context/ThemeContext', () => ({
  useTheme: () => ({
    resolved: 'light' as const,
    theme: 'light' as const,
    setTheme: mockSetTheme,
  }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'nav.dashboard': 'Dashboard',
        'nav.bookings': 'Bookings',
        'nav.vehicles': 'Vehicles',
        'nav.absences': 'Absences',
        'nav.team': 'Team',
        'nav.calendar': 'Calendar',
        'nav.credits': 'Credits',
        'nav.notifications': 'Notifications',
        'nav.profile': 'Profile',
        'nav.admin': 'Admin',
        'nav.lightMode': 'Light Mode',
        'nav.darkMode': 'Dark Mode',
        'nav.logout': 'Log Out',
        'nav.openMenu': 'Open navigation menu',
        'nav.closeMenu': 'Close navigation menu',
        'nav.skipToContent': 'Skip to content',
        'nav.switchToLight': 'Switch to light mode',
        'nav.switchToDark': 'Switch to dark mode',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, variants, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    aside: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <aside ref={ref} {...props}>{children}</aside>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  House: (props: any) => <span data-testid="icon-house" {...props} />,
  CalendarCheck: (props: any) => <span data-testid="icon-calendar-check" {...props} />,
  Car: (props: any) => <span data-testid="icon-car" {...props} />,
  Calendar: (props: any) => <span data-testid="icon-calendar" {...props} />,
  CalendarX: (props: any) => <span data-testid="icon-calendar-x" {...props} />,
  Coins: (props: any) => <span data-testid="icon-coins" {...props} />,
  UserCircle: (props: any) => <span data-testid="icon-user" {...props} />,
  Users: (props: any) => <span data-testid="icon-users" {...props} />,
  Bell: (props: any) => <span data-testid="icon-bell" {...props} />,
  GearSix: (props: any) => <span data-testid="icon-gear" {...props} />,
  SignOut: (props: any) => <span data-testid="icon-signout" {...props} />,
  List: (props: any) => <span data-testid="icon-list" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  CarSimple: (props: any) => <span data-testid="icon-car-simple" {...props} />,
  SunDim: (props: any) => <span data-testid="icon-sun" {...props} />,
  Moon: (props: any) => <span data-testid="icon-moon" {...props} />,
  CalendarPlus: (props: any) => <span data-testid="icon-calendar-plus" {...props} />,
}));

import { Layout } from './Layout';

describe('Layout', () => {
  beforeEach(() => {
    mockNavigate.mockClear();
    mockLogout.mockClear();
    mockSetTheme.mockClear();
    mockUser = {
      id: '1',
      username: 'testuser',
      email: 'test@example.com',
      name: 'Alice Smith',
      role: 'user',
    };
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the ParkHub logo text', () => {
    render(<Layout />);
    // There are multiple "ParkHub" texts (desktop + mobile), just check at least one
    const logos = screen.getAllByText('ParkHub');
    expect(logos.length).toBeGreaterThanOrEqual(1);
  });

  it('renders all navigation items in the desktop sidebar', () => {
    render(<Layout />);
    expect(screen.getAllByText('Dashboard').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Bookings').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Vehicles').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Absences').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Team').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Calendar').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Credits').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Notifications').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Profile').length).toBeGreaterThanOrEqual(1);
  });

  it('renders the Outlet for page content', () => {
    render(<Layout />);
    expect(screen.getByTestId('outlet')).toBeInTheDocument();
  });

  it('displays user name and email', () => {
    render(<Layout />);
    expect(screen.getByText('Alice Smith')).toBeInTheDocument();
    expect(screen.getByText('test@example.com')).toBeInTheDocument();
  });

  it('displays user initial avatar', () => {
    render(<Layout />);
    expect(screen.getByText('A')).toBeInTheDocument();
  });

  it('calls logout and navigates to /welcome on logout click', async () => {
    const user = userEvent.setup();
    render(<Layout />);

    // Multiple logout buttons (desktop sidebar + possibly mobile)
    const logoutButtons = screen.getAllByText('Log Out');
    await user.click(logoutButtons[0]);

    expect(mockLogout).toHaveBeenCalledOnce();
    expect(mockNavigate).toHaveBeenCalledWith('/welcome');
  });

  it('does not show Admin link for regular users', () => {
    render(<Layout />);
    expect(screen.queryByText('Admin')).not.toBeInTheDocument();
  });

  it('shows Admin link for admin users', () => {
    mockUser = { ...mockUser, role: 'admin' };
    render(<Layout />);
    expect(screen.getAllByText('Admin').length).toBeGreaterThanOrEqual(1);
  });

  it('shows Admin link for superadmin users', () => {
    mockUser = { ...mockUser, role: 'superadmin' };
    render(<Layout />);
    expect(screen.getAllByText('Admin').length).toBeGreaterThanOrEqual(1);
  });

  it('renders the mobile menu toggle button', () => {
    render(<Layout />);
    expect(screen.getByLabelText('Open navigation menu')).toBeInTheDocument();
  });

  it('opens the mobile sidebar when menu button is clicked', async () => {
    const user = userEvent.setup();
    render(<Layout />);

    await user.click(screen.getByLabelText('Open navigation menu'));

    // Mobile sidebar opens with a dialog role and close button
    expect(screen.getByLabelText('Navigation menu')).toBeInTheDocument();
    expect(screen.getByLabelText('Close navigation menu')).toBeInTheDocument();
  });

  it('closes the mobile sidebar when close button is clicked', async () => {
    const user = userEvent.setup();
    render(<Layout />);

    // Open
    await user.click(screen.getByLabelText('Open navigation menu'));
    expect(screen.getByLabelText('Navigation menu')).toBeInTheDocument();

    // Close
    await user.click(screen.getByLabelText('Close navigation menu'));

    // AnimatePresence mock renders children immediately; the sidebar state is toggled
    // Since AnimatePresence is mocked to just render children, we check the state changed
    // by verifying the close button is gone (sidebar closed means no dialog)
    expect(screen.queryByLabelText('Navigation menu')).not.toBeInTheDocument();
  });

  it('renders theme toggle in the sidebar', async () => {
    const user = userEvent.setup();
    render(<Layout />);

    // Desktop theme toggle text — resolved is 'light' so it shows "Dark Mode"
    expect(screen.getByText('Dark Mode')).toBeInTheDocument();

    await user.click(screen.getByText('Dark Mode'));
    expect(mockSetTheme).toHaveBeenCalledWith('dark');
  });

  it('falls back to username when name is not set', () => {
    mockUser = { ...mockUser, name: '', username: 'fallbackuser' };
    render(<Layout />);
    expect(screen.getByText('fallbackuser')).toBeInTheDocument();
  });
});
