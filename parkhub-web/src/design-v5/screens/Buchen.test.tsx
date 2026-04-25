import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockGetLots = vi.fn();
const mockGetLotSlots = vi.fn();
const mockGetVehicles = vi.fn();
const mockCreateBooking = vi.fn();

vi.mock('../../api/client', () => ({
  api: {
    getLots: (...a: unknown[]) => mockGetLots(...a),
    getLotSlots: (...a: unknown[]) => mockGetLotSlots(...a),
    getVehicles: (...a: unknown[]) => mockGetVehicles(...a),
    createBooking: (...a: unknown[]) => mockCreateBooking(...a),
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

import { BuchenV5 } from './Buchen';

function renderScreen(navigate = vi.fn()) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <BuchenV5 navigate={navigate} />
    </QueryClientProvider>
  );
}

const LOT_OPEN = {
  id: 'lot-1',
  name: 'Parkhaus Nord',
  address: 'Hauptstr. 10',
  total_slots: 10,
  available_slots: 6,
  status: 'open',
  hourly_rate: 2.5,
  currency: '€',
};

const LOT_FULL = {
  id: 'lot-2',
  name: 'Parkhaus Süd',
  total_slots: 5,
  available_slots: 0,
  status: 'open',
};

const LOT_CLOSED = {
  id: 'lot-3',
  name: 'Parkhaus West',
  total_slots: 5,
  available_slots: 5,
  status: 'closed',
};

const SLOT_A = { id: 's-1', lot_id: 'lot-1', slot_number: 'A01', status: 'available' };
const SLOT_B = { id: 's-2', lot_id: 'lot-1', slot_number: 'A02', status: 'available' };
const SLOT_OCCUPIED = { id: 's-3', lot_id: 'lot-1', slot_number: 'A03', status: 'occupied' };

describe('BuchenV5', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Smart defaults persist last-used lot + vehicle in localStorage.
    // Clear between tests so the auto-advance to Step 2 doesn't leak
    // between tests that need to start on the lot picker (Step 1).
    window.localStorage.clear();
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
  });

  it('renders empty state when no lots are open', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [LOT_CLOSED] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine offenen Stellplätze')).toBeInTheDocument());
  });

  it('renders error state when getLots rejects', async () => {
    mockGetLots.mockRejectedValue(new Error('network'));
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders open lots as selectable cards', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [LOT_OPEN, LOT_FULL, LOT_CLOSED] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Parkhaus Nord')).toBeInTheDocument());
    // Closed lot filtered out
    expect(screen.queryByText('Parkhaus West')).not.toBeInTheDocument();
    // Full lot shown with "Voll" badge
    expect(screen.getByText('Voll')).toBeInTheDocument();
    const cards = screen.getAllByTestId('buchen-lot-card');
    expect(cards).toHaveLength(2);
  });

  it('advances to step 2 after lot click and shows slot grid', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [LOT_OPEN] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: [SLOT_A, SLOT_B, SLOT_OCCUPIED] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Parkhaus Nord')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-lot-card'));
    await waitFor(() => expect(screen.getByText('Stellplatz wählen')).toBeInTheDocument());
    await waitFor(() => expect(screen.getAllByTestId('buchen-slot')).toHaveLength(3));
    // Occupied slot is disabled
    const occupiedBtn = screen.getByLabelText(/A03.*belegt/);
    expect(occupiedBtn).toBeDisabled();
  });

  it('confirms booking on step 3 and invokes createBooking + success toast', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [LOT_OPEN] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: [SLOT_A, SLOT_B] });
    mockCreateBooking.mockResolvedValue({ success: true, data: { id: 'new-b-1' } });
    const navigate = vi.fn();
    renderScreen(navigate);

    await waitFor(() => expect(screen.getByText('Parkhaus Nord')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-lot-card'));
    await waitFor(() => expect(screen.getAllByTestId('buchen-slot').length).toBeGreaterThan(0));
    fireEvent.click(screen.getAllByTestId('buchen-slot')[0]!);
    fireEvent.click(screen.getByRole('button', { name: /Weiter/ }));

    await waitFor(() => expect(screen.getByTestId('buchen-confirm')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-confirm'));

    await waitFor(() => {
      expect(mockCreateBooking).toHaveBeenCalledTimes(1);
      expect(mockToast).toHaveBeenCalledWith('Buchung bestätigt', 'success');
      expect(navigate).toHaveBeenCalledWith('buchungen');
    });
    const payload = mockCreateBooking.mock.calls[0]![0];
    expect(payload.lot_id).toBe('lot-1');
    expect(payload.slot_id).toBe('s-1');
  });

  it('emits error toast when createBooking rejects', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [LOT_OPEN] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: [SLOT_A] });
    mockCreateBooking.mockRejectedValue(new Error('boom'));
    renderScreen();

    await waitFor(() => expect(screen.getByText('Parkhaus Nord')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-lot-card'));
    await waitFor(() => expect(screen.getByTestId('buchen-slot')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-slot'));
    fireEvent.click(screen.getByRole('button', { name: /Weiter/ }));
    await waitFor(() => expect(screen.getByTestId('buchen-confirm')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-confirm'));

    // onError now propagates the thrown Error's message; falls back to 'Buchung fehlgeschlagen' only if empty
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('boom', 'error'));
  });

  it('surfaces query error when getLots responds success:false', async () => {
    mockGetLots.mockResolvedValue({ success: false, data: null, error: { code: 'FORBIDDEN', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('blocks confirm when startDate is cleared and toasts an error without calling createBooking', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [LOT_OPEN] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: [SLOT_A] });
    renderScreen();

    await waitFor(() => expect(screen.getByText('Parkhaus Nord')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-lot-card'));
    await waitFor(() => expect(screen.getByTestId('buchen-slot')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-slot'));

    // Clear the datetime-local input — user empties the field before confirming.
    const startInput = document.getElementById('buchen-start') as HTMLInputElement;
    expect(startInput).toBeTruthy();
    fireEvent.change(startInput, { target: { value: '' } });

    fireEvent.click(screen.getByRole('button', { name: /Weiter/ }));
    await waitFor(() => expect(screen.getByTestId('buchen-confirm')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-confirm'));

    await waitFor(() =>
      expect(mockToast).toHaveBeenCalledWith('Bitte gültige Zeiten angeben', 'error'),
    );
    expect(mockCreateBooking).not.toHaveBeenCalled();
  });

  it('sends calendarEvents-equivalent ISO datetime-local values to createBooking on valid confirm (regression guard)', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [LOT_OPEN] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: [SLOT_A] });
    mockCreateBooking.mockResolvedValue({ success: true, data: { id: 'b-valid' } });
    renderScreen();

    await waitFor(() => expect(screen.getByText('Parkhaus Nord')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-lot-card'));
    await waitFor(() => expect(screen.getByTestId('buchen-slot')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-slot'));
    fireEvent.click(screen.getByRole('button', { name: /Weiter/ }));
    await waitFor(() => expect(screen.getByTestId('buchen-confirm')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-confirm'));

    await waitFor(() => expect(mockCreateBooking).toHaveBeenCalledTimes(1));
    const payload = mockCreateBooking.mock.calls[0]![0];
    // Default start is prefilled → toISOString() must succeed and produce a Z-suffixed ISO string.
    expect(payload.start_time).toMatch(/\dT\d/);
    expect(payload.end_time).toMatch(/\dT\d/);
    expect(new Date(payload.start_time).valueOf()).not.toBeNaN();
    expect(new Date(payload.end_time).valueOf()).not.toBeNaN();
  });

  it('clears stale persisted vehicle id when it no longer matches the loaded vehicles list', async () => {
    // Regression: useState(() => readLastUsed('buchen:vehicle')) can hold a
    // vehicle id that was since deleted server-side. Reconcile it against
    // the vehicles query result, and clear when no match is found, so we
    // never submit a stale vehicle_id to createBooking.
    window.localStorage.setItem('ph-v5-last:buchen:vehicle', 'ghost-vehicle-999');

    mockGetLots.mockResolvedValue({ success: true, data: [LOT_OPEN] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: [SLOT_A] });
    mockGetVehicles.mockResolvedValue({
      success: true,
      data: [{ id: 'v-1', plate: 'B-AA-1', make: 'VW' }],
    });
    mockCreateBooking.mockResolvedValue({ success: true, data: { id: 'b-new' } });
    renderScreen();

    await waitFor(() => expect(screen.getByText('Parkhaus Nord')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-lot-card'));
    await waitFor(() => expect(screen.getByTestId('buchen-slot')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-slot'));

    // The Fahrzeug <select> must reflect the cleared state — not the stale
    // ghost id — once the vehicles query resolves.
    const vehicleSelect = document.getElementById('buchen-vehicle') as HTMLSelectElement;
    await waitFor(() => expect(vehicleSelect).toBeTruthy());
    await waitFor(() => expect(vehicleSelect.value).toBe(''));

    fireEvent.click(screen.getByRole('button', { name: /Weiter/ }));
    await waitFor(() => expect(screen.getByTestId('buchen-confirm')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-confirm'));

    await waitFor(() => expect(mockCreateBooking).toHaveBeenCalledTimes(1));
    const payload = mockCreateBooking.mock.calls[0]![0];
    expect(payload.vehicle_id).toBeUndefined();
  });

  it('does not leave a phantom booking in cache when createBooking fails and ["buchungen"] was never fetched', async () => {
    // Regression: onMutate inserts an optimistic booking even when
    // queryClient.getQueryData(['buchungen']) is undefined. onError only
    // restores when ctx.previous is truthy, so a first-time failure leaves
    // a phantom optimistic row cached until a later refetch replaces it.
    // Fix: skip the optimistic insert when no prior cache exists; the cache
    // must remain undefined after the failure.
    mockGetLots.mockResolvedValue({ success: true, data: [LOT_OPEN] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: [SLOT_A] });
    mockCreateBooking.mockRejectedValue(new Error('boom'));

    // Capture the QueryClient so we can assert its cache state after failure.
    let captured: QueryClient | null = null;
    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    captured = qc;
    render(
      <QueryClientProvider client={qc}>
        <BuchenV5 navigate={vi.fn()} />
      </QueryClientProvider>
    );

    await waitFor(() => expect(screen.getByText('Parkhaus Nord')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-lot-card'));
    await waitFor(() => expect(screen.getByTestId('buchen-slot')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-slot'));
    fireEvent.click(screen.getByRole('button', { name: /Weiter/ }));
    await waitFor(() => expect(screen.getByTestId('buchen-confirm')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-confirm'));

    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('boom', 'error'));

    // Cache must either be undefined (never fetched) or empty — NOT contain
    // the phantom optimistic row.
    const bookings = captured!.getQueryData(['buchungen']);
    if (bookings !== undefined) {
      expect(Array.isArray(bookings)).toBe(true);
      expect((bookings as unknown[]).length).toBe(0);
    }
  });

  it('calls onError (no success toast) when createBooking responds success:false with INSUFFICIENT_CREDITS', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [LOT_OPEN] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: [SLOT_A] });
    mockCreateBooking.mockResolvedValue({ success: false, data: null, error: { code: 'INSUFFICIENT_CREDITS', message: 'no credits' } });
    renderScreen();

    await waitFor(() => expect(screen.getByText('Parkhaus Nord')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-lot-card'));
    await waitFor(() => expect(screen.getByTestId('buchen-slot')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-slot'));
    fireEvent.click(screen.getByRole('button', { name: /Weiter/ }));
    await waitFor(() => expect(screen.getByTestId('buchen-confirm')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('buchen-confirm'));

    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('Nicht genug Credits', 'error'));
    expect(mockToast).not.toHaveBeenCalledWith('Buchung bestätigt', 'success');
  });
});
