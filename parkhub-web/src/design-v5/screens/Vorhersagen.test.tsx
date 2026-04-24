import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockGetStats = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    getAdminStatsExtended: (...a: unknown[]) => mockGetStats(...a),
  },
}));

const mockToast = vi.fn();
vi.mock('../Toast', () => ({
  useV5Toast: () => mockToast,
  V5ToastProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

import { VorhersagenV5 } from './Vorhersagen';

function renderScreen(navigate = vi.fn()) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <VorhersagenV5 navigate={navigate} />
    </QueryClientProvider>
  );
}

const STATS_BASE = {
  total_users: 10,
  total_lots: 2,
  total_bookings: 100,
  active_bookings: 20,
};

describe('VorhersagenV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders error state when query fails', async () => {
    mockGetStats.mockRejectedValue(new Error('network'));
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('surfaces error when success:false', async () => {
    mockGetStats.mockResolvedValue({ success: false, data: null, error: { code: 'FORBIDDEN', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders 7 day cards with fallback data when no historical stats', async () => {
    mockGetStats.mockResolvedValue({ success: true, data: STATS_BASE });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('day-card')).toHaveLength(7));
    expect(screen.getByTestId('forecast-grid')).toBeInTheDocument();
  });

  it('shows recommendation card with best day/time', async () => {
    mockGetStats.mockResolvedValue({ success: true, data: STATS_BASE });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('recommendation')).toBeInTheDocument());
    expect(screen.getByText('Beste Buchungszeit')).toBeInTheDocument();
  });

  it('uses historical occupancy when provided and renders high-load badge', async () => {
    mockGetStats.mockResolvedValue({
      success: true,
      data: {
        ...STATS_BASE,
        occupancy_by_day: {
          '0': { avg_percentage: 85, peak_hour: 10, peak_percentage: 95, bookings: 100 },
        },
        occupancy_by_hour: { '10': 90 },
      },
    });
    renderScreen();
    await waitFor(() => expect(screen.getByText('85%')).toBeInTheDocument());
    expect(screen.getAllByText('Voll').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Ruhig').length).toBeGreaterThan(0);
  });

  it('renders confidence tag on each day', async () => {
    mockGetStats.mockResolvedValue({ success: true, data: STATS_BASE });
    renderScreen();
    await waitFor(() => expect(screen.getAllByText(/% Konfidenz/i).length).toBe(7));
  });

  it('shows KI label in header', async () => {
    mockGetStats.mockResolvedValue({ success: true, data: STATS_BASE });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Vorhersagen')).toBeInTheDocument());
    expect(screen.getByText('KI')).toBeInTheDocument();
  });
});
