import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Hoisted API mocks ──
const { mockGetNotificationPreferences, mockUpdateNotificationPreferences } = vi.hoisted(() => ({
  mockGetNotificationPreferences: vi.fn(),
  mockUpdateNotificationPreferences: vi.fn(),
}));

vi.mock('../api/client', () => ({
  api: {
    getNotificationPreferences: mockGetNotificationPreferences,
    updateNotificationPreferences: mockUpdateNotificationPreferences,
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string | Record<string, string>) => {
      if (typeof fallback === 'string') return fallback;
      return key;
    },
  }),
}));

const { mockToast } = vi.hoisted(() => ({
  mockToast: { success: vi.fn(), error: vi.fn() },
}));

vi.mock('react-hot-toast', () => ({
  default: mockToast,
}));

vi.mock('@phosphor-icons/react', () => ({
  Bell: (props: any) => <span data-testid="icon-Bell" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-SpinnerGap" {...props} />,
  FloppyDisk: (props: any) => <span data-testid="icon-FloppyDisk" {...props} />,
  EnvelopeSimple: (props: any) => <span data-testid="icon-EnvelopeSimple" {...props} />,
  DeviceMobile: (props: any) => <span data-testid="icon-DeviceMobile" {...props} />,
  ChatCircleDots: (props: any) => <span data-testid="icon-ChatCircleDots" {...props} />,
  Phone: (props: any) => <span data-testid="icon-Phone" {...props} />,
}));

import { NotificationPreferencesComponent } from './NotificationPreferences';

const defaultPrefs = {
  email_booking_confirm: true,
  email_booking_reminder: true,
  email_swap_request: true,
  push_enabled: true,
  sms_booking_confirm: false,
  sms_booking_reminder: false,
  sms_booking_cancelled: false,
  whatsapp_booking_confirm: false,
  whatsapp_booking_reminder: false,
  whatsapp_booking_cancelled: false,
  phone_number: undefined,
};

