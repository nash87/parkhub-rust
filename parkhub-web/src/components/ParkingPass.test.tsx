import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';

// ── Mocks ──

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'pass.title': 'Parking Pass',
        'pass.lot': 'Parking Lot',
        'pass.slot': 'Slot',
        'pass.time': 'Time',
        'pass.vehicle': 'Vehicle',
        'pass.download': 'Download QR Pass',
        'pass.qrAlt': 'QR code parking pass',
        'pass.loadError': 'Could not load parking pass',
        'common.close': 'Close',
      };
      return map[key] || key;
    },
    i18n: { language: 'en' },
  }),
}));

vi.mock('@phosphor-icons/react', () => ({
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  DownloadSimple: (props: any) => <span data-testid="icon-download" {...props} />,
  Printer: (props: any) => <span data-testid="icon-printer" {...props} />,
}));

vi.mock('date-fns', () => ({
  format: (_date: Date, fmt: string) => {
    if (fmt === 'HH:mm') return '08:00';
    return '2026-03-21';
  },
}));

vi.mock('../api/client', () => ({
  getInMemoryToken: vi.fn(() => 'test-token'),
}));

import { ParkingPass } from './ParkingPass';

const mockBooking = {
  id: '550e8400-e29b-41d4-a716-446655440000',
  user_id: 'u1',
  lot_id: 'l1',
  slot_id: 's1',
  lot_name: 'Main Garage',
  slot_number: 'A5',
  vehicle_plate: 'M-AB-123',
  start_time: '2026-03-21T08:00:00Z',
  end_time: '2026-03-21T17:00:00Z',
  status: 'active' as const,
};

describe('ParkingPass', () => {
  const mockOnClose = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    // Mock fetch to return a blob
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      blob: () => Promise.resolve(new Blob(['png-data'], { type: 'image/png' })),
    });
    // Mock URL APIs
    globalThis.URL.createObjectURL = vi.fn(() => 'blob:mock-qr-url');
    globalThis.URL.revokeObjectURL = vi.fn();
  });

  it('renders booking details', async () => {
    render(<ParkingPass booking={mockBooking} onClose={mockOnClose} />);
    expect(screen.getByText('Parking Pass')).toBeInTheDocument();
    expect(screen.getByText('Main Garage')).toBeInTheDocument();
    expect(screen.getByText('A5')).toBeInTheDocument();
    expect(screen.getByText('M-AB-123')).toBeInTheDocument();
  });

  it('renders download button', () => {
    render(<ParkingPass booking={mockBooking} onClose={mockOnClose} />);
    expect(screen.getByText('Download QR Pass')).toBeInTheDocument();
  });

  it('calls onClose when backdrop is clicked', () => {
    render(<ParkingPass booking={mockBooking} onClose={mockOnClose} />);
    const dialog = screen.getByRole('dialog');
    fireEvent.click(dialog);
    expect(mockOnClose).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when close button is clicked', () => {
    render(<ParkingPass booking={mockBooking} onClose={mockOnClose} />);
    const closeBtn = screen.getByLabelText('Close');
    fireEvent.click(closeBtn);
    expect(mockOnClose).toHaveBeenCalledTimes(1);
  });

  it('fetches QR image on mount', async () => {
    render(<ParkingPass booking={mockBooking} onClose={mockOnClose} />);
    await waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalledWith(
        expect.stringContaining('/api/v1/bookings/550e8400'),
        expect.any(Object),
      );
    });
  });

  it('shows QR image after successful load', async () => {
    render(<ParkingPass booking={mockBooking} onClose={mockOnClose} />);
    await waitFor(() => {
      const img = screen.getByAltText('QR code parking pass');
      expect(img).toBeInTheDocument();
      expect(img.getAttribute('src')).toBe('blob:mock-qr-url');
    });
  });

  it('shows error when fetch fails', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false,
      blob: () => Promise.reject(new Error('fail')),
    });
    render(<ParkingPass booking={mockBooking} onClose={mockOnClose} />);
    await waitFor(() => {
      expect(screen.getByText('Could not load parking pass')).toBeInTheDocument();
    });
  });

  it('shows error on network failure', async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new Error('net'));
    render(<ParkingPass booking={mockBooking} onClose={mockOnClose} />);
    await waitFor(() => {
      expect(screen.getByText('Could not load parking pass')).toBeInTheDocument();
    });
  });

  it('shows loading state initially', () => {
    globalThis.fetch = vi.fn().mockReturnValue(new Promise(() => {}));
    render(<ParkingPass booking={mockBooking} onClose={mockOnClose} />);
    // Should show loading pulse div (no img, no error)
    expect(screen.queryByAltText('QR code parking pass')).not.toBeInTheDocument();
    expect(screen.queryByText('Could not load parking pass')).not.toBeInTheDocument();
  });

  it('download button works', async () => {
    render(<ParkingPass booking={mockBooking} onClose={mockOnClose} />);
    await waitFor(() => {
      expect(screen.getByAltText('QR code parking pass')).toBeInTheDocument();
    });
    const downloadBtn = screen.getByText('Download QR Pass');
    fireEvent.click(downloadBtn);
    // Should create an anchor and click it
  });

  it('download disabled when no image', () => {
    globalThis.fetch = vi.fn().mockReturnValue(new Promise(() => {}));
    render(<ParkingPass booking={mockBooking} onClose={mockOnClose} />);
    const downloadBtn = screen.getByText('Download QR Pass').closest('button');
    expect(downloadBtn).toBeDisabled();
  });

  it('print button calls window.print', async () => {
    const printSpy = vi.spyOn(window, 'print').mockImplementation(() => {});
    render(<ParkingPass booking={mockBooking} onClose={mockOnClose} />);
    await waitFor(() => expect(screen.getByAltText('QR code parking pass')).toBeInTheDocument());
    const printBtn = screen.getByTitle('Print booking confirmation');
    fireEvent.click(printBtn);
    expect(printSpy).toHaveBeenCalled();
    printSpy.mockRestore();
  });

  it('does not close when inner card is clicked', () => {
    render(<ParkingPass booking={mockBooking} onClose={mockOnClose} />);
    const card = screen.getByText('Parking Pass').closest('.print-pass');
    if (card) fireEvent.click(card);
    expect(mockOnClose).not.toHaveBeenCalled();
  });

  it('shows booking without vehicle plate', () => {
    const bookingNoPlate = { ...mockBooking, vehicle_plate: undefined };
    render(<ParkingPass booking={bookingNoPlate as any} onClose={mockOnClose} />);
    expect(screen.queryByText('Vehicle')).not.toBeInTheDocument();
  });

  it('renders time correctly', async () => {
    render(<ParkingPass booking={mockBooking} onClose={mockOnClose} />);
    // date-fns is mocked to always return '08:00', rendered as "08:00 — 08:00"
    expect(screen.getByText(/08:00/)).toBeInTheDocument();
  });

  it('handles re-mount with different booking id', async () => {
    const { rerender, unmount } = render(<ParkingPass booking={mockBooking} onClose={mockOnClose} />);
    await waitFor(() => expect(screen.getByAltText('QR code parking pass')).toBeInTheDocument());
    rerender(<ParkingPass booking={{ ...mockBooking, id: 'different-id' }} onClose={mockOnClose} />);
    await waitFor(() => expect(screen.getByAltText('QR code parking pass')).toBeInTheDocument());
    unmount();
  });
});
