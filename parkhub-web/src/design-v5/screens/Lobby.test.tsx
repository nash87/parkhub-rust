import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockGet = vi.fn();
const mockUpdate = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    getLobbyConfig: (...a: unknown[]) => mockGet(...a),
    updateLobbyConfig: (...a: unknown[]) => mockUpdate(...a),
  },
}));

const mockToast = vi.fn();
vi.mock('../Toast', () => ({
  useV5Toast: () => mockToast,
  V5ToastProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

import { LobbyV5 } from './Lobby';

const CFG = {
  active_screen: 'queue' as const,
  rotate_interval_seconds: 30,
  show_clock: true,
  show_weather: false,
};

function renderScreen() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <LobbyV5 navigate={vi.fn()} />
    </QueryClientProvider>
  );
}

describe('LobbyV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders error state when config fails', async () => {
    mockGet.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'fail' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders preview with active screen label', async () => {
    mockGet.mockResolvedValue({ success: true, data: CFG });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('lobby-preview')).toBeInTheDocument());
    expect(screen.getAllByText('Warteschlange').length).toBeGreaterThan(0);
  });

  it('selecting a different screen calls updateLobbyConfig', async () => {
    mockGet.mockResolvedValue({ success: true, data: CFG });
    mockUpdate.mockResolvedValue({ success: true, data: { ...CFG, active_screen: 'map' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Karte')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Karte'));
    await waitFor(() => {
      expect(mockUpdate).toHaveBeenCalledWith({ active_screen: 'map' });
      expect(mockToast).toHaveBeenCalledWith('Lobby aktualisiert', 'success');
    });
  });

  it('update error surfaces via toast', async () => {
    mockGet.mockResolvedValue({ success: true, data: CFG });
    mockUpdate.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Karte')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Karte'));
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('denied', 'error'));
  });

  it('toggling clock fires update', async () => {
    mockGet.mockResolvedValue({ success: true, data: CFG });
    mockUpdate.mockResolvedValue({ success: true, data: { ...CFG, show_clock: false } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Uhr anzeigen')).toBeInTheDocument());
    const toggles = screen.getAllByRole('switch');
    fireEvent.click(toggles[0]);
    await waitFor(() => expect(mockUpdate).toHaveBeenCalledWith({ show_clock: false }));
  });

  it('changing rotate interval commits on blur', async () => {
    mockGet.mockResolvedValue({ success: true, data: CFG });
    mockUpdate.mockResolvedValue({ success: true, data: { ...CFG, rotate_interval_seconds: 60 } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('lobby-interval')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('lobby-interval'), { target: { value: '60' } });
    fireEvent.blur(screen.getByTestId('lobby-interval'));
    await waitFor(() => expect(mockUpdate).toHaveBeenCalledWith({ rotate_interval_seconds: 60 }));
  });
});
