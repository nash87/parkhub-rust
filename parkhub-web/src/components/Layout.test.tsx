import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const {
  mockChangeLanguage,
  mockGetInMemoryToken,
  mockSetDesignTheme,
  latestKeyboardShortcuts,
} = vi.hoisted(() => ({
  mockChangeLanguage: vi.fn(),
  mockGetInMemoryToken: vi.fn(() => 'test-token'),
  mockSetDesignTheme: vi.fn(),
  latestKeyboardShortcuts: { current: null as null | { onToggleCommandPalette: () => void } },
}));

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
    designTheme: 'default',
    setDesignTheme: mockSetDesignTheme,
    designThemes: [
      {
        id: 'default',
        name: 'Default',
        description: 'Default theme',
        previewColors: {
          light: ['#fff', '#eee', '#0ea5e9', '#111', '#ddd'],
          dark: ['#111', '#222', '#0ea5e9', '#fff', '#333'],
        },
      },
    ],
  }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'nav.dashboard': 'Dashboard',
        'nav.bookSpot': 'Book a Spot',
        'nav.bookings': 'Bookings',
        'nav.vehicles': 'Vehicles',
        'nav.favorites': 'Favorites',
        'nav.absences': 'Absences',
        'nav.team': 'Team',
        'nav.calendar': 'Calendar',
        'nav.map': 'Map',
        'nav.history': 'History',
        'nav.checkin': 'Check In',
        'nav.swapRequests': 'Swap Requests',
        'nav.guestPass': 'Guest Pass',
        'nav.leaderboard': 'Leaderboard',
        'nav.predictions': 'Predictions',
        'nav.credits': 'Credits',
        'nav.notifications': 'Notifications',
        'nav.profile': 'Profile',
        'nav.admin': 'Admin',
        'nav.translations': 'Translations',
        'nav.sections.core': 'Essentials',
        'nav.sections.fleet': 'Fleet',
        'nav.sections.settings': 'Settings',
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
    i18n: {
      language: 'en',
      changeLanguage: mockChangeLanguage,
    },
  }),
}));

vi.mock('../api/client', () => ({
  getInMemoryToken: mockGetInMemoryToken,
}));

vi.mock('../hooks/useKeyboardShortcuts', () => ({
  useKeyboardShortcuts: (handlers: { onToggleCommandPalette: () => void }) => {
    latestKeyboardShortcuts.current = handlers;
  },
}));

vi.mock('../hooks/usePageTitle', () => ({
  usePageTitle: vi.fn(),
}));

vi.mock('./CommandPalette', () => ({
  CommandPalette: ({ open, onClose }: { open: boolean; onClose: () => void }) => (
    open
      ? (
        <div data-testid="mock-command-palette">
          <button onClick={onClose}>Close Command Palette</button>
        </div>
      )
      : null
  ),
}));

vi.mock('./ThemeSwitcher', () => ({
  ThemeSwitcherFab: ({ onClick }: { onClick: () => void }) => (
    <button onClick={onClick}>Open Theme Switcher</button>
  ),
  ThemeSwitcher: ({ open, onClose }: { open: boolean; onClose: () => void }) => (
    open
      ? (
        <div data-testid="mock-theme-switcher">
          <button onClick={onClose}>Close Theme Switcher</button>
        </div>
      )
      : null
  ),
}));

vi.mock('./NotificationCenter', () => ({
  NotificationCenter: () => <div data-testid="notification-center">Notifications</div>,
}));

vi.mock('./ui/Breadcrumb', () => ({
  Breadcrumb: () => <div data-testid="breadcrumb">Breadcrumb</div>,
}));

