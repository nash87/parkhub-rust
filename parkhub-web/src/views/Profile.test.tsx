import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// ── matchMedia mock — must run before any module-level code in ThemeContext ──
vi.hoisted(() => {
  Object.defineProperty(globalThis.window ?? globalThis, 'matchMedia', {
    writable: true,
    configurable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }),
  });
});

import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockLogout = vi.fn();
const mockGetUserStats = vi.fn();
const mockUpdateMe = vi.fn();
const mockChangePassword = vi.fn();
const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('../components/ProfileThemeSection', () => ({
  ProfileThemeSection: () => <div data-testid="theme-section">Theme Section</div>,
}));

vi.mock('../components/TwoFactorSetup', () => ({
  TwoFactorSetupComponent: () => <div data-testid="2fa-setup">2FA Setup</div>,
}));

vi.mock('../components/NotificationPreferences', () => ({
  NotificationPreferencesComponent: () => <div data-testid="notification-prefs">Notification Preferences</div>,
}));

vi.mock('../components/LoginHistory', () => ({
  LoginHistoryComponent: () => <div data-testid="login-history">Login History</div>,
}));

vi.mock('../context/AuthContext', () => ({
  useAuth: () => ({
    user: {
      id: 'u-1',
      username: 'jdoe',
      name: 'John Doe',
      email: 'john@example.com',
      role: 'admin',
      credits_balance: 5,
      credits_monthly_quota: 10,
    },
    logout: mockLogout,
  }),
}));

