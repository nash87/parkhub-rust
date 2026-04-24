import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockGetMapMarkers = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    getMapMarkers: (...a: unknown[]) => mockGetMapMarkers(...a),
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

import { KarteV5 } from './Karte';

function renderScreen(navigate = vi.fn()) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <KarteV5 navigate={navigate} />
    </QueryClientProvider>
  );
}

const MARKER_NORD = {
  id: 'lot-1',
  name: 'Parkhaus Nord',
  address: 'Hauptstr. 10',
  latitude: 48.14,
  longitude: 11.58,
  available_slots: 40,
  total_slots: 50,
  status: 'open',
  color: 'green' as const,
};

const MARKER_SUED = {
  id: 'lot-2',
  name: 'Parkhaus Süd',
  address: 'Marienplatz 5',
  latitude: 48.13,
  longitude: 11.57,
  available_slots: 2,
  total_slots: 60,
  status: 'open',
  color: 'red' as const,
};

describe('KarteV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state when there are no markers', async () => {
    mockGetMapMarkers.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine Standorte')).toBeInTheDocument());
  });

  it('renders error state on query failure', async () => {
    mockGetMapMarkers.mockRejectedValue(new Error('network'));
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders marker pins and list rows with occupancy', async () => {
    mockGetMapMarkers.mockResolvedValue({ success: true, data: [MARKER_NORD, MARKER_SUED] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('karte-marker')).toHaveLength(2));
    expect(screen.getAllByTestId('karte-list-row')).toHaveLength(2);
    // Default selected is the first marker
    expect(screen.getAllByText('Parkhaus Nord').length).toBeGreaterThan(0);
    expect(screen.getByText('Viel frei')).toBeInTheDocument();
  });

  it('updates the selected-lot card when a marker is clicked', async () => {
    mockGetMapMarkers.mockResolvedValue({ success: true, data: [MARKER_NORD, MARKER_SUED] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('karte-marker')).toHaveLength(2));
    fireEvent.click(screen.getAllByTestId('karte-marker')[1]);
    await waitFor(() => expect(screen.getByText('Voll')).toBeInTheDocument());
    // Active marker aria-pressed is true
    expect(screen.getAllByTestId('karte-marker')[1]).toHaveAttribute('aria-pressed', 'true');
  });

  it('navigates to buchen when the "Platz buchen" button is clicked', async () => {
    mockGetMapMarkers.mockResolvedValue({ success: true, data: [MARKER_NORD] });
    const navigate = vi.fn();
    renderScreen(navigate);
    await waitFor(() => expect(screen.getByText('Platz buchen')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Platz buchen'));
    expect(navigate).toHaveBeenCalledWith('buchen');
  });

  it('surfaces query error when success:false', async () => {
    mockGetMapMarkers.mockResolvedValue({ success: false, data: null, error: { code: 'FORBIDDEN', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });
});
