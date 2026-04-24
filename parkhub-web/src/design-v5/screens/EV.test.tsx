import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockGetLots = vi.fn();
const mockGetChargers = vi.fn();
const mockGetSessions = vi.fn();
const mockStart = vi.fn();
const mockStop = vi.fn();

vi.mock('../../api/client', () => ({
  api: {
    getLots: (...a: unknown[]) => mockGetLots(...a),
    getLotChargers: (...a: unknown[]) => mockGetChargers(...a),
    getChargerSessions: (...a: unknown[]) => mockGetSessions(...a),
    startCharging: (...a: unknown[]) => mockStart(...a),
    stopCharging: (...a: unknown[]) => mockStop(...a),
  },
}));

vi.mock('@number-flow/react', () => ({
  default: ({ value }: { value: number }) => <span>{value}</span>,
}));

const mockToast = vi.fn();
vi.mock('../Toast', () => ({
  useV5Toast: () => mockToast,
  V5ToastProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

import { EVV5 } from './EV';

function renderScreen(navigate = vi.fn()) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <EVV5 navigate={navigate} />
    </QueryClientProvider>
  );
}

const LOT = { id: 'lot-1', name: 'Nord' };
const CHARGER_AVAIL = {
  id: 'c1', lot_id: 'lot-1', label: 'C1',
  connector_type: 'type2' as const, power_kw: 22,
  status: 'available' as const, location_hint: 'Level 1',
};
const CHARGER_BUSY = {
  id: 'c2', lot_id: 'lot-1', label: 'C2',
  connector_type: 'ccs' as const, power_kw: 50,
  status: 'in_use' as const, location_hint: null,
};
const SESSION_ACTIVE = {
  id: 's1', charger_id: 'c2', user_id: 'u1',
  start_time: '2026-04-23T08:00:00Z', end_time: null,
  kwh_consumed: 5, status: 'active' as const,
};

describe('EVV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state when no chargers', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [LOT] });
    mockGetChargers.mockResolvedValue({ success: true, data: [] });
    mockGetSessions.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine Ladestationen')).toBeInTheDocument());
  });

  it('renders error state when chargers query fails', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [LOT] });
    mockGetChargers.mockRejectedValue(new Error('network'));
    mockGetSessions.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('surfaces query error when chargers success:false', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [LOT] });
    mockGetChargers.mockResolvedValue({ success: false, data: null, error: { code: 'FORBIDDEN', message: 'denied' } });
    mockGetSessions.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders charger cards with status badges', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [LOT] });
    mockGetChargers.mockResolvedValue({ success: true, data: [CHARGER_AVAIL, CHARGER_BUSY] });
    mockGetSessions.mockResolvedValue({ success: true, data: [SESSION_ACTIVE] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('charger-card')).toHaveLength(2));
    // "Verfügbar" appears both as stat tile + status badge
    expect(screen.getAllByText('Verfügbar').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('Belegt')).toBeInTheDocument();
  });

  it('start button triggers startCharging and toast', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [LOT] });
    mockGetChargers.mockResolvedValue({ success: true, data: [CHARGER_AVAIL] });
    mockGetSessions.mockResolvedValue({ success: true, data: [] });
    mockStart.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Laden starten')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Laden starten'));
    await waitFor(() => {
      expect(mockStart).toHaveBeenCalledWith('c1');
      expect(mockToast).toHaveBeenCalledWith('Laden gestartet', 'success');
    });
  });

  it('emits error toast when startCharging success:false', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [LOT] });
    mockGetChargers.mockResolvedValue({ success: true, data: [CHARGER_AVAIL] });
    mockGetSessions.mockResolvedValue({ success: true, data: [] });
    mockStart.mockResolvedValue({ success: false, data: null, error: { code: 'CONFLICT', message: 'in use' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Laden starten')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Laden starten'));
    await waitFor(() => {
      expect(mockToast).toHaveBeenCalledWith('in use', 'error');
    });
    expect(mockToast).not.toHaveBeenCalledWith('Laden gestartet', 'success');
  });

  it('shows stop button on charger with active session', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [LOT] });
    mockGetChargers.mockResolvedValue({ success: true, data: [CHARGER_BUSY] });
    mockGetSessions.mockResolvedValue({ success: true, data: [SESSION_ACTIVE] });
    mockStop.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Laden beenden')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Laden beenden'));
    await waitFor(() => {
      expect(mockStop).toHaveBeenCalledWith('c2');
      expect(mockToast).toHaveBeenCalledWith('Laden beendet', 'success');
    });
  });
});
