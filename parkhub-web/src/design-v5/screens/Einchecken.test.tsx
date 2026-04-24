import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockGetBookings = vi.fn();
const mockGetStatus = vi.fn();
const mockCheckIn = vi.fn();
const mockCheckOut = vi.fn();

vi.mock('../../api/client', () => ({
  api: {
    getBookings: (...a: unknown[]) => mockGetBookings(...a),
    getCheckInStatus: (...a: unknown[]) => mockGetStatus(...a),
    checkIn: (...a: unknown[]) => mockCheckIn(...a),
    checkOut: (...a: unknown[]) => mockCheckOut(...a),
  },
}));

const mockToast = vi.fn();
vi.mock('../Toast', () => ({
  useV5Toast: () => mockToast,
  V5ToastProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

import { EincheckenV5 } from './Einchecken';

function renderScreen(navigate = vi.fn()) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <EincheckenV5 navigate={navigate} />
    </QueryClientProvider>
  );
}

// Booking that is "active right now" — window from 1h ago to 1h ahead
function activeBooking() {
  const now = Date.now();
  return {
    id: 'b1', user_id: 'u1', lot_id: 'l1', slot_id: 's1',
    lot_name: 'Nord', slot_number: 'A1', vehicle_plate: 'M-XY',
    start_time: new Date(now - 3600000).toISOString(),
    end_time: new Date(now + 3600000).toISOString(),
    status: 'active' as const,
  };
}

describe('EincheckenV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state when no active booking', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine aktive Buchung')).toBeInTheDocument());
  });

  it('renders error state when bookings query fails', async () => {
    mockGetBookings.mockRejectedValue(new Error('network'));
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('surfaces error when bookings success:false', async () => {
    mockGetBookings.mockResolvedValue({ success: false, data: null, error: { code: 'FORBIDDEN', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('shows check-in card when booking is active and not yet checked in', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [activeBooking()] });
    mockGetStatus.mockResolvedValue({ success: true, data: { checked_in: false, checked_in_at: null, checked_out_at: null } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('checkin-card')).toBeInTheDocument());
    expect(screen.getByText('Nord')).toBeInTheDocument();
  });

  it('check-in button calls api.checkIn and toast', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [activeBooking()] });
    mockGetStatus.mockResolvedValue({ success: true, data: { checked_in: false, checked_in_at: null, checked_out_at: null } });
    mockCheckIn.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('checkin-btn')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('checkin-btn'));
    await waitFor(() => {
      expect(mockCheckIn).toHaveBeenCalledWith('b1');
      expect(mockToast).toHaveBeenCalledWith('Eingecheckt', 'success');
    });
  });

  it('emits error toast when checkIn success:false', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [activeBooking()] });
    mockGetStatus.mockResolvedValue({ success: true, data: { checked_in: false, checked_in_at: null, checked_out_at: null } });
    mockCheckIn.mockResolvedValue({ success: false, data: null, error: { code: 'TOO_EARLY', message: 'too early' } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('checkin-btn')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('checkin-btn'));
    await waitFor(() => {
      expect(mockToast).toHaveBeenCalledWith('too early', 'error');
    });
    expect(mockToast).not.toHaveBeenCalledWith('Eingecheckt', 'success');
  });

  it('shows checked-in card with elapsed timer when already checked in', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [activeBooking()] });
    mockGetStatus.mockResolvedValue({
      success: true,
      data: { checked_in: true, checked_in_at: new Date(Date.now() - 65000).toISOString(), checked_out_at: null },
    });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('checked-in-card')).toBeInTheDocument());
    expect(screen.getByTestId('elapsed')).toBeInTheDocument();
    expect(screen.getByTestId('checkout-btn')).toBeInTheDocument();
  });

  it('checkout button calls api.checkOut and toast', async () => {
    mockGetBookings.mockResolvedValue({ success: true, data: [activeBooking()] });
    mockGetStatus.mockResolvedValue({
      success: true,
      data: { checked_in: true, checked_in_at: new Date().toISOString(), checked_out_at: null },
    });
    mockCheckOut.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('checkout-btn')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('checkout-btn'));
    await waitFor(() => {
      expect(mockCheckOut).toHaveBeenCalledWith('b1');
      expect(mockToast).toHaveBeenCalledWith('Ausgecheckt', 'success');
    });
  });
});