vi.mock('../api/client', () => ({
  api: {
    getUserStats: (...args: any[]) => mockGetUserStats(...args),
    updateMe: (...args: any[]) => mockUpdateMe(...args),
    changePassword: (...args: any[]) => mockChangePassword(...args),
    getNotificationPreferences: vi.fn().mockResolvedValue({ success: true, data: { email_bookings: true, email_reminders: true, push_bookings: false, push_reminders: false } }),
    updateNotificationPreferences: vi.fn().mockResolvedValue({ success: true }),
    getLoginHistory: vi.fn().mockResolvedValue({ success: true, data: [] }),
    getSessions: vi.fn().mockResolvedValue({ success: true, data: [] }),
    revokeSession: vi.fn().mockResolvedValue({ success: true }),
    get2FAStatus: vi.fn().mockResolvedValue({ success: true, data: { enabled: false } }),
    setup2FA: vi.fn().mockResolvedValue({ success: true, data: { secret: 'ABCD', qr_url: 'otpauth://test' } }),
    verify2FA: vi.fn().mockResolvedValue({ success: true }),
    disable2FA: vi.fn().mockResolvedValue({ success: true }),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'profile.title': 'Profil',
        'profile.subtitle': 'Persönliche Daten verwalten',
        'profile.name': 'Name',
        'profile.email': 'E-Mail',
        'profile.updated': 'Profil aktualisiert',
        'profile.changePassword': 'Passwort ändern',
        'profile.currentPassword': 'Aktuelles Passwort',
        'profile.newPassword': 'Neues Passwort',
        'profile.confirmPassword': 'Passwort bestätigen',
        'profile.changePasswordBtn': 'Passwort ändern',
        'profile.passwordTooShort': 'Mind. 8 Zeichen',
        'profile.passwordsMismatch': 'Passwörter stimmen nicht überein',
        'profile.currentPasswordRequired': 'Aktuelles Passwort eingeben',
        'profile.passwordChanged': 'Passwort geändert',
        'profile.bookingsThisMonth': 'Buchungen (Monat)',
        'profile.homeOfficeDays': 'Homeoffice-Tage',
        'profile.avgDuration': 'Durchschn. Dauer',
        'common.edit': 'Bearbeiten',
        'common.save': 'Speichern',
        'common.cancel': 'Abbrechen',
        'gdpr.rights': 'Ihre Rechte gemäß DSGVO Art. 15, 17 und 20.',
        'gdpr.dataExport': 'Daten exportieren',
        'gdpr.dataExportDesc': 'Art. 20 Datenportabilität',
        'gdpr.deleteAccount': 'Konto löschen',
        'gdpr.deleteAccountDesc': 'Alle Daten unwiderruflich löschen',
        'gdpr.exported': 'Daten exportiert',
        'gdpr.deleted': 'Konto gelöscht',
        'gdpr.exportFailed': 'Export fehlgeschlagen',
        'gdpr.deleteFailed': 'Löschen fehlgeschlagen',
        'profile.roles.user': 'Benutzer',
        'profile.roles.admin': 'Admin',
        'profile.roles.superadmin': 'Super-Admin',
        'profile.minChars': 'Mind. 8 Zeichen',
        'profile.passwordsNoMatch': 'Passwörter stimmen nicht überein',
        'common.error': 'Fehler',
      };
      return map[key] || fallback || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, variants, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  UserCircle: (props: any) => <span data-testid="icon-user" {...props} />,
  Envelope: (props: any) => <span data-testid="icon-envelope" {...props} />,
  PencilSimple: (props: any) => <span data-testid="icon-pencil" {...props} />,
  FloppyDisk: (props: any) => <span data-testid="icon-save" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  Lock: (props: any) => <span data-testid="icon-lock" {...props} />,
  CalendarCheck: (props: any) => <span data-testid="icon-calendar" {...props} />,
  House: (props: any) => <span data-testid="icon-house" {...props} />,
  ChartBar: (props: any) => <span data-testid="icon-chart" {...props} />,
  DownloadSimple: (props: any) => <span data-testid="icon-download" {...props} />,
  Trash: (props: any) => <span data-testid="icon-trash" {...props} />,
  CaretDown: (props: any) => <span data-testid="icon-caret-down" {...props} />,
  CaretUp: (props: any) => <span data-testid="icon-caret-up" {...props} />,
  Shield: (props: any) => <span data-testid="icon-shield" {...props} />,
  Warning: (props: any) => <span data-testid="icon-warning" {...props} />,
  ShieldCheck: (props: any) => <span data-testid="icon-shield-check" {...props} />,
  Bell: (props: any) => <span data-testid="icon-bell" {...props} />,
  EnvelopeSimple: (props: any) => <span data-testid="icon-envelope-simple" {...props} />,
  DeviceMobile: (props: any) => <span data-testid="icon-device-mobile" {...props} />,
  ClockCounterClockwise: (props: any) => <span data-testid="icon-clock" {...props} />,
  Desktop: (props: any) => <span data-testid="icon-desktop" {...props} />,
  Globe: (props: any) => <span data-testid="icon-globe" {...props} />,
  ShieldWarning: (props: any) => <span data-testid="icon-shield-warning" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  Palette: (props: any) => <span data-testid="icon-palette" {...props} />,
  MapPin: (props: any) => <span data-testid="icon-map-pin" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: {
    success: (...args: any[]) => mockToastSuccess(...args),
    error: (...args: any[]) => mockToastError(...args),
  },
}));

vi.mock('../constants/animations', () => ({
  staggerSlow: { hidden: {}, show: {} },
  fadeUp: { hidden: {}, show: {} },
}));

import { ProfilePage } from './Profile';

