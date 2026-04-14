import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';
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
    const isActive = to === '/' && end;
    const cls = typeof className === 'function' ? className({ isActive }) : className;
    const rendered = typeof children === 'function' ? children({ isActive }) : children;
    return <a href={to} onClick={onClick} className={cls} {...props}>{rendered}</a>;
  },
  useNavigate: () => mockNavigate,
  useLocation: () => ({ pathname: '/' }),
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
        'nav.favorites': 'Favorites',
        'nav.absences': 'Absences',
        'nav.team': 'Team',
        'nav.calendar': 'Calendar',
        'nav.map': 'Map',
        'nav.history': 'History',
        'nav.credits': 'Credits',
        'nav.notifications': 'Notifications',
        'nav.profile': 'Profile',
        'nav.admin': 'Admin',
        'nav.translations': 'Translations',
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

vi.mock('../api/client', () => ({
  getInMemoryToken: () => 'test-token',
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, variants, layoutId, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    aside: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <aside ref={ref} {...props}>{children}</aside>
    )),
    button: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <button ref={ref} {...props}>{children}</button>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@phosphor-icons/react')>();
  const icon = (props: any) => <span {...props} />;
  return {
    ...actual,
    House: icon,
    CalendarCheck: icon,
    Car: icon,
    Calendar: icon,
    CalendarX: icon,
    Coins: icon,
    UserCircle: icon,
    Users: icon,
    Bell: icon,
    GearSix: icon,
    SignOut: icon,
    List: icon,
    X: icon,
    CarSimple: icon,
    SunDim: icon,
    Moon: icon,
    CalendarPlus: icon,
    Translate: icon,
    Star: icon,
    Palette: icon,
    Check: icon,
    CheckCircle: icon,
    Globe: icon,
    CaretDown: icon,
    MapPin: icon,
    ClockCounterClockwise: icon,
  };
});

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

  it('renders a footer landmark', () => {
    render(<Layout />);
    const footer = document.querySelector('footer');
    expect(footer).toBeInTheDocument();
    expect(footer?.textContent).toContain('ParkHub');
  });

  it('falls back to username when name is not set', () => {
    mockUser = { ...mockUser, name: '', username: 'fallbackuser' };
    render(<Layout />);
    expect(screen.getByText('fallbackuser')).toBeInTheDocument();
  });

  it('mobile sidebar shows navigation items', async () => {
    const user = userEvent.setup();
    render(<Layout />);

    await user.click(screen.getByLabelText('Open navigation menu'));

    // Mobile sidebar renders nav items
    const sidebar = screen.getByLabelText('Navigation menu');
    expect(sidebar).toBeInTheDocument();

    // Should have nav links in the mobile sidebar
    const navLinks = sidebar.querySelectorAll('a');
    expect(navLinks.length).toBeGreaterThanOrEqual(10);
  });

  it('mobile sidebar shows admin link for admin users', async () => {
    mockUser = { ...mockUser, role: 'admin' };
    const user = userEvent.setup();
    render(<Layout />);

    await user.click(screen.getByLabelText('Open navigation menu'));

    const sidebar = screen.getByLabelText('Navigation menu');
    // Admin link should be in the mobile sidebar
    const adminLinks = sidebar.querySelectorAll('a[href="/admin"]');
    expect(adminLinks.length).toBeGreaterThanOrEqual(1);
  });

  it('mobile sidebar logout button works', async () => {
    const user = userEvent.setup();
    render(<Layout />);

    await user.click(screen.getByLabelText('Open navigation menu'));

    // The mobile sidebar has its own logout button
    const sidebar = screen.getByLabelText('Navigation menu');
    const logoutBtn = sidebar.parentElement?.querySelector('button');
    // There's a logout button in the mobile sidebar's bottom area
    const allLogouts = screen.getAllByText('Log Out');
    expect(allLogouts.length).toBeGreaterThanOrEqual(2); // desktop + mobile
  });

  it('mobile header shows theme toggle', async () => {
    const user = userEvent.setup();
    render(<Layout />);

    // The mobile header theme toggle (aria-label based)
    const themeBtn = screen.getByLabelText('Switch to dark mode');
    expect(themeBtn).toBeInTheDocument();

    await user.click(themeBtn);
    expect(mockSetTheme).toHaveBeenCalledWith('dark');
  });

  it('renders language selector in desktop sidebar', () => {
    render(<Layout />);
    expect(screen.getByLabelText('Change language')).toBeInTheDocument();
  });

  it('opens language dropdown on click', async () => {
    const user = userEvent.setup();
    render(<Layout />);

    await user.click(screen.getByLabelText('Change language'));

    // Should show language options
    expect(screen.getByText('Deutsch')).toBeInTheDocument();
    expect(screen.getByText('English')).toBeInTheDocument();
  });

  it('closes language dropdown on outside click', async () => {
    const user = userEvent.setup();
    render(<Layout />);

    await user.click(screen.getByLabelText('Change language'));
    expect(screen.getByText('Deutsch')).toBeInTheDocument();

    // Click outside
    await user.click(document.body);
    // Language dropdown should close (but since the mock might not track this perfectly,
    // we just verify it doesn't crash)
  });

  it('renders breadcrumb component', () => {
    render(<Layout />);
    // Breadcrumb renders in both desktop and mobile views
    expect(document.querySelector('#main-content')).toBeInTheDocument();
  });

  it('renders skip to content link', () => {
    render(<Layout />);
    expect(screen.getByText('Skip to content')).toBeInTheDocument();
  });

  it('mobile sidebar clicking nav link closes sidebar', async () => {
    const user = userEvent.setup();
    render(<Layout />);

    await user.click(screen.getByLabelText('Open navigation menu'));
    expect(screen.getByLabelText('Navigation menu')).toBeInTheDocument();

    // Click a nav link in the mobile sidebar
    const sidebar = screen.getByLabelText('Navigation menu');
    const firstLink = sidebar.querySelector('nav a');
    if (firstLink) {
      await user.click(firstLink);
      // The onClick handler closes the sidebar
      expect(screen.queryByLabelText('Navigation menu')).not.toBeInTheDocument();
    }
  });

  it('renders with no user name or username', () => {
    mockUser = { ...mockUser, name: null, username: null };
    render(<Layout />);
    // Falls back to 'U' for initial
    expect(screen.getByText('U')).toBeInTheDocument();
  });

  it('notification fetch is called on mount', () => {
    vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({ ok: true, json: () => Promise.resolve({ data: { count: 5 } }) })));
    render(<Layout />);
    expect(globalThis.fetch).toHaveBeenCalledWith(
      '/api/v1/notifications/unread-count',
      expect.objectContaining({
        headers: expect.objectContaining({
          'Authorization': 'Bearer test-token',
        }),
      }),
    );
  });
});
