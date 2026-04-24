import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

// v5 Einstellungen is under the admin nav section → it must surface tenant
// settings (company, booking rules, cost-center tagging, etc.) via
// `api.adminGetSettings`/`api.adminUpdateSettings`. The former draft called
// `api.me`/`api.updateMe`, which is the user-scoped profile endpoint and is
// wired into the separate Profil screen instead.
const mockGetSettings = vi.fn();
const mockUpdateSettings = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    adminGetSettings: (...a: unknown[]) => mockGetSettings(...a),
    adminUpdateSettings: (...a: unknown[]) => mockUpdateSettings(...a),
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

const SETTINGS: Record<string, string> = {
  company_name: 'ACME GmbH',
  default_currency: 'EUR',
  booking_window_days: '30',
  cost_center_required: 'true',
};

function renderScreen() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <EinstellungenV5 navigate={vi.fn()} />
    </QueryClientProvider>
  );
}

describe('EinstellungenV5 (admin)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders error state when adminGetSettings fails', async () => {
    mockGetSettings.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'oops' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders company name field populated from admin settings', async () => {
    mockGetSettings.mockResolvedValue({ success: true, data: SETTINGS });
    renderScreen();
    await waitFor(() => expect((screen.getByTestId('einst-company-name') as HTMLInputElement).value).toBe('ACME GmbH'));
  });

  it('renders booking window (days) field populated from admin settings', async () => {
    mockGetSettings.mockResolvedValue({ success: true, data: SETTINGS });
    renderScreen();
    await waitFor(() => expect((screen.getByTestId('einst-booking-window') as HTMLInputElement).value).toBe('30'));
  });

  it('saves changed admin settings via adminUpdateSettings', async () => {
    mockGetSettings.mockResolvedValue({ success: true, data: SETTINGS });
    mockUpdateSettings.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('einst-company-name')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('einst-company-name'), { target: { value: 'New Co' } });
    fireEvent.change(screen.getByTestId('einst-booking-window'), { target: { value: '45' } });
    fireEvent.click(screen.getByTestId('einst-save'));
    await waitFor(() => {
      expect(mockUpdateSettings).toHaveBeenCalledWith(
        expect.objectContaining({
          company_name: 'New Co',
          booking_window_days: '45',
        }),
      );
      expect(mockToast).toHaveBeenCalledWith('Einstellungen gespeichert', 'success');
    });
  });

  it('surfaces adminUpdateSettings error when success:false', async () => {
    mockGetSettings.mockResolvedValue({ success: true, data: SETTINGS });
    mockUpdateSettings.mockResolvedValue({ success: false, data: null, error: { code: 'E', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('einst-company-name')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('einst-company-name'), { target: { value: 'Changed' } });
    fireEvent.click(screen.getByTestId('einst-save'));
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('denied', 'error'));
  });

  it('switches language chip and toasts (UI pref, independent of admin settings)', async () => {
    mockGetSettings.mockResolvedValue({ success: true, data: SETTINGS });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Deutsch')).toBeInTheDocument());
    fireEvent.click(screen.getByText('English'));
    expect(mockToast).toHaveBeenCalledWith('Sprache aktualisiert', 'success');
  });
});