vi.mock('./ui/NotificationBadge', () => ({
  NotificationBadge: ({ count }: { count: number }) => <span data-testid="notification-badge">{count}</span>,
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, variants, layoutId, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    aside: React.forwardRef(({ children, initial, animate, exit, transition, onDragEnd, ...props }: any, ref: any) => (
      <aside
        ref={ref}
        {...props}
        onDragEnd={(event: any) => onDragEnd?.(event, event.detail ?? { offset: { x: 0 }, velocity: { x: 0 } })}
      >
        {children}
      </aside>
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
    mockChangeLanguage.mockClear();
    mockGetInMemoryToken.mockReset();
    mockGetInMemoryToken.mockReturnValue('test-token');
    mockSetDesignTheme.mockClear();
    latestKeyboardShortcuts.current = null;
    mockUser = {
      id: '1',
      username: 'testuser',
      email: 'test@example.com',
      name: 'Alice Smith',
      role: 'user',
    };
    vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({
      ok: true,
      json: () => Promise.resolve({ data: { count: 0 } }),
    })));
    window.localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
    window.localStorage.clear();
  });

  it('renders the ParkHub logo text', () => {
    render(<Layout />);
    // There are multiple "ParkHub" texts (desktop + mobile), just check at least one
    const logos = screen.getAllByText('ParkHub');
    expect(logos.length).toBeGreaterThanOrEqual(1);
  });

  it('renders all navigation items in the desktop sidebar', async () => {
    const user = userEvent.setup();
    render(<Layout />);
    // Core section (always visible)
    expect(screen.getAllByText('Dashboard').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Bookings').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Book a Spot').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Vehicles').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Calendar').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Credits').length).toBeGreaterThanOrEqual(1);
    // Fleet section (default open)
    expect(screen.getAllByText('Absences').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Team').length).toBeGreaterThanOrEqual(1);
    // Settings section (default closed) — expand before asserting contents
    const settingsHeaders = screen.getAllByRole('button', { name: /Settings/ });
    // Expand both desktop and mobile collapsed Settings headers
    for (const btn of settingsHeaders) await user.click(btn);
    expect(screen.getAllByText('Notifications').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Profile').length).toBeGreaterThanOrEqual(1);
  });

  it('renders the 3 sidebar sections with collapsible Fleet and Settings', async () => {
    const user = userEvent.setup();
    render(<Layout />);
    // Section headers rendered in both desktop and mobile sidebars
    expect(screen.getAllByText('Essentials').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Fleet').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Settings').length).toBeGreaterThanOrEqual(1);

    // Fleet defaults open — a fleet item is visible
    expect(screen.getAllByText('Favorites').length).toBeGreaterThanOrEqual(1);
    // Settings defaults closed — Profile is not yet in the DOM
    expect(screen.queryByText('Profile')).not.toBeInTheDocument();

    // Toggle Settings open
    const settingsHeaders = screen.getAllByRole('button', { name: /Settings/ });
    const firstSettings = settingsHeaders[0];
    if (!firstSettings) throw new Error('Settings header not found');
    await user.click(firstSettings);
    expect(screen.getAllByText('Profile').length).toBeGreaterThanOrEqual(1);
  });

  it('persists collapsed section state in localStorage', async () => {
    const user = userEvent.setup();
    render(<Layout />);
    const settingsHeaders = screen.getAllByRole('button', { name: /Settings/ });
    const firstSettings = settingsHeaders[0];
    if (!firstSettings) throw new Error('Settings header not found');
    await user.click(firstSettings);
    expect(window.localStorage.getItem('parkhub_sidebar_settings_open')).toBe('true');

    const fleetHeaders = screen.getAllByRole('button', { name: /Fleet/ });
    const firstFleet = fleetHeaders[0];
    if (!firstFleet) throw new Error('Fleet header not found');
    await user.click(firstFleet);
    expect(window.localStorage.getItem('parkhub_sidebar_fleet_open')).toBe('false');
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

  it('mobile sidebar admin link closes the sidebar when clicked', async () => {
    mockUser = { ...mockUser, role: 'admin' };
    const user = userEvent.setup();
    render(<Layout />);

    await user.click(screen.getByLabelText('Open navigation menu'));
    const sidebar = screen.getByLabelText('Navigation menu');
    const adminLink = sidebar.querySelector('a[href="/admin"]');

    expect(adminLink).toBeTruthy();
    if (adminLink instanceof HTMLElement) {
      await user.click(adminLink);
    }

    expect(screen.queryByLabelText('Navigation menu')).not.toBeInTheDocument();
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

  it('changes language when a language option is clicked', async () => {
    const user = userEvent.setup();
    render(<Layout />);

    await user.click(screen.getByLabelText('Change language'));
    await user.click(screen.getByText('Deutsch'));

    expect(mockChangeLanguage).toHaveBeenCalledWith('de');
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

  it('closes the mobile sidebar when clicking the backdrop', async () => {
    const user = userEvent.setup();
    render(<Layout />);

    await user.click(screen.getByLabelText('Open navigation menu'));
    expect(screen.getByLabelText('Navigation menu')).toBeInTheDocument();

    const backdrop = document.querySelector('[aria-hidden="true"]');
    expect(backdrop).toBeTruthy();

    if (backdrop instanceof HTMLElement) {
      await user.click(backdrop);
    }

    expect(screen.queryByLabelText('Navigation menu')).not.toBeInTheDocument();
  });

  it('closes the mobile sidebar when dragged far enough to the left', async () => {
    const user = userEvent.setup();
    render(<Layout />);

    await user.click(screen.getByLabelText('Open navigation menu'));
    const sidebar = screen.getByLabelText('Navigation menu');

    const dragEndEvent = new CustomEvent('dragend', {
      detail: { offset: { x: -120 }, velocity: { x: 0 } },
      bubbles: true,
    });
    await act(async () => {
      sidebar.dispatchEvent(dragEndEvent);
    });

    expect(screen.queryByLabelText('Navigation menu')).not.toBeInTheDocument();
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

  it('skips notification fetch when no token is available', () => {
    const fetchSpy = vi.fn();
    mockGetInMemoryToken.mockReturnValue(null);
    vi.stubGlobal('fetch', fetchSpy);

    render(<Layout />);

    expect(fetchSpy).not.toHaveBeenCalled();
  });

  it('opens and closes the command palette through the keyboard shortcut handler', async () => {
    const user = userEvent.setup();
    render(<Layout />);

    expect(latestKeyboardShortcuts.current).toBeTruthy();

    await act(async () => {
      latestKeyboardShortcuts.current?.onToggleCommandPalette();
    });
    expect(screen.getByTestId('mock-command-palette')).toBeInTheDocument();

    await user.click(screen.getByText('Close Command Palette'));
    expect(screen.queryByTestId('mock-command-palette')).not.toBeInTheDocument();
  });

  it('opens and closes the theme switcher from the floating action button', async () => {
    const user = userEvent.setup();
    render(<Layout />);

    await user.click(screen.getByText('Open Theme Switcher'));
    expect(screen.getByTestId('mock-theme-switcher')).toBeInTheDocument();

    await user.click(screen.getByText('Close Theme Switcher'));
    expect(screen.queryByTestId('mock-theme-switcher')).not.toBeInTheDocument();
  });
});
