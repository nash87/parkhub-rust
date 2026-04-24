import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockGetSwaps = vi.fn();
const mockGetBookings = vi.fn();
const mockAccept = vi.fn();
const mockDecline = vi.fn();
const mockCreate = vi.fn();

vi.mock('../../api/client', () => ({
  api: {
    getSwapRequests: (...a: unknown[]) => mockGetSwaps(...a),
    getBookings: (...a: unknown[]) => mockGetBookings(...a),
    acceptSwap: (...a: unknown[]) => mockAccept(...a),
    declineSwap: (...a: unknown[]) => mockDecline(...a),
    createSwapRequest: (...a: unknown[]) => mockCreate(...a),
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

import { TauschV5 } from './Tausch';

function renderScreen(navigate = vi.fn()) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <TauschV5 navigate={navigate} />
    </QueryClientProvider>
  );
}

const SWAP_PENDING = {
  id: 'sw1', requester_id: 'u1',
  source_booking_id: 'b1', target_booking_id: 'b2',
  source_booking: { lot_name: 'Nord', slot_number: 'A1', start_time: '2026-04-23T08:00:00Z', end_time: '2026-04-23T18:00:00Z' },
  target_booking: { lot_name: 'Süd', slot_number: 'B2', start_time: '2026-04-23T08:00:00Z', end_time: '2026-04-23T18:00:00Z' },
  message: 'Kurztausch',
  status: 'pending' as const,
  created_at: '2026-04-22T10:00:00Z',
};
const SWAP_ACCEPTED = { ...SWAP_PENDING, id: 'sw2', status: 'accepted' as const, message: null };

const BOOKING_ACTIVE = {
  id: 'b1', user_id: 'u1', lot_id: 'l1', slot_id: 's1',
  lot_name: 'Nord', slot_number: 'A1', vehicle_plate: 'M-XY',
  start_time: '2026-04-23T08:00:00Z', end_time: '2026-04-23T18:00:00Z',
  status: 'active' as const,
};

describe('TauschV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state when no swap requests', async () => {
    mockGetSwaps.mockResolvedValue({ success: true, data: [] });
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine Tauschanfragen')).toBeInTheDocument());
  });

  it('renders error state when query fails', async () => {
    mockGetSwaps.mockRejectedValue(new Error('network'));
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('surfaces query error when success:false', async () => {
    mockGetSwaps.mockResolvedValue({ success: false, data: null, error: { code: 'FORBIDDEN', message: 'denied' } });
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders swap rows with status badges', async () => {
    mockGetSwaps.mockResolvedValue({ success: true, data: [SWAP_PENDING, SWAP_ACCEPTED] });
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('swap-row')).toHaveLength(2));
    expect(screen.getByText('Offen')).toBeInTheDocument();
    expect(screen.getByText('Angenommen')).toBeInTheDocument();
  });

  it('accept button calls acceptSwap and toast', async () => {
    mockGetSwaps.mockResolvedValue({ success: true, data: [SWAP_PENDING] });
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockAccept.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Annehmen')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Annehmen'));
    await waitFor(() => {
      expect(mockAccept).toHaveBeenCalledWith('sw1');
      expect(mockToast).toHaveBeenCalledWith('Tausch angenommen', 'success');
    });
  });

  it('emits error toast when acceptSwap success:false', async () => {
    mockGetSwaps.mockResolvedValue({ success: true, data: [SWAP_PENDING] });
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockAccept.mockResolvedValue({ success: false, data: null, error: { code: 'CONFLICT', message: 'expired' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Annehmen')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Annehmen'));
    await waitFor(() => {
      expect(mockToast).toHaveBeenCalledWith('expired', 'error');
    });
    expect(mockToast).not.toHaveBeenCalledWith('Tausch angenommen', 'success');
  });

  it('decline button calls declineSwap and toast', async () => {
    mockGetSwaps.mockResolvedValue({ success: true, data: [SWAP_PENDING] });
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    mockDecline.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Ablehnen')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Ablehnen'));
    await waitFor(() => {
      expect(mockDecline).toHaveBeenCalledWith('sw1');
      expect(mockToast).toHaveBeenCalledWith('Tausch abgelehnt', 'success');
    });
  });

  it('can open create modal and submit when active bookings exist', async () => {
    mockGetSwaps.mockResolvedValue({ success: true, data: [] });
    mockGetBookings.mockResolvedValue({ success: true, data: [BOOKING_ACTIVE] });
    mockCreate.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('open-create-swap')).not.toBeDisabled());
    fireEvent.click(screen.getByTestId('open-create-swap'));
    expect(screen.getByTestId('swap-source')).toBeInTheDocument();
    fireEvent.change(screen.getByTestId('swap-source'), { target: { value: 'b1' } });
    fireEvent.change(screen.getByTestId('swap-target'), { target: { value: 'b2' } });
    fireEvent.click(screen.getByTestId('swap-submit'));
    await waitFor(() => {
      expect(mockCreate).toHaveBeenCalledWith('b1', 'b2', null);
      expect(mockToast).toHaveBeenCalledWith('Tauschanfrage gesendet', 'success');
    });
  });
});
