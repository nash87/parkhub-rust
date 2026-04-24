import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockGetBookings = vi.fn();
const mockCancelBooking = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    getBookings: (...a: unknown[]) => mockGetBookings(...a),
    cancelBooking: (...a: unknown[]) => mockCancelBooking(...a),
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

import { BuchungenV5 } from './Buchungen';

function renderScreen(navigate = vi.fn()) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <BuchungenV5 navigate={navigate} />
    </QueryClientProvider>
  );
}

const BOOKING_ACTIVE = {
  id: 'b-active-001',
  user_id: 'u-1',
  lot_id: 'l-1',
  slot_id: 's-1',
  lot_name: 'Parkhaus Nord',
  slot_number: 'A3',
  vehicle_plate: 'M-XY 999',
  start_time: '2026-04-23T08:00:00Z',
  end_time: '2026-04-23T18:00:00Z',
  status: 'active' as const,
};

const BOOKING_CONFIRMED = { ...BOOKING_ACTIVE, id: 'b-conf-001', status: 'confirmed' as const };
const BOOKING_DONE = { ...BOOKING_ACTIVE, id: 'b-done-001', status: 'completed' as const };

describe('BuchungenV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state with CTA when no bookings', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine Buchungen gefunden')).toBeInTheDocument());
    expect(screen.getByText('+ Platz buchen')).toBeInTheDocument();
  });

  it('renders error state when query fails', async () => {
    mockGetBookings.mockRejectedValue(new Error('network'));
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders booking rows with lot, plate and status badge', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [BOOKING_ACTIVE] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Parkhaus Nord')).toBeInTheDocument());
    expect(screen.getByText('M-XY 999')).toBeInTheDocument();
    // "Aktiv" also appears as a filter chip — check the count matches a badge + chip
    expect(screen.getAllByText('Aktiv')).toHaveLength(2);
  });

  it('filters rows by status when a chip is activated', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [BOOKING_ACTIVE, BOOKING_DONE] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('buchungen-row')).toHaveLength(2));
    fireEvent.click(screen.getByRole('button', { name: 'Abgeschlossen' }));
    await waitFor(() => expect(screen.getAllByTestId('buchungen-row')).toHaveLength(1));
    expect(screen.getByRole('button', { name: 'Abgeschlossen' })).toHaveAttribute('aria-pressed', 'true');
  });

  it('Storno button calls cancelBooking + toast', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [BOOKING_CONFIRMED] });
    mockCancelBooking.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Storno')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Storno'));
    await waitFor(() => {
      expect(mockCancelBooking).toHaveBeenCalledWith('b-conf-001');
      expect(mockToast).toHaveBeenCalledWith('Buchung storniert', 'success');
    });
  });

  it('Check-in button navigates and emits toast on active booking', async () => {
    const navigate = vi.fn();
    mockGetBookings.mockResolvedValue({ success: true, data: [BOOKING_ACTIVE] });
    renderScreen(navigate);
    await waitFor(() => expect(screen.getByText('Check-in')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Check-in'));
    expect(navigate).toHaveBeenCalledWith('einchecken');
    expect(mockToast).toHaveBeenCalledWith('Einchecken geöffnet', 'info');
  });

  it('surfaces query error when success:false', async () => {
    mockGetBookings.mockResolvedValue({ success: false, data: null, error: { code: 'FORBIDDEN', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('calls onError (no success toast) when cancel mutation responds success:false', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [BOOKING_CONFIRMED] });
    mockCancelBooking.mockResolvedValue({ success: false, data: null, error: { code: 'CONFLICT', message: 'already cancelled' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Storno')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Storno'));
    await waitFor(() => {
      expect(mockCancelBooking).toHaveBeenCalledWith('b-conf-001');
      expect(mockToast).toHaveBeenCalledWith('Stornierung fehlgeschlagen', 'error');
    });
    expect(mockToast).not.toHaveBeenCalledWith('Buchung storniert', 'success');
  });
});
