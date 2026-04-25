import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

/**
 * Einstellungen v5 — multi-tab settings hub. Admin/system fields live on the
 * "System" tab and surface tenant settings via `api.adminGetSettings` /
 * `api.adminUpdateSettings`. Personal preferences (mode, sidebar variant,
 * density, font, feature toggles) live on the other tabs and persist to the
 * client-side V5SettingsProvider.
 */
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
  useV5Theme: () => ({ mode: 'marble_light', setMode: vi.fn(), isVoid: false, isDark: false }),
  V5_MODES: ['marble_light', 'marble_dark', 'void'] as const,
  V5_MODE_LABELS: { marble_light: 'Marble', marble_dark: 'Marble Dark', void: 'Void' },
}));

import { EinstellungenV5 } from './Einstellungen';
import { V5SettingsProvider } from '../settings';

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
      <V5SettingsProvider>
        <EinstellungenV5 navigate={vi.fn()} />
      </V5SettingsProvider>
    </QueryClientProvider>,
  );
}

async function switchTab(label: string) {
  fireEvent.click(screen.getByRole('tab', { name: label }));
}

describe('EinstellungenV5 (admin tab — System)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
  });

  it('renders error state when adminGetSettings fails', async () => {
    mockGetSettings.mockResolvedValue({
      success: false,
      data: null,
      error: { code: 'X', message: 'oops' },
    });
    renderScreen();
    await switchTab('System');
    await waitFor(() =>
      expect(screen.getByText('Fehler beim Laden der System-Einstellungen.')).toBeInTheDocument(),
    );
  });

  it('renders company name field populated from admin settings', async () => {
    mockGetSettings.mockResolvedValue({ success: true, data: SETTINGS });
    renderScreen();
    await switchTab('System');
    await waitFor(() =>
      expect((screen.getByTestId('einst-company-name') as HTMLInputElement).value).toBe('ACME GmbH'),
    );
  });

  it('renders booking window (days) field populated from admin settings', async () => {
    mockGetSettings.mockResolvedValue({ success: true, data: SETTINGS });
    renderScreen();
    await switchTab('System');
    await waitFor(() =>
      expect((screen.getByTestId('einst-booking-window') as HTMLInputElement).value).toBe('30'),
    );
  });

  it('saves changed admin settings via adminUpdateSettings', async () => {
    mockGetSettings.mockResolvedValue({ success: true, data: SETTINGS });
    mockUpdateSettings.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await switchTab('System');
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
    mockUpdateSettings.mockResolvedValue({
      success: false,
      data: null,
      error: { code: 'E', message: 'denied' },
    });
    renderScreen();
    await switchTab('System');
    await waitFor(() => expect(screen.getByTestId('einst-company-name')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('einst-company-name'), { target: { value: 'Changed' } });
    fireEvent.click(screen.getByTestId('einst-save'));
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('denied', 'error'));
  });
});

describe('EinstellungenV5 (Erscheinungsbild tab)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
  });

  it('switches language chip and toasts (UI pref, independent of admin settings)', async () => {
    mockGetSettings.mockResolvedValue({ success: true, data: SETTINGS });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Deutsch')).toBeInTheDocument());
    fireEvent.click(screen.getByText('English'));
    expect(mockToast).toHaveBeenCalledWith('Sprache aktualisiert', 'success');
  });

  it('exposes mode, sidebar, density, font and language pickers', () => {
    mockGetSettings.mockResolvedValue({ success: true, data: SETTINGS });
    renderScreen();
    expect(screen.getAllByTestId('einst-theme').length).toBeGreaterThanOrEqual(3);
    expect(screen.getAllByTestId('einst-sidebar').length).toBe(3);
    expect(screen.getAllByTestId('einst-density').length).toBe(3);
    expect(screen.getAllByTestId('einst-font').length).toBe(5);
  });

  it('clicking the columns sidebar chip persists the setting', async () => {
    mockGetSettings.mockResolvedValue({ success: true, data: SETTINGS });
    renderScreen();
    const columnsChip = screen
      .getAllByTestId('einst-sidebar')
      .find((b) => (b as HTMLElement).dataset.value === 'columns');
    fireEvent.click(columnsChip!);
    await waitFor(() => {
      const stored = JSON.parse(window.localStorage.getItem('ph-v5-settings') ?? '{}');
      expect(stored.appearance.sidebar).toBe('columns');
    });
  });
});