describe('NotificationPreferencesComponent', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading state initially', () => {
    mockGetNotificationPreferences.mockReturnValue(new Promise(() => {}));
    render(<NotificationPreferencesComponent />);
    expect(screen.getByText('Loading preferences...')).toBeInTheDocument();
  });

  it('renders all notification sections after loading', async () => {
    mockGetNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });

    render(<NotificationPreferencesComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading preferences...')).not.toBeInTheDocument();
    });

    expect(screen.getByText('Notification Preferences')).toBeInTheDocument();
    expect(screen.getByText('Email Notifications')).toBeInTheDocument();
    expect(screen.getByText('Push Notifications')).toBeInTheDocument();
    expect(screen.getByText('Phone Number')).toBeInTheDocument();
    expect(screen.getByText('SMS Notifications')).toBeInTheDocument();
    expect(screen.getByText('WhatsApp Notifications')).toBeInTheDocument();
  });

  it('renders toggle labels for email preferences', async () => {
    mockGetNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });

    render(<NotificationPreferencesComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading preferences...')).not.toBeInTheDocument();
    });

    // Multiple sections share "Booking confirmations" label (email, SMS, WhatsApp)
    const confirmLabels = screen.getAllByText('Booking confirmations');
    expect(confirmLabels.length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('Swap request notifications')).toBeInTheDocument();
  });

  it('shows Coming soon badges for SMS and WhatsApp', async () => {
    mockGetNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });

    render(<NotificationPreferencesComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading preferences...')).not.toBeInTheDocument();
    });

    const badges = screen.getAllByText('Coming soon');
    expect(badges).toHaveLength(2); // SMS and WhatsApp
  });

  it('does not show save button initially (no changes)', async () => {
    mockGetNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });

    render(<NotificationPreferencesComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading preferences...')).not.toBeInTheDocument();
    });

    expect(screen.queryByText('Save Preferences')).not.toBeInTheDocument();
  });

  it('shows save button after toggling a preference', async () => {
    const user = userEvent.setup();
    mockGetNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });

    render(<NotificationPreferencesComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading preferences...')).not.toBeInTheDocument();
    });

    // Click the first toggle switch (email_booking_confirm)
    const switches = screen.getAllByRole('switch');
    await user.click(switches[0]);

    expect(screen.getByText('Save Preferences')).toBeInTheDocument();
  });

  it('saves preferences successfully', async () => {
    const user = userEvent.setup();
    mockGetNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });
    mockUpdateNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });

    render(<NotificationPreferencesComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading preferences...')).not.toBeInTheDocument();
    });

    // Toggle a switch to make form dirty
    const switches = screen.getAllByRole('switch');
    await user.click(switches[0]);

    await user.click(screen.getByText('Save Preferences'));

    await waitFor(() => {
      expect(mockUpdateNotificationPreferences).toHaveBeenCalled();
    });
    expect(mockToast.success).toHaveBeenCalledWith('Notification preferences saved');
  });

  it('shows error toast on save failure', async () => {
    const user = userEvent.setup();
    mockGetNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });
    mockUpdateNotificationPreferences.mockResolvedValue({ success: false, error: { message: 'Server error' } });

    render(<NotificationPreferencesComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading preferences...')).not.toBeInTheDocument();
    });

    const switches = screen.getAllByRole('switch');
    await user.click(switches[0]);
    await user.click(screen.getByText('Save Preferences'));

    await waitFor(() => {
      expect(mockToast.error).toHaveBeenCalledWith('Server error');
    });
  });

  it('hides save button after successful save', async () => {
    const user = userEvent.setup();
    mockGetNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });
    mockUpdateNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });

    render(<NotificationPreferencesComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading preferences...')).not.toBeInTheDocument();
    });

    const switches = screen.getAllByRole('switch');
    await user.click(switches[0]);

    expect(screen.getByText('Save Preferences')).toBeInTheDocument();

    await user.click(screen.getByText('Save Preferences'));

    await waitFor(() => {
      expect(screen.queryByText('Save Preferences')).not.toBeInTheDocument();
    });
  });

  it('handles API failure on load gracefully', async () => {
    mockGetNotificationPreferences.mockRejectedValue(new Error('Network error'));

    render(<NotificationPreferencesComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading preferences...')).not.toBeInTheDocument();
    });

    // Should render with default state
    expect(screen.getByText('Notification Preferences')).toBeInTheDocument();
  });

  it('phone number input is editable', async () => {
    const user = userEvent.setup();
    mockGetNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });

    render(<NotificationPreferencesComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading preferences...')).not.toBeInTheDocument();
    });

    const phoneInput = screen.getByLabelText('Phone Number');
    await user.type(phoneInput, '+49123456789');
    expect(phoneInput).toHaveValue('+49123456789');

    // Phone input change marks form as dirty
    expect(screen.getByText('Save Preferences')).toBeInTheDocument();
  });

  it('renders push notification toggle', async () => {
    mockGetNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });

    render(<NotificationPreferencesComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading preferences...')).not.toBeInTheDocument();
    });

    expect(screen.getByText('Enable push notifications')).toBeInTheDocument();
  });

  it('updates push, sms, and whatsapp toggles before saving', async () => {
    const user = userEvent.setup();
    mockGetNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });
    mockUpdateNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });

    render(<NotificationPreferencesComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading preferences...')).not.toBeInTheDocument();
    });

    const switches = screen.getAllByRole('switch');

    await user.click(switches[3]); // push_enabled
    fireEvent.click(switches[4]); // sms_booking_confirm
    fireEvent.click(switches[5]); // sms_booking_reminder
    fireEvent.click(switches[6]); // sms_booking_cancelled
    fireEvent.click(switches[7]); // whatsapp_booking_confirm
    fireEvent.click(switches[8]); // whatsapp_booking_reminder
    fireEvent.click(switches[9]); // whatsapp_booking_cancelled

    await user.click(screen.getByText('Save Preferences'));

    await waitFor(() => {
      expect(mockUpdateNotificationPreferences).toHaveBeenCalledWith(expect.objectContaining({
        push_enabled: false,
        sms_booking_confirm: true,
        sms_booking_reminder: true,
        sms_booking_cancelled: true,
        whatsapp_booking_confirm: true,
        whatsapp_booking_reminder: true,
        whatsapp_booking_cancelled: true,
      }));
    });
  });

  it('updates all email toggles before saving', async () => {
    const user = userEvent.setup();
    mockGetNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });
    mockUpdateNotificationPreferences.mockResolvedValue({ success: true, data: defaultPrefs });

    render(<NotificationPreferencesComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading preferences...')).not.toBeInTheDocument();
    });

    const switches = screen.getAllByRole('switch');
    await user.click(switches[1]);
    await user.click(switches[2]);

    await user.click(screen.getByText('Save Preferences'));

    await waitFor(() => {
      expect(mockUpdateNotificationPreferences).toHaveBeenCalledWith(expect.objectContaining({
        email_booking_reminder: false,
        email_swap_request: false,
      }));
    });
  });
});
