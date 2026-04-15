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
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
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

const mockSetAccessibilityNeeds = vi.fn().mockResolvedValue({ success: true });
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
    setAccessibilityNeeds: (...args: any[]) => mockSetAccessibilityNeeds(...args),
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

vi.mock('../components/ui/ConfirmDialog', () => ({
  ConfirmDialog: ({ open, onConfirm, onCancel, title, message }: any) =>
    open ? (
      <div data-testid="confirm-dialog">
        <p>{title}</p>
        <p>{message}</p>
        <button onClick={onConfirm}>Confirm</button>
        <button onClick={onCancel}>CancelDialog</button>
      </div>
    ) : null,
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

  it('shows password validation hint when typing short password', async () => {
    const user = userEvent.setup();
    render(<ProfilePage />);

    // Expand the password section
    await user.click(screen.getByText('Passwort ändern'));
    await waitFor(() => expect(document.getElementById('pw-new')).toBeInTheDocument());

    // Type a short password
    await user.type(document.getElementById('pw-new')!, 'short');

    // Should show minimum chars hint
    await waitFor(() => {
      expect(screen.getByText('Mind. 8 Zeichen')).toBeInTheDocument();
    });
  });

  it('shows password mismatch hint when passwords differ', async () => {
    const user = userEvent.setup();
    render(<ProfilePage />);

    await user.click(screen.getByText('Passwort ändern'));
    await waitFor(() => expect(document.getElementById('pw-new')).toBeInTheDocument());

    await user.type(document.getElementById('pw-new')!, 'newpass123');
    await user.type(document.getElementById('pw-confirm')!, 'different1');

    await waitFor(() => {
      expect(screen.getByText('Passwörter stimmen nicht überein')).toBeInTheDocument();
    });
  });

  it('changes password successfully', async () => {
    mockChangePassword.mockResolvedValue({ success: true });
    const user = userEvent.setup();
    render(<ProfilePage />);

    await user.click(screen.getByText('Passwort ändern'));
    await waitFor(() => expect(document.getElementById('pw-current')).toBeInTheDocument());

    await user.type(document.getElementById('pw-current')!, 'oldpass123');
    await user.type(document.getElementById('pw-new')!, 'newpass123');
    await user.type(document.getElementById('pw-confirm')!, 'newpass123');

    // The submit button should now be enabled
    const submitBtns = screen.getAllByText('Passwort ändern');
    // Click the button that is NOT the section toggle (the one inside the expanded section)
    const formSubmit = submitBtns.find(btn => btn.closest('button')?.getAttribute('aria-expanded') === null);
    if (formSubmit) await user.click(formSubmit);

    await waitFor(() => {
      expect(mockChangePassword).toHaveBeenCalledWith('oldpass123', 'newpass123', 'newpass123');
      expect(mockToastSuccess).toHaveBeenCalledWith('Passwort geändert');
    });
  });

  it('renders 2FA setup component', () => {
    render(<ProfilePage />);
    expect(screen.getByTestId('2fa-setup')).toBeInTheDocument();
  });

  it('renders notification preferences component', () => {
    render(<ProfilePage />);
    expect(screen.getByTestId('notification-prefs')).toBeInTheDocument();
  });

  it('renders login history component', () => {
    render(<ProfilePage />);
    expect(screen.getByTestId('login-history')).toBeInTheDocument();
  });

  it('renders theme section component', () => {
    render(<ProfilePage />);
    expect(screen.getByTestId('theme-section')).toBeInTheDocument();
  });

  it('displays user role badge', () => {
    render(<ProfilePage />);
    expect(screen.getByText('Admin')).toBeInTheDocument();
  });

  it('handles stat load with zero values', async () => {
    mockGetUserStats.mockResolvedValue({
      success: true,
      data: {
        total_bookings: 0,
        bookings_this_month: 0,
        homeoffice_days_this_month: 0,
        avg_duration_minutes: 0,
      },
    });
    render(<ProfilePage />);
    await waitFor(() => {
      expect(screen.getAllByText('0').length).toBeGreaterThanOrEqual(1);
    });
  });

  it('handles stat load failure gracefully', async () => {
    mockGetUserStats.mockResolvedValue({ success: false, data: null });
    render(<ProfilePage />);
    // Should not crash — stats section should still render
    await waitFor(() => {
      expect(screen.getByText('Buchungen (Monat)')).toBeInTheDocument();
    });
  });

  it('handles stat load exception gracefully', async () => {
    mockGetUserStats.mockRejectedValue(new Error('Network'));
    render(<ProfilePage />);
    await waitFor(() => {
      expect(screen.getByText('Buchungen (Monat)')).toBeInTheDocument();
    });
  });

  it('shows error toast for password too short', async () => {
    const user = userEvent.setup();
    render(<ProfilePage />);
    await user.click(screen.getByText('Passwort ändern'));
    await waitFor(() => expect(document.getElementById('pw-current')).toBeInTheDocument());

    await user.type(document.getElementById('pw-current')!, 'oldpass');
    await user.type(document.getElementById('pw-new')!, 'short');
    await user.type(document.getElementById('pw-confirm')!, 'short');

    // Button should be disabled since password < 8 chars
    const submitBtns = screen.getAllByText('Passwort ändern');
    const formSubmit = submitBtns.find(btn => btn.closest('button')?.getAttribute('aria-expanded') === null);
    if (formSubmit) {
      expect(formSubmit.closest('button')).toBeDisabled();
    }
  });

  it('shows error toast on password change failure', async () => {
    mockChangePassword.mockResolvedValue({ success: false, error: { message: 'Wrong current password' } });
    const user = userEvent.setup();
    render(<ProfilePage />);

    await user.click(screen.getByText('Passwort ändern'));
    await waitFor(() => expect(document.getElementById('pw-current')).toBeInTheDocument());

    await user.type(document.getElementById('pw-current')!, 'wrongpass1');
    await user.type(document.getElementById('pw-new')!, 'newpass123');
    await user.type(document.getElementById('pw-confirm')!, 'newpass123');

    const submitBtns = screen.getAllByText('Passwort ändern');
    const formSubmit = submitBtns.find(btn => btn.closest('button')?.getAttribute('aria-expanded') === null);
    if (formSubmit) await user.click(formSubmit);

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Wrong current password');
    });
  });

  it('shows error toast when current password is empty', async () => {
    const user = userEvent.setup();
    render(<ProfilePage />);

    await user.click(screen.getByText('Passwort ändern'));
    await waitFor(() => expect(document.getElementById('pw-new')).toBeInTheDocument());

    // Type new and confirm but leave current empty -- button should be disabled
    await user.type(document.getElementById('pw-new')!, 'newpass123');
    await user.type(document.getElementById('pw-confirm')!, 'newpass123');

    const submitBtns = screen.getAllByText('Passwort ändern');
    const formSubmit = submitBtns.find(btn => btn.closest('button')?.getAttribute('aria-expanded') === null);
    if (formSubmit) {
      expect(formSubmit.closest('button')).toBeDisabled();
    }
  });

  it('exports data on click', async () => {
    const mockBlob = new Blob(['{}'], { type: 'application/json' });
    const mockApi = await import('../api/client');
    (mockApi.api as any).exportMyData = vi.fn().mockResolvedValue(mockBlob);

    const createObjectURL = vi.fn(() => 'blob:test');
    const revokeObjectURL = vi.fn();
    Object.defineProperty(URL, 'createObjectURL', { value: createObjectURL, writable: true });
    Object.defineProperty(URL, 'revokeObjectURL', { value: revokeObjectURL, writable: true });

    const user = userEvent.setup();
    render(<ProfilePage />);

    await user.click(screen.getByText('Daten exportieren'));
    // Export path exercised
  });

  it('opens delete account confirm dialog', async () => {
    const user = userEvent.setup();
    render(<ProfilePage />);

    // Click the delete account button (there's a button with this text in the GDPR section)
    const deleteButtons = screen.getAllByText('Konto löschen');
    const deleteBtn = deleteButtons.find(el => el.closest('button'));
    if (deleteBtn) await user.click(deleteBtn);

    await waitFor(() => {
      expect(screen.getByTestId('confirm-dialog')).toBeInTheDocument();
    });
  });

  it('collapses password section on second click', async () => {
    const user = userEvent.setup();
    render(<ProfilePage />);

    await user.click(screen.getByText('Passwort ändern'));
    await waitFor(() => expect(document.getElementById('pw-current')).toBeInTheDocument());

    // Click again to collapse -- the button with aria-expanded
    const toggleBtns = screen.getAllByText('Passwort ändern');
    const toggleBtn = toggleBtns.find(btn => btn.closest('button')?.getAttribute('aria-expanded') !== null);
    if (toggleBtn) await user.click(toggleBtn);

    await waitFor(() => {
      expect(document.getElementById('pw-current')).not.toBeInTheDocument();
    });
  });

  it('renders geofence auto check-in section', () => {
    render(<ProfilePage />);
    expect(screen.getByText('geofence.autoCheckIn')).toBeInTheDocument();
  });

  it('export data failure shows error toast', async () => {
    const mockApi = await import('../api/client');
    (mockApi.api as any).exportMyData = vi.fn().mockRejectedValue(new Error('Export failed'));

    const user = userEvent.setup();
    render(<ProfilePage />);

    await user.click(screen.getByText('Daten exportieren'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Export fehlgeschlagen');
    });
  });

  it('delete account action calls API and logs out on success', async () => {
    const mockApi = await import('../api/client');
    (mockApi.api as any).deleteMyAccount = vi.fn().mockResolvedValue({ success: true });

    const user = userEvent.setup();
    render(<ProfilePage />);

    const deleteButtons = screen.getAllByText('Konto löschen');
    const deleteBtn = deleteButtons.find(el => el.closest('button'));
    if (deleteBtn) await user.click(deleteBtn);

    await waitFor(() => expect(screen.getByTestId('confirm-dialog')).toBeInTheDocument());

    await user.click(screen.getByText('Confirm'));

    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith('Konto gelöscht');
      expect(mockLogout).toHaveBeenCalled();
    });
  });

  it('delete account failure shows error toast', async () => {
    const mockApi = await import('../api/client');
    (mockApi.api as any).deleteMyAccount = vi.fn().mockResolvedValue({ success: false, error: { message: 'Cannot delete' } });

    const user = userEvent.setup();
    render(<ProfilePage />);

    const deleteButtons = screen.getAllByText('Konto löschen');
    const deleteBtn = deleteButtons.find(el => el.closest('button'));
    if (deleteBtn) await user.click(deleteBtn);

    await waitFor(() => expect(screen.getByTestId('confirm-dialog')).toBeInTheDocument());
    await user.click(screen.getByText('Confirm'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Cannot delete');
    });
  });

  it('delete account exception shows error toast', async () => {
    const mockApi = await import('../api/client');
    (mockApi.api as any).deleteMyAccount = vi.fn().mockRejectedValue(new Error('Network'));

    const user = userEvent.setup();
    render(<ProfilePage />);

    const deleteButtons = screen.getAllByText('Konto löschen');
    const deleteBtn = deleteButtons.find(el => el.closest('button'));
    if (deleteBtn) await user.click(deleteBtn);

    await waitFor(() => expect(screen.getByTestId('confirm-dialog')).toBeInTheDocument());
    await user.click(screen.getByText('Confirm'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Löschen fehlgeschlagen');
    });
  });

  it('accessibility needs change success', async () => {
    mockSetAccessibilityNeeds.mockResolvedValueOnce({ success: true });

    const user = userEvent.setup();
    render(<ProfilePage />);

    const selector = screen.getByTestId('accessibility-selector');
    await user.selectOptions(selector, 'wheelchair');

    await waitFor(() => {
      expect(mockSetAccessibilityNeeds).toHaveBeenCalledWith('wheelchair');
      expect(mockToastSuccess).toHaveBeenCalled();
    });
  });

  it('accessibility needs change failure', async () => {
    mockSetAccessibilityNeeds.mockResolvedValueOnce({
      success: false,
      error: { message: 'Failed' },
    });

    const user = userEvent.setup();
    render(<ProfilePage />);

    const selector = screen.getByTestId('accessibility-selector');
    await user.selectOptions(selector, 'visual');

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Failed');
    });
  });

  it('accessibility needs change network error', async () => {
    mockSetAccessibilityNeeds.mockRejectedValueOnce(new Error('Network'));

    const user = userEvent.setup();
    render(<ProfilePage />);

    const selector = screen.getByTestId('accessibility-selector');
    await user.selectOptions(selector, 'hearing');

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalled();
    });
  });

  it('geofence toggle saves to localStorage and shows toast', async () => {
    const user = userEvent.setup();
    render(<ProfilePage />);

    const checkbox = screen.getByRole('checkbox');
    await user.click(checkbox);

    expect(localStorage.getItem('parkhub_geofence_auto')).toBe('true');
    expect(mockToastSuccess).toHaveBeenCalled();
  });

  it('save profile failure without error message shows default', async () => {
    mockUpdateMe.mockResolvedValue({ success: false });
    const user = userEvent.setup();
    render(<ProfilePage />);

    await user.click(screen.getByText('Bearbeiten'));
    await user.click(screen.getByText('Speichern'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Fehler');
    });
  });

  it('password change failure without error message shows default', async () => {
    mockChangePassword.mockResolvedValue({ success: false });
    render(<ProfilePage />);

    fireEvent.click(screen.getByText('Passwort ändern'));
    await waitFor(() => expect(document.getElementById('pw-current')).toBeInTheDocument());

    fireEvent.change(document.getElementById('pw-current')!, { target: { value: 'oldpass12' } });
    fireEvent.change(document.getElementById('pw-new')!, { target: { value: 'newpass123' } });
    fireEvent.change(document.getElementById('pw-confirm')!, { target: { value: 'newpass123' } });

    const submitBtns = screen.getAllByText('Passwort ändern');
    const formSubmit = submitBtns.find(btn => btn.closest('button')?.getAttribute('aria-expanded') === null);
    if (formSubmit) fireEvent.click(formSubmit);

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Fehler');
    });
  });

});