describe('EinstellungenV5 (Funktionen tab)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
    mockGetSettings.mockResolvedValue({ success: true, data: SETTINGS });
  });

  it('renders all 11 feature toggles', async () => {
    renderScreen();
    await switchTab('Funktionen');
    expect(screen.getByTestId('einst-feature-smartSuggestions')).toBeInTheDocument();
    expect(screen.getByTestId('einst-feature-optimisticUI')).toBeInTheDocument();
    expect(screen.getByTestId('einst-feature-viewTransitions')).toBeInTheDocument();
    expect(screen.getByTestId('einst-feature-voiceCommands')).toBeInTheDocument();
    expect(screen.getByTestId('einst-feature-qrCheckin')).toBeInTheDocument();
    expect(screen.getByTestId('einst-feature-deepLinking')).toBeInTheDocument();
    expect(screen.getByTestId('einst-feature-predictiveCard')).toBeInTheDocument();
    expect(screen.getByTestId('einst-feature-swAutoUpdate')).toBeInTheDocument();
    expect(screen.getByTestId('einst-feature-plateScan')).toBeInTheDocument();
    expect(screen.getByTestId('einst-feature-semanticSearch')).toBeInTheDocument();
    expect(screen.getByTestId('einst-feature-fleetSSE')).toBeInTheDocument();
  });

  it('toggling voiceCommands persists', async () => {
    renderScreen();
    await switchTab('Funktionen');
    const toggle = screen.getByTestId('einst-feature-voiceCommands');
    expect(toggle.getAttribute('aria-checked')).toBe('false');
    fireEvent.click(toggle);
    await waitFor(() => {
      const stored = JSON.parse(window.localStorage.getItem('ph-v5-settings') ?? '{}');
      expect(stored.features.voiceCommands).toBe(true);
    });
  });

  it('NEVER references "AI" in any feature label or hint', async () => {
    renderScreen();
    await switchTab('Funktionen');
    const panel = screen.getByRole('tabpanel');
    expect(panel.textContent ?? '').not.toMatch(/\bAI\b|\bKI\b/i);
  });
});

describe('EinstellungenV5 (Barrierefreiheit tab)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
    mockGetSettings.mockResolvedValue({ success: true, data: SETTINGS });
  });

  it('exposes reduced-motion + high-contrast toggles + font scale picker', async () => {
    renderScreen();
    await switchTab('Barrierefreiheit');
    expect(screen.getByTestId('einst-reduced-motion')).toBeInTheDocument();
    expect(screen.getByTestId('einst-high-contrast')).toBeInTheDocument();
    expect(screen.getAllByTestId('einst-fontscale').length).toBe(4);
  });

  it('toggling reduced-motion sets data-ph-reduced-motion on <html>', async () => {
    renderScreen();
    await switchTab('Barrierefreiheit');
    fireEvent.click(screen.getByTestId('einst-reduced-motion'));
    await waitFor(() =>
      expect(document.documentElement.getAttribute('data-ph-reduced-motion')).toBe('true'),
    );
  });
});

describe('EinstellungenV5 (Datenschutz tab)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.clear();
    mockGetSettings.mockResolvedValue({ success: true, data: SETTINGS });
  });

  it('reset button restores defaults and toasts', async () => {
    renderScreen();
    // Make a change first so reset is observable.
    const columnsChip = screen
      .getAllByTestId('einst-sidebar')
      .find((b) => (b as HTMLElement).dataset.value === 'columns');
    fireEvent.click(columnsChip!);
    await switchTab('Datenschutz');
    fireEvent.click(screen.getByTestId('einst-reset'));
    await waitFor(() => {
      const stored = JSON.parse(window.localStorage.getItem('ph-v5-settings') ?? '{}');
      expect(stored.appearance.sidebar).toBe('marble');
      expect(mockToast).toHaveBeenCalledWith('Einstellungen zurückgesetzt', 'success');
    });
  });
});