describe('ProfilePage', () => {
  beforeEach(() => {
    mockGetUserStats.mockResolvedValue({
      success: true,
      data: {
        total_bookings: 42,
        bookings_this_month: 5,
        homeoffice_days_this_month: 8,
        avg_duration_minutes: 120,
      },
    });
    mockUpdateMe.mockClear();
    mockChangePassword.mockClear();
    mockToastSuccess.mockClear();
    mockToastError.mockClear();
    mockLogout.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders profile heading and subtitle', () => {
    render(<ProfilePage />);
    expect(screen.getByText('Profil')).toBeInTheDocument();
    expect(screen.getByText('Persönliche Daten verwalten')).toBeInTheDocument();
  });

  it('renders user name, role, and email in view mode', () => {
    render(<ProfilePage />);
    expect(screen.getByText('John Doe')).toBeInTheDocument();
    expect(screen.getByText('Admin')).toBeInTheDocument();
    expect(screen.getByText('john@example.com')).toBeInTheDocument();
    expect(screen.getByText('@jdoe')).toBeInTheDocument();
  });

  it('renders user initials', () => {
    render(<ProfilePage />);
    expect(screen.getByText('JD')).toBeInTheDocument();
  });

  it('renders edit button', () => {
    render(<ProfilePage />);
    expect(screen.getByText('Bearbeiten')).toBeInTheDocument();
  });

  it('enters edit mode on click and shows form fields', async () => {
    const user = userEvent.setup();
    render(<ProfilePage />);

    await user.click(screen.getByText('Bearbeiten'));

    expect(screen.getByDisplayValue('John Doe')).toBeInTheDocument();
    expect(screen.getByDisplayValue('john@example.com')).toBeInTheDocument();
    expect(screen.getByText('Speichern')).toBeInTheDocument();
    expect(screen.getByText('Abbrechen')).toBeInTheDocument();
  });

  it('cancels editing and returns to view mode', async () => {
    const user = userEvent.setup();
    render(<ProfilePage />);

    await user.click(screen.getByText('Bearbeiten'));
    expect(screen.getByText('Abbrechen')).toBeInTheDocument();

    await user.click(screen.getByText('Abbrechen'));
    expect(screen.getByText('John Doe')).toBeInTheDocument();
    expect(screen.getByText('Bearbeiten')).toBeInTheDocument();
  });

  it('saves profile and shows toast on success', async () => {
    mockUpdateMe.mockResolvedValue({ success: true });
    const user = userEvent.setup();
    render(<ProfilePage />);

    await user.click(screen.getByText('Bearbeiten'));

    const nameInput = screen.getByDisplayValue('John Doe');
    await user.clear(nameInput);
    await user.type(nameInput, 'Jane Doe');
    await user.click(screen.getByText('Speichern'));

    await waitFor(() => {
      expect(mockUpdateMe).toHaveBeenCalledWith({ name: 'Jane Doe', email: 'john@example.com' });
    });

    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith('Profil aktualisiert');
    });
  });

  it('shows error toast on failed save', async () => {
    mockUpdateMe.mockResolvedValue({ success: false, error: { message: 'Update failed' } });
    const user = userEvent.setup();
    render(<ProfilePage />);

    await user.click(screen.getByText('Bearbeiten'));
    await user.click(screen.getByText('Speichern'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Update failed');
    });
  });

  it('renders password change section', () => {
    render(<ProfilePage />);
    expect(screen.getByText('Passwort ändern')).toBeInTheDocument();
  });

  it('expands password change form on click', async () => {
    const user = userEvent.setup();
    render(<ProfilePage />);

    await user.click(screen.getByText('Passwort ändern'));

    expect(screen.getByText('Aktuelles Passwort')).toBeInTheDocument();
    expect(screen.getByText('Neues Passwort')).toBeInTheDocument();
    expect(screen.getByText('Passwort bestätigen')).toBeInTheDocument();
  });

  it('renders stat cards', async () => {
    render(<ProfilePage />);

    await waitFor(() => {
      expect(screen.getByText('Buchungen (Monat)')).toBeInTheDocument();
    });
    expect(screen.getByText('Homeoffice-Tage')).toBeInTheDocument();
    expect(screen.getByText('Durchschn. Dauer')).toBeInTheDocument();
  });

  it('renders GDPR section', () => {
    render(<ProfilePage />);
    expect(screen.getByText('DSGVO / GDPR')).toBeInTheDocument();
    expect(screen.getByText('Daten exportieren')).toBeInTheDocument();
    expect(screen.getByText('Konto löschen')).toBeInTheDocument();
  });

  it('renders accessibility needs section', () => {
    render(<ProfilePage />);
    expect(screen.getByTestId('accessibility-section')).toBeInTheDocument();
    expect(screen.getByTestId('accessibility-selector')).toBeInTheDocument();
  });

  it('accessibility selector has correct options', () => {
    render(<ProfilePage />);
    const selector = screen.getByTestId('accessibility-selector') as HTMLSelectElement;
    const options = Array.from(selector.options).map(o => o.value);
    expect(options).toContain('none');
    expect(options).toContain('wheelchair');
    expect(options).toContain('reduced_mobility');
    expect(options).toContain('visual');
    expect(options).toContain('hearing');
  });
});
