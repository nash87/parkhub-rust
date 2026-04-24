import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockGetGuests = vi.fn();
const mockGetLots = vi.fn();
const mockGetSlots = vi.fn();
const mockCreateGuest = vi.fn();
const mockCancelGuest = vi.fn();

vi.mock('../../api/client', () => ({
  api: {
    getGuestBookings: (...a: unknown[]) => mockGetGuests(...a),
    getLots: (...a: unknown[]) => mockGetLots(...a),
    getLotSlots: (...a: unknown[]) => mockGetSlots(...a),
    createGuestBooking: (...a: unknown[]) => mockCreateGuest(...a),
    cancelGuestBooking: (...a: unknown[]) => mockCancelGuest(...a),
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

import { GaestepassV5 } from './Gaestepass';

function renderScreen(navigate = vi.fn()) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <GaestepassV5 navigate={navigate} />
    </QueryClientProvider>
  );
}

const LOT = { id: 'lot-1', name: 'Nord' };
const GUEST_ACTIVE = {
  id: 'g1', lot_id: 'lot-1', lot_name: 'Nord',
  slot_id: 's1', slot_number: 'A1',
  guest_name: 'Max Mustermann', guest_email: 'max@example.com',
  guest_code: 'ABC123',
  start_time: '2026-04-23T08:00:00Z', end_time: '2026-04-23T18:00:00Z',
  status: 'active' as const, created_at: '2026-04-22T10:00:00Z',
};
const GUEST_EXPIRED = { ...GUEST_ACTIVE, id: 'g2', guest_code: 'XYZ999', status: 'expired' as const };

describe('GaestepassV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state when no guest passes', async () => {
    mockGetGuests.mockResolvedValue({ success: true, data: [] });
    mockGetLots.mockResolvedValue({ success: true, data: [LOT] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Noch keine Gäste-Pässe')).toBeInTheDocument());
  });

  it('renders error state when guests query fails', async () => {
    mockGetGuests.mockRejectedValue(new Error('network'));
    mockGetLots.mockResolvedValue({ success: true, data: [LOT] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('surfaces error when guests success:false', async () => {
    mockGetGuests.mockResolvedValue({ success: false, data: null, error: { code: 'FORBIDDEN', message: 'denied' } });
    mockGetLots.mockResolvedValue({ success: true, data: [LOT] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders guest rows with codes and statuses', async () => {
    mockGetGuests.mockResolvedValue({ success: true, data: [GUEST_ACTIVE, GUEST_EXPIRED] });
    mockGetLots.mockResolvedValue({ success: true, data: [LOT] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('guest-row')).toHaveLength(2));
    expect(screen.getAllByText('Max Mustermann').length).toBeGreaterThan(0);
    expect(screen.getByText('ABC123')).toBeInTheDocument();
    expect(screen.getByText('XYZ999')).toBeInTheDocument();
    expect(screen.getAllByText(/Aktiv/).length).toBeGreaterThan(0);
    expect(screen.getByText('Abgelaufen')).toBeInTheDocument();
  });

  it('cancel button calls cancelGuestBooking and toast', async () => {
    mockGetGuests.mockResolvedValue({ success: true, data: [GUEST_ACTIVE] });
    mockGetLots.mockResolvedValue({ success: true, data: [LOT] });
    mockCancelGuest.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('cancel-g1')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('cancel-g1'));
    await waitFor(() => {
      expect(mockCancelGuest).toHaveBeenCalledWith('g1');
      expect(mockToast).toHaveBeenCalledWith('Pass storniert', 'success');
    });
  });

  it('emits error toast when cancel success:false', async () => {
    mockGetGuests.mockResolvedValue({ success: true, data: [GUEST_ACTIVE] });
    mockGetLots.mockResolvedValue({ success: true, data: [LOT] });
    mockCancelGuest.mockResolvedValue({ success: false, data: null, error: { code: 'CONFLICT', message: 'expired' } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('cancel-g1')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('cancel-g1'));
    await waitFor(() => {
      expect(mockToast).toHaveBeenCalledWith('expired', 'error');
    });
    expect(mockToast).not.toHaveBeenCalledWith('Pass storniert', 'success');
  });

  it('opens create modal with create button', async () => {
    mockGetGuests.mockResolvedValue({ success: true, data: [] });
    mockGetLots.mockResolvedValue({ success: true, data: [LOT] });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('open-create-guest')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('open-create-guest'));
    expect(screen.getByTestId('guest-name')).toBeInTheDocument();
    expect(screen.getByTestId('guest-lot')).toBeInTheDocument();
  });
});
