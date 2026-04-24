import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockMe = vi.fn();
const mockUpdateMe = vi.fn();
const mockGetPrefs = vi.fn();
const mockUpdatePrefs = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    me: (...a: unknown[]) => mockMe(...a),
    updateMe: (...a: unknown[]) => mockUpdateMe(...a),
    getNotificationPreferences: (...a: unknown[]) => mockGetPrefs(...a),
    updateNotificationPreferences: (...a: unknown[]) => mockUpdatePrefs(...a),
  },
}));

const mockToast = vi.fn();
vi.mock('../Toast', () => ({
  useV5Toast: () => mockToast,
  V5ToastProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('../ThemeProvider', () => ({
  useV5Theme: () => ({ mode: 'marble', setMode: vi.fn() }),
  V5_MODES: ['marble', 'void'] as const,
  V5_MODE_LABELS: { marble: 'Marble', void: 'Void' },
}));

import { EinstellungenV5 } from './Einstellungen';

const USER = {
  id: 'u-1', username: 'flo', email: 'f@e', name: 'Flo',
  role: 'admin' as const, preferences: {}, is_active: true,
  credits_balance: 0, credits_monthly_quota: 0, department: 'IT',
};

const PREFS = {
  email_booking_confirm: true, email_booking_reminder: false,
  email_swap_request: true, push_enabled: false,
  sms_booking_confirm: false, sms_booking_reminder: false, sms_booking_cancelled: false,
  whatsapp_booking_confirm: false, whatsapp_booking_reminder: false, whatsapp_booking_cancelled: false,
};

function renderScreen() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <EinstellungenV5 navigate={vi.fn()} />
    </QueryClientProvider>
  );
}

describe('EinstellungenV5', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetPrefs.mockResolvedValue({ success: true, data: PREFS });
  });

  it('renders error state when me fails', async () => {
    mockMe.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'oops' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders department field populated from user', async () => {
    mockMe.mockResolvedValue({ success: true, data: USER });
    renderScreen();
    await waitFor(() => expect((screen.getByTestId('einst-department') as HTMLInputElement).value).toBe('IT'));
  });

  it('saves department change via updateMe', async () => {
    mockMe.mockResolvedValue({ success: true, data: USER });
    mockUpdateMe.mockResolvedValue({ success: true, data: { ...USER, department: 'Ops' } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('einst-department')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('einst-department'), { target: { value: 'Ops' } });
    fireEvent.click(screen.getByTestId('einst-save'));
    await waitFor(() => {
      expect(mockUpdateMe).toHaveBeenCalledWith({ department: 'Ops' });
      expect(mockToast).toHaveBeenCalledWith('Einstellungen gespeichert', 'success');
    });
  });

  it('surfaces updateMe error when success:false', async () => {
    mockMe.mockResolvedValue({ success: true, data: USER });
    mockUpdateMe.mockResolvedValue({ success: false, data: null, error: { code: 'E', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('einst-department')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('einst-department'), { target: { value: 'Ops' } });
    fireEvent.click(screen.getByTestId('einst-save'));
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('denied', 'error'));
  });

  it('toggles a notification preference and calls updatePrefs', async () => {
    mockMe.mockResolvedValue({ success: true, data: USER });
    mockUpdatePrefs.mockResolvedValue({ success: true, data: { ...PREFS, push_enabled: true } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Push aktiviert')).toBeInTheDocument());
    const toggles = screen.getAllByRole('switch');
    fireEvent.click(toggles[toggles.length - 1]);
    await waitFor(() => {
      expect(mockUpdatePrefs).toHaveBeenCalled();
      expect(mockToast).toHaveBeenCalledWith('Benachrichtigungen gespeichert', 'success');
    });
  });

  it('surfaces prefs error when success:false', async () => {
    mockMe.mockResolvedValue({ success: true, data: USER });
    mockUpdatePrefs.mockResolvedValue({ success: false, data: null, error: { code: 'E', message: 'fail' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Push aktiviert')).toBeInTheDocument());
    const toggles = screen.getAllByRole('switch');
    fireEvent.click(toggles[0]);
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('fail', 'error'));
  });

  it('switches language chip and toasts', async () => {
    mockMe.mockResolvedValue({ success: true, data: USER });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Deutsch')).toBeInTheDocument());
    fireEvent.click(screen.getByText('English'));
    expect(mockToast).toHaveBeenCalledWith('Sprache aktualisiert', 'success');
  });
});
