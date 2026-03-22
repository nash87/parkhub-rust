import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockNavigate = vi.fn();
const mockGetLots = vi.fn();
const mockGetLotSlots = vi.fn();
const mockGetVehicles = vi.fn();
const mockCreateBooking = vi.fn();
const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('react-router-dom', () => ({
  useNavigate: () => mockNavigate,
}));

vi.mock('../api/client', () => ({
  api: {
    getLots: (...args: any[]) => mockGetLots(...args),
    getLotSlots: (...args: any[]) => mockGetLotSlots(...args),
    getVehicles: (...args: any[]) => mockGetVehicles(...args),
    createBooking: (...args: any[]) => mockCreateBooking(...args),
    getDynamicPrice: vi.fn().mockResolvedValue({ price: 5.0, multiplier: 1.0, reason: 'normal' }),
    getOperatingHours: vi.fn().mockResolvedValue({ hours: [] }),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => {
      const map: Record<string, string> = {
        'book.title': 'Book a Spot',
        'book.step1Label': 'Choose a parking lot',
        'book.step2Label': 'Pick a slot & time',
        'book.step3Label': 'Review & confirm',
        'book.stepName1': 'Lot',
        'book.stepName2': 'Slot',
        'book.stepName3': 'Confirm',
        'book.noLots': 'No lots available',
        'book.availableSlots': `${opts?.count ?? 0} / ${opts?.total ?? 0} available`,
        'book.full': 'Full',
        'book.selectSlot': 'Select a slot',
        'book.noAvailableSlots': 'No available slots',
        'book.startTime': 'Start time',
        'book.duration': 'Duration',
        'book.vehicle': 'Vehicle',
        'book.noVehicle': 'No vehicle',
        'book.continue': 'Continue',
        'book.lot': 'Lot',
        'book.slot': 'Slot',
        'book.from': 'From',
        'book.to': 'To',
        'book.estimatedCost': 'Estimated cost',
        'book.confirm': 'Confirm Booking',
        'book.confirming': 'Confirming...',
        'book.success': 'Booking confirmed!',
        'common.error': 'Something went wrong',
        'bookings.insufficientCredits': 'Insufficient credits',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, variants, custom, layoutId, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    button: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, variants, custom, layoutId, ...props }: any, ref: any) => (
      <button ref={ref} {...props}>{children}</button>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  ArrowLeft: (props: any) => <span data-testid="icon-arrow-left" {...props} />,
  MapPin: (props: any) => <span data-testid="icon-pin" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  Car: (props: any) => <span data-testid="icon-car" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  Lightning: (props: any) => <span data-testid="icon-lightning" {...props} />,
  Wheelchair: (props: any) => <span data-testid="icon-wheelchair" {...props} />,
  Motorcycle: (props: any) => <span data-testid="icon-motorcycle" {...props} />,
  Star: (props: any) => <span data-testid="icon-star" {...props} />,
}));

vi.mock('../components/Skeleton', () => ({
  SkeletonCard: ({ height }: any) => <div data-testid="skeleton-card" className={height}>Loading...</div>,
}));

vi.mock('react-hot-toast', () => ({
  default: { success: (...args: any[]) => mockToastSuccess(...args), error: (...args: any[]) => mockToastError(...args) },
}));

import { BookPage } from './Book';
import type { ParkingLot, ParkingSlot, Vehicle } from '../api/client';

function makeLot(overrides: Partial<ParkingLot> = {}): ParkingLot {
  return {
    id: 'lot-1',
    name: 'Garage Alpha',
    address: '123 Main St',
    total_slots: 20,
    available_slots: 8,
    status: 'open',
    hourly_rate: 2.5,
    currency: '€',
    ...overrides,
  };
}

function makeSlot(overrides: Partial<ParkingSlot> = {}): ParkingSlot {
  return {
    id: 'slot-1',
    lot_id: 'lot-1',
    slot_number: 'A1',
    status: 'available',
    ...overrides,
  };
}

function makeVehicle(overrides: Partial<Vehicle> = {}): Vehicle {
  return {
    id: 'v-1',
    plate: 'M-AB-123',
    make: 'BMW',
    model: '320i',
    is_default: true,
    ...overrides,
  };
}

describe('BookPage', () => {
  beforeEach(() => {
    mockNavigate.mockClear();
    mockGetLots.mockClear();
    mockGetLotSlots.mockClear();
    mockGetVehicles.mockClear();
    mockCreateBooking.mockClear();
    mockToastSuccess.mockClear();
    mockToastError.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders loading skeletons initially', () => {
    mockGetLots.mockReturnValue(new Promise(() => {}));
    mockGetVehicles.mockReturnValue(new Promise(() => {}));

    render(<BookPage />);

    expect(screen.getAllByTestId('skeleton-card')).toHaveLength(3);
  });

  it('shows lot cards after loading', async () => {
    const lots = [makeLot(), makeLot({ id: 'lot-2', name: 'Garage Beta' })];
    mockGetLots.mockResolvedValue({ success: true, data: lots });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });

    render(<BookPage />);

    await waitFor(() => {
      expect(screen.getByText('Garage Alpha')).toBeInTheDocument();
    });
    expect(screen.getByText('Garage Beta')).toBeInTheDocument();
    // Both lots have addresses displayed
    expect(screen.getAllByText('123 Main St')).toHaveLength(2);
  });

  it('shows empty state when no open lots', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });

    render(<BookPage />);

    await waitFor(() => {
      expect(screen.getByText('No lots available')).toBeInTheDocument();
    });
  });

  it('filters out closed lots', async () => {
    const lots = [makeLot({ status: 'closed', name: 'Closed Lot' }), makeLot({ name: 'Open Lot' })];
    mockGetLots.mockResolvedValue({ success: true, data: lots });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });

    render(<BookPage />);

    await waitFor(() => {
      expect(screen.getByText('Open Lot')).toBeInTheDocument();
    });
    expect(screen.queryByText('Closed Lot')).not.toBeInTheDocument();
  });

  it('clicking a lot advances to step 2 and loads slots', async () => {
    const user = userEvent.setup();
    const lot = makeLot();
    const slots = [makeSlot(), makeSlot({ id: 'slot-2', slot_number: 'A2', status: 'occupied' })];

    mockGetLots.mockResolvedValue({ success: true, data: [lot] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: slots });

    render(<BookPage />);

    await waitFor(() => {
      expect(screen.getByText('Garage Alpha')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Garage Alpha'));

    await waitFor(() => {
      expect(screen.getByText('Select a slot')).toBeInTheDocument();
    });
    expect(mockGetLotSlots).toHaveBeenCalledWith('lot-1');
    expect(screen.getByText('A1')).toBeInTheDocument();
    expect(screen.getByText('A2')).toBeInTheDocument();
  });

  it('shows slot grid in step 2 with available and occupied slots', async () => {
    const user = userEvent.setup();
    const lot = makeLot();
    const slots = [
      makeSlot({ id: 's1', slot_number: 'B1', status: 'available' }),
      makeSlot({ id: 's2', slot_number: 'B2', status: 'occupied' }),
      makeSlot({ id: 's3', slot_number: 'B3', status: 'available' }),
    ];

    mockGetLots.mockResolvedValue({ success: true, data: [lot] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: slots });

    render(<BookPage />);

    await waitFor(() => {
      expect(screen.getByText('Garage Alpha')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Garage Alpha'));

    await waitFor(() => {
      expect(screen.getByText('B1')).toBeInTheDocument();
    });

    // Occupied slot button should be disabled
    const occupiedBtn = screen.getByText('B2').closest('button');
    expect(occupiedBtn).toBeDisabled();

    // Available slot buttons should be enabled
    const availableBtn = screen.getByText('B1').closest('button');
    expect(availableBtn).toBeEnabled();
  });

  it('duration buttons work', async () => {
    const user = userEvent.setup();
    const lot = makeLot();

    mockGetLots.mockResolvedValue({ success: true, data: [lot] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: [makeSlot()] });

    render(<BookPage />);

    await waitFor(() => {
      expect(screen.getByText('Garage Alpha')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Garage Alpha'));

    await waitFor(() => {
      expect(screen.getByText('1h')).toBeInTheDocument();
    });

    // All four duration buttons should be visible
    expect(screen.getByText('1h')).toBeInTheDocument();
    expect(screen.getByText('2h')).toBeInTheDocument();
    expect(screen.getByText('4h')).toBeInTheDocument();
    expect(screen.getByText('8h')).toBeInTheDocument();

    // Click 4h duration
    await user.click(screen.getByText('4h'));

    // 4h button should now have the selected style (bg-teal-600)
    const btn4h = screen.getByText('4h').closest('button');
    expect(btn4h?.className).toContain('bg-teal-600');
  });

  it('back button returns from step 2 to step 1', async () => {
    const user = userEvent.setup();
    const lot = makeLot();

    mockGetLots.mockResolvedValue({ success: true, data: [lot] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: [makeSlot()] });

    render(<BookPage />);

    await waitFor(() => {
      expect(screen.getByText('Garage Alpha')).toBeInTheDocument();
    });

    // Go to step 2
    await user.click(screen.getByText('Garage Alpha'));

    await waitFor(() => {
      expect(screen.getByText('Select a slot')).toBeInTheDocument();
    });

    // Click back button
    const backBtn = screen.getByTestId('icon-arrow-left').closest('button');
    await user.click(backBtn!);

    // Should be back on step 1 showing lots
    await waitFor(() => {
      expect(screen.getByText('Garage Alpha')).toBeInTheDocument();
    });
    expect(screen.getByText('Choose a parking lot')).toBeInTheDocument();
  });

  it('continue button is disabled until a slot is selected', async () => {
    const user = userEvent.setup();
    const lot = makeLot();

    mockGetLots.mockResolvedValue({ success: true, data: [lot] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: [makeSlot()] });

    render(<BookPage />);

    await waitFor(() => {
      expect(screen.getByText('Garage Alpha')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Garage Alpha'));

    await waitFor(() => {
      expect(screen.getByText('Continue')).toBeInTheDocument();
    });

    // Continue disabled before selecting slot
    expect(screen.getByText('Continue').closest('button')).toBeDisabled();

    // Select a slot
    await user.click(screen.getByText('A1'));

    // Continue now enabled
    expect(screen.getByText('Continue').closest('button')).toBeEnabled();
  });

  it('confirm button calls createBooking API and navigates on success', async () => {
    const user = userEvent.setup();
    const lot = makeLot();
    const slot = makeSlot();
    const vehicle = makeVehicle();

    mockGetLots.mockResolvedValue({ success: true, data: [lot] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [vehicle] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: [slot] });
    mockCreateBooking.mockResolvedValue({ success: true, data: { id: 'b1' } });

    render(<BookPage />);

    // Step 1: select lot
    await waitFor(() => {
      expect(screen.getByText('Garage Alpha')).toBeInTheDocument();
    });
    await user.click(screen.getByText('Garage Alpha'));

    // Step 2: select slot and continue
    await waitFor(() => {
      expect(screen.getByText('A1')).toBeInTheDocument();
    });
    await user.click(screen.getByText('A1'));
    await user.click(screen.getByText('Continue'));

    // Step 3: confirm
    await waitFor(() => {
      expect(screen.getByText('Confirm Booking')).toBeInTheDocument();
    });

    // Should show summary
    expect(screen.getByText('Garage Alpha')).toBeInTheDocument();
    expect(screen.getByText('A1')).toBeInTheDocument();

    await user.click(screen.getByText('Confirm Booking'));

    await waitFor(() => {
      expect(mockCreateBooking).toHaveBeenCalledWith(
        expect.objectContaining({
          lot_id: 'lot-1',
          slot_id: 'slot-1',
          vehicle_id: 'v-1',
        })
      );
    });

    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith('Booking confirmed!');
    });
    expect(mockNavigate).toHaveBeenCalledWith('/bookings');
  });

  it('shows error toast when booking fails', async () => {
    const user = userEvent.setup();
    const lot = makeLot();
    const slot = makeSlot();

    mockGetLots.mockResolvedValue({ success: true, data: [lot] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: [slot] });
    mockCreateBooking.mockResolvedValue({
      success: false,
      data: null,
      error: { code: 'INSUFFICIENT_CREDITS', message: 'Not enough credits' },
    });

    render(<BookPage />);

    await waitFor(() => {
      expect(screen.getByText('Garage Alpha')).toBeInTheDocument();
    });
    await user.click(screen.getByText('Garage Alpha'));

    await waitFor(() => {
      expect(screen.getByText('A1')).toBeInTheDocument();
    });
    await user.click(screen.getByText('A1'));
    await user.click(screen.getByText('Continue'));

    await waitFor(() => {
      expect(screen.getByText('Confirm Booking')).toBeInTheDocument();
    });
    await user.click(screen.getByText('Confirm Booking'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Insufficient credits');
    });
  });

  it('shows step indicator with three steps', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [makeLot()] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });

    render(<BookPage />);

    await waitFor(() => {
      expect(screen.getByText('Lot')).toBeInTheDocument();
    });
    expect(screen.getByText('Slot')).toBeInTheDocument();
    expect(screen.getByText('Confirm')).toBeInTheDocument();
  });

  it('selects default vehicle automatically', async () => {
    const user = userEvent.setup();
    const lot = makeLot();
    const slot = makeSlot();
    const defaultVehicle = makeVehicle({ id: 'v-default', plate: 'DEF-123', is_default: true });
    const otherVehicle = makeVehicle({ id: 'v-other', plate: 'OTH-456', is_default: false });

    mockGetLots.mockResolvedValue({ success: true, data: [lot] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [otherVehicle, defaultVehicle] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: [slot] });

    render(<BookPage />);

    await waitFor(() => {
      expect(screen.getByText('Garage Alpha')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Garage Alpha'));

    await waitFor(() => {
      expect(screen.getByText('Select a slot')).toBeInTheDocument();
    });

    // Vehicle dropdown should show the default vehicle selected
    const select = screen.getByRole('combobox') as HTMLSelectElement;
    expect(select.value).toBe('v-default');
  });

  it('shows hourly rate on lot cards', async () => {
    mockGetLots.mockResolvedValue({
      success: true,
      data: [makeLot({ hourly_rate: 3.5, currency: '€' })],
    });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });

    render(<BookPage />);

    await waitFor(() => {
      expect(screen.getByText('€3.50/h')).toBeInTheDocument();
    });
  });

  it('back button from step 3 returns to step 2', async () => {
    const user = userEvent.setup();
    const lot = makeLot();
    const slot = makeSlot();

    mockGetLots.mockResolvedValue({ success: true, data: [lot] });
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    mockGetLotSlots.mockResolvedValue({ success: true, data: [slot] });

    render(<BookPage />);

    // Step 1 → 2
    await waitFor(() => {
      expect(screen.getByText('Garage Alpha')).toBeInTheDocument();
    });
    await user.click(screen.getByText('Garage Alpha'));

    // Step 2 → 3
    await waitFor(() => {
      expect(screen.getByText('A1')).toBeInTheDocument();
    });
    await user.click(screen.getByText('A1'));
    await user.click(screen.getByText('Continue'));

    await waitFor(() => {
      expect(screen.getByText('Confirm Booking')).toBeInTheDocument();
    });

    // Step 3 → 2
    const backBtn = screen.getByTestId('icon-arrow-left').closest('button');
    await user.click(backBtn!);

    await waitFor(() => {
      expect(screen.getByText('Select a slot')).toBeInTheDocument();
    });
  });
});
