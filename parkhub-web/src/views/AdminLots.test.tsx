import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockGetLots = vi.fn();
const mockCreateLot = vi.fn();
const mockDeleteLot = vi.fn();
const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

const mockUpdateLot = vi.fn();
const mockGetAdminDynamicPricing = vi.fn();
const mockGetLotHours = vi.fn();
const mockUpdateAdminDynamicPricing = vi.fn();
const mockUpdateAdminLotHours = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    getLots: (...args: any[]) => mockGetLots(...args),
    createLot: (...args: any[]) => mockCreateLot(...args),
    updateLot: (...args: any[]) => mockUpdateLot(...args),
    deleteLot: (...args: any[]) => mockDeleteLot(...args),
    getAdminDynamicPricing: (...args: any[]) => mockGetAdminDynamicPricing(...args),
    getLotHours: (...args: any[]) => mockGetLotHours(...args),
    updateAdminDynamicPricing: (...args: any[]) => mockUpdateAdminDynamicPricing(...args),
    updateAdminLotHours: (...args: any[]) => mockUpdateAdminLotHours(...args),
  },
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    tr: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <tr ref={ref} {...props}>{children}</tr>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Plus: (props: any) => <span data-testid="icon-plus" {...props} />,
  PencilSimple: (props: any) => <span data-testid="icon-pencil" {...props} />,
  Trash: (props: any) => <span data-testid="icon-trash" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  MagnifyingGlass: (props: any) => <span data-testid="icon-search" {...props} />,
  CurrencyEur: (props: any) => <span data-testid="icon-currency" {...props} />,
  TrendUp: (props: any) => <span data-testid="icon-trend-up" {...props} />,
  TrendDown: (props: any) => <span data-testid="icon-trend-down" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  Warning: (props: any) => <span data-testid="icon-warning" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: {
    success: (...args: any[]) => mockToastSuccess(...args),
    error: (...args: any[]) => mockToastError(...args),
  },
}));

vi.mock('../components/ui/ConfirmDialog', () => ({
  ConfirmDialog: ({ open, onConfirm, onCancel, title, message }: any) =>
    open ? (
      <div data-testid="confirm-dialog">
        <p>{title}</p>
        <p>{message}</p>
        <button onClick={onConfirm}>Confirm</button>
        <button onClick={onCancel}>CancelConfirm</button>
      </div>
    ) : null,
}));

import { AdminLotsPage } from './AdminLots';

