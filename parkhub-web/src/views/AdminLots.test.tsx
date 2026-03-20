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

vi.mock('../api/client', () => ({
  api: {
    getLots: (...args: any[]) => mockGetLots(...args),
    createLot: (...args: any[]) => mockCreateLot(...args),
    updateLot: vi.fn(),
    deleteLot: (...args: any[]) => mockDeleteLot(...args),
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
}));

vi.mock('react-hot-toast', () => ({
  default: {
    success: (...args: any[]) => mockToastSuccess(...args),
    error: (...args: any[]) => mockToastError(...args),
  },
}));

import { AdminLotsPage } from './AdminLots';

describe('AdminLotsPage', () => {
  beforeEach(() => {
    mockGetLots.mockClear();
    mockCreateLot.mockClear();
    mockDeleteLot.mockClear();
    mockToastSuccess.mockClear();
    mockToastError.mockClear();
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
      expect(screen.getByText('Parking Lots')).toBeInTheDocument();
    });
    expect(screen.getByText('Lot')).toBeInTheDocument();
    expect(screen.getByText('Slots')).toBeInTheDocument();
    expect(screen.getByText('Status')).toBeInTheDocument();
    expect(screen.getByText('Pricing')).toBeInTheDocument();
    expect(screen.getByText('Actions')).toBeInTheDocument();
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

    const searchInput = screen.getByLabelText('Search parking lots');
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

    const searchInput = screen.getByLabelText('Search parking lots');
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

    expect(screen.getByText('New Parking Lot')).toBeInTheDocument();
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
});