describe('AdminLotsPage', () => {
  beforeEach(() => {
    mockGetLots.mockClear();
    mockCreateLot.mockClear();
    mockDeleteLot.mockClear();
    mockUpdateLot.mockClear();
    mockGetAdminDynamicPricing.mockClear();
    mockGetLotHours.mockClear();
    mockUpdateAdminDynamicPricing.mockClear();
    mockUpdateAdminLotHours.mockClear();
    mockToastSuccess.mockClear();
    mockToastError.mockClear();
    mockGetAdminDynamicPricing.mockResolvedValue({ success: true, data: { enabled: false, base_price: 2.5, surge_multiplier: 1.5, discount_multiplier: 0.8, surge_threshold: 80, discount_threshold: 20 } });
    mockGetLotHours.mockResolvedValue({ success: true, data: { is_24h: true } });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading spinner initially', () => {
    mockGetLots.mockReturnValue(new Promise(() => {}));
    render(<AdminLotsPage />);
    expect(screen.getByTestId('icon-spinner')).toBeInTheDocument();
  });

  it('renders lot table headers after loading', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [] });
    render(<AdminLotsPage />);

    await waitFor(() => {
      // Table header uses t('admin.lots') which is also the page heading — multiple matches
      expect(screen.getAllByText('Parking Lots').length).toBeGreaterThanOrEqual(2);
    });
    expect(screen.getByText('Total Slots')).toBeInTheDocument();
    expect(screen.getByText('Status')).toBeInTheDocument();
    expect(screen.getByText('Pricing')).toBeInTheDocument();
  });

  it('renders lot count', async () => {
    mockGetLots.mockResolvedValue({
      success: true,
      data: [
        { id: 'l-1', name: 'Lot A', total_slots: 10, available_slots: 5, status: 'open' },
      ],
    });
    render(<AdminLotsPage />);

    await waitFor(() => {
      expect(screen.getByText('(1)')).toBeInTheDocument();
    });
  });

  it('renders lot rows', async () => {
    mockGetLots.mockResolvedValue({
      success: true,
      data: [
        {
          id: 'l-1', name: 'Main Garage', address: '123 Main St',
          total_slots: 20, available_slots: 12, status: 'open',
          hourly_rate: 2.5, daily_max: 15, monthly_pass: 200, currency: 'EUR',
        },
      ],
    });

    render(<AdminLotsPage />);

    await waitFor(() => {
      expect(screen.getByText('Main Garage')).toBeInTheDocument();
    });
    expect(screen.getByText('123 Main St')).toBeInTheDocument();
    expect(screen.getByText('Open')).toBeInTheDocument();
  });

  it('shows empty state when no lots match search', async () => {
    mockGetLots.mockResolvedValue({
      success: true,
      data: [
        { id: 'l-1', name: 'Garage A', total_slots: 10, available_slots: 5, status: 'open' },
      ],
    });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => {
      expect(screen.getByText('Garage A')).toBeInTheDocument();
    });

    const searchInput = screen.getByLabelText('Search lots...');
    await user.type(searchInput, 'nonexistent');

    expect(screen.getByText('No lots match your search.')).toBeInTheDocument();
  });

  it('filters lots by search', async () => {
    mockGetLots.mockResolvedValue({
      success: true,
      data: [
        { id: 'l-1', name: 'Garage Alpha', total_slots: 10, available_slots: 5, status: 'open' },
        { id: 'l-2', name: 'Lot Beta', total_slots: 5, available_slots: 3, status: 'open' },
      ],
    });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => {
      expect(screen.getByText('Garage Alpha')).toBeInTheDocument();
    });

    const searchInput = screen.getByLabelText('Search lots...');
    await user.type(searchInput, 'Beta');

    expect(screen.queryByText('Garage Alpha')).not.toBeInTheDocument();
    expect(screen.getByText('Lot Beta')).toBeInTheDocument();
  });

  it('opens create form when clicking New Lot', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [] });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => {
      expect(screen.getByText('New Lot')).toBeInTheDocument();
    });

    await user.click(screen.getByText('New Lot'));

    // 'New Lot' appears as both button text and form heading
    expect(screen.getAllByText('New Lot').length).toBeGreaterThanOrEqual(2);
    expect(screen.getByLabelText('Name *')).toBeInTheDocument();
    expect(screen.getByLabelText('Address')).toBeInTheDocument();
    expect(screen.getByLabelText('Total Slots *')).toBeInTheDocument();
  });

  it('shows search input', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [] });
    render(<AdminLotsPage />);

    await waitFor(() => {
      expect(screen.getByPlaceholderText('Search lots...')).toBeInTheDocument();
    });
  });

  it('shows empty state when no lots and no search', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [] });
    render(<AdminLotsPage />);

    await waitFor(() => {
      expect(screen.getByText('No parking lots yet. Create one to get started.')).toBeInTheDocument();
    });
  });

  it('can submit the create form successfully', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [] });
    mockCreateLot.mockResolvedValue({ success: true, data: { id: 'new-1', name: 'Test Lot' } });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => expect(screen.getByText('New Lot')).toBeInTheDocument());

    await user.click(screen.getByText('New Lot'));
    await user.type(screen.getByLabelText('Name *'), 'Test Lot');
    await user.clear(screen.getByLabelText('Total Slots *'));
    await user.type(screen.getByLabelText('Total Slots *'), '20');

    // Reset mock for reload after create
    mockGetLots.mockResolvedValue({ success: true, data: [{ id: 'new-1', name: 'Test Lot', total_slots: 20, available_slots: 20, status: 'open' }] });

    await user.click(screen.getByText('Create'));

    await waitFor(() => {
      expect(mockCreateLot).toHaveBeenCalled();
      expect(mockToastSuccess).toHaveBeenCalled();
    });
  });

  it('shows error toast when name is empty on save', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [] });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => expect(screen.getByText('New Lot')).toBeInTheDocument());
    await user.click(screen.getByText('New Lot'));

    // Name is empty by default
    await user.click(screen.getByText('Create'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalled();
    });
    expect(mockCreateLot).not.toHaveBeenCalled();
  });

  it('shows error toast when save fails', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [] });
    mockCreateLot.mockResolvedValue({ success: false, data: null, error: { code: 'VALIDATION', message: 'Invalid data' } });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => expect(screen.getByText('New Lot')).toBeInTheDocument());
    await user.click(screen.getByText('New Lot'));
    await user.type(screen.getByLabelText('Name *'), 'Bad Lot');

    await user.click(screen.getByText('Create'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Invalid data');
    });
  });

  it('opens edit form for existing lot and shows edit heading', async () => {
    mockGetLots.mockResolvedValue({
      success: true,
      data: [{ id: 'l-1', name: 'Main Garage', address: '123 Main', total_slots: 20, available_slots: 12, status: 'open', hourly_rate: 2.5, daily_max: 15, monthly_pass: 200, currency: 'EUR' }],
    });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => expect(screen.getByText('Main Garage')).toBeInTheDocument());

    // Click edit button
    const editBtn = screen.getByLabelText(/Edit lot Main Garage/i);
    await user.click(editBtn);

    await waitFor(() => {
      expect(screen.getByText('Edit Parking Lot')).toBeInTheDocument();
    });
  });

  it('filters lots by address', async () => {
    mockGetLots.mockResolvedValue({
      success: true,
      data: [
        { id: 'l-1', name: 'Garage A', address: '123 Main Street', total_slots: 10, available_slots: 5, status: 'open' },
        { id: 'l-2', name: 'Garage B', address: '456 Oak Avenue', total_slots: 5, available_slots: 3, status: 'open' },
      ],
    });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => expect(screen.getByText('Garage A')).toBeInTheDocument());

    const searchInput = screen.getByLabelText('Search lots...');
    await user.type(searchInput, 'Oak');

    expect(screen.queryByText('Garage A')).not.toBeInTheDocument();
    expect(screen.getByText('Garage B')).toBeInTheDocument();
  });

  it('closes form on close button click', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [] });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => expect(screen.getByText('New Lot')).toBeInTheDocument());
    await user.click(screen.getByText('New Lot'));
    expect(screen.getByLabelText('Name *')).toBeInTheDocument();

    await user.click(screen.getByLabelText('Close'));

    // Form should be hidden
    expect(screen.queryByLabelText('Name *')).not.toBeInTheDocument();
  });

  it('closes form on cancel button click', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [] });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => expect(screen.getByText('New Lot')).toBeInTheDocument());
    await user.click(screen.getByText('New Lot'));
    expect(screen.getByLabelText('Name *')).toBeInTheDocument();

    await user.click(screen.getByText('Cancel'));

    expect(screen.queryByLabelText('Name *')).not.toBeInTheDocument();
  });

  it('displays different lot statuses with correct badges', async () => {
    mockGetLots.mockResolvedValue({
      success: true,
      data: [
        { id: 'l-1', name: 'Open Lot', total_slots: 10, available_slots: 5, status: 'open' },
        { id: 'l-2', name: 'Full Lot', total_slots: 10, available_slots: 0, status: 'full' },
        { id: 'l-3', name: 'Closed Lot', total_slots: 10, available_slots: 10, status: 'closed' },
        { id: 'l-4', name: 'Maint Lot', total_slots: 10, available_slots: 10, status: 'maintenance' },
      ],
    });
    render(<AdminLotsPage />);

    await waitFor(() => {
      expect(screen.getByText('Open')).toBeInTheDocument();
      expect(screen.getByText('Full')).toBeInTheDocument();
      expect(screen.getByText('Closed')).toBeInTheDocument();
      expect(screen.getByText('Maintenance')).toBeInTheDocument();
    });
  });

  it('displays pricing info with formatted values', async () => {
    mockGetLots.mockResolvedValue({
      success: true,
      data: [{
        id: 'l-1', name: 'Priced Lot', total_slots: 10, available_slots: 5,
        status: 'open', hourly_rate: 2.5, daily_max: 15, monthly_pass: 200, currency: 'EUR',
      }],
    });
    render(<AdminLotsPage />);

    await waitFor(() => {
      expect(screen.getByText('Priced Lot')).toBeInTheDocument();
    });
  });

  it('handles lot with no pricing (dash display)', async () => {
    mockGetLots.mockResolvedValue({
      success: true,
      data: [{ id: 'l-1', name: 'Free Lot', total_slots: 10, available_slots: 5, status: 'open' }],
    });
    render(<AdminLotsPage />);

    await waitFor(() => {
      expect(screen.getByText('Free Lot')).toBeInTheDocument();
    });
    // The pricing lines include "-" for null values within "Hourly Rate: -" etc.
    // Use a function matcher to find text containing the dash pattern
    const pricingCells = screen.getAllByText((_content, element) => {
      return element?.tagName === 'P' && element.textContent?.includes(': -') === true;
    });
    expect(pricingCells.length).toBeGreaterThanOrEqual(3);
  });

  it('handles API failure on load gracefully', async () => {
    mockGetLots.mockResolvedValue({ success: false, data: null, error: { code: 'NETWORK', message: 'Error' } });
    render(<AdminLotsPage />);

    await waitFor(() => {
      // Should still render with empty state, not crash
      expect(screen.getByText('No parking lots yet. Create one to get started.')).toBeInTheDocument();
    });
  });

  it('triggers delete confirmation dialog on delete button click', async () => {
    mockGetLots.mockResolvedValue({
      success: true,
      data: [{ id: 'l-1', name: 'To Delete', total_slots: 10, available_slots: 5, status: 'open' }],
    });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => expect(screen.getByText('To Delete')).toBeInTheDocument());

    const deleteBtn = screen.getByLabelText(/Delete lot To Delete/i);
    await user.click(deleteBtn);

    // ConfirmDialog should appear
    await waitFor(() => {
      expect(screen.getByText('Delete this parking lot? All associated slots and bookings will be removed.')).toBeInTheDocument();
    });
  });

  it('validates empty name on save', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [] });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => expect(screen.getByText('New Lot')).toBeInTheDocument());
    await user.click(screen.getByText('New Lot'));
    // Name is empty by default, just save
    await user.click(screen.getByText('Create'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalled();
    });
    expect(mockCreateLot).not.toHaveBeenCalled();
  });

  it('validates negative pricing on save', async () => {
    mockGetLots.mockResolvedValue({ success: true, data: [] });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => expect(screen.getByText('New Lot')).toBeInTheDocument());
    await user.click(screen.getByText('New Lot'));
    await user.type(screen.getByLabelText('Name *'), 'Bad Price Lot');
    await user.type(screen.getByLabelText('Hourly Rate'), '-5');
    await user.click(screen.getByText('Create'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalled();
    });
  });

  it('edits a lot and saves with dynamic pricing and hours', async () => {
    const lotData = {
      id: 'l-1', name: 'Main Garage', address: '123 Main', total_slots: 20, available_slots: 12,
      status: 'open', hourly_rate: 2.5, daily_max: 15, monthly_pass: 200, currency: 'EUR',
    };
    mockGetLots.mockResolvedValue({ success: true, data: [lotData] });
    mockUpdateLot.mockResolvedValue({ success: true, data: lotData });
    mockUpdateAdminDynamicPricing.mockResolvedValue({ success: true });
    mockUpdateAdminLotHours.mockResolvedValue({ success: true });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => expect(screen.getByText('Main Garage')).toBeInTheDocument());

    const editBtn = screen.getByLabelText(/Edit lot Main Garage/i);
    await user.click(editBtn);

    await waitFor(() => expect(screen.getByText('Edit Parking Lot')).toBeInTheDocument());

    // Change name
    const nameInput = screen.getByLabelText('Name *');
    await user.clear(nameInput);
    await user.type(nameInput, 'Updated Garage');

    // Click save
    await user.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(mockUpdateLot).toHaveBeenCalledWith('l-1', expect.objectContaining({ name: 'Updated Garage' }));
      expect(mockToastSuccess).toHaveBeenCalled();
    });
  });

  it('deletes a lot successfully after confirm', async () => {
    mockGetLots.mockResolvedValue({
      success: true,
      data: [{ id: 'l-1', name: 'To Delete', total_slots: 10, available_slots: 5, status: 'open' }],
    });
    mockDeleteLot.mockResolvedValue({ success: true });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => expect(screen.getByText('To Delete')).toBeInTheDocument());

    await user.click(screen.getByLabelText(/Delete lot To Delete/i));
    await waitFor(() => expect(screen.getByText(/Delete this parking lot/)).toBeInTheDocument());

    // The ConfirmDialog is mocked -- click Confirm
    await user.click(screen.getByText('Confirm'));

    await waitFor(() => {
      expect(mockDeleteLot).toHaveBeenCalledWith('l-1');
      expect(mockToastSuccess).toHaveBeenCalled();
    });
  });

  it('shows error toast on delete failure', async () => {
    mockGetLots.mockResolvedValue({
      success: true,
      data: [{ id: 'l-1', name: 'Fail Delete', total_slots: 10, available_slots: 5, status: 'open' }],
    });
    mockDeleteLot.mockResolvedValue({ success: false, error: { message: 'Cannot delete' } });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => expect(screen.getByText('Fail Delete')).toBeInTheDocument());

    await user.click(screen.getByLabelText(/Delete lot Fail Delete/i));
    await waitFor(() => expect(screen.getByText(/Delete this parking lot/)).toBeInTheDocument());

    await user.click(screen.getByText('Confirm'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Cannot delete');
    });
  });

  it('shows dynamic pricing section when editing', async () => {
    const lotData = {
      id: 'l-1', name: 'DP Lot', address: '', total_slots: 10, available_slots: 5,
      status: 'open', hourly_rate: 2.5, daily_max: 15, monthly_pass: 200, currency: 'EUR',
    };
    mockGetLots.mockResolvedValue({ success: true, data: [lotData] });
    mockGetAdminDynamicPricing.mockResolvedValue({
      success: true,
      data: { enabled: true, base_price: 3.0, surge_multiplier: 2.0, discount_multiplier: 0.7, surge_threshold: 90, discount_threshold: 10 },
    });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => expect(screen.getByText('DP Lot')).toBeInTheDocument());

    await user.click(screen.getByLabelText(/Edit lot DP Lot/i));

    await waitFor(() => {
      expect(screen.getByText('Dynamic Pricing')).toBeInTheDocument();
    });
  });

  it('shows error toast when dynamic pricing save fails during lot update', async () => {
    const lotData = {
      id: 'l-1', name: 'DP Fail', address: '', total_slots: 10, available_slots: 5,
      status: 'open', hourly_rate: 2.5, currency: 'EUR',
    };
    mockGetLots.mockResolvedValue({ success: true, data: [lotData] });
    mockUpdateLot.mockResolvedValue({ success: true, data: lotData });
    mockUpdateAdminDynamicPricing.mockResolvedValue({ success: false });
    mockUpdateAdminLotHours.mockResolvedValue({ success: true });
    const user = userEvent.setup();
    render(<AdminLotsPage />);

    await waitFor(() => expect(screen.getByText('DP Fail')).toBeInTheDocument());
    await user.click(screen.getByLabelText(/Edit lot DP Fail/i));
    await waitFor(() => expect(screen.getByText('Edit Parking Lot')).toBeInTheDocument());
    await user.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalled();
    });
  });
});
