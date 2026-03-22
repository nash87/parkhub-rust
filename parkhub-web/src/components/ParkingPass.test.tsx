import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';

// ── Mocks ──

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'pass.title': 'Parking Pass',
        'pass.lot': 'Parking Lot',
        'pass.slot': 'Slot',
        'pass.time': 'Time',
        'pass.vehicle': 'Vehicle',
        'pass.download': 'Download QR Pass',
        'pass.qrAlt': 'QR code parking pass',
        'pass.loadError': 'Could not load parking pass',
        'pass.activeSession': 'Active Session',
        'pass.assignedSlot': 'Assigned Slot',
        'pass.location': 'Location',
        'pass.validFrom': 'Valid From',
        'pass.expires': 'Expires',
        'pass.navigateToSlot': 'Navigate to Slot',
        'pass.extend': 'Extend',
        'pass.cancel': 'Cancel',
        'pass.tabPasses': 'Passes',
        'pass.tabMap': 'Map',
        'pass.tabHistory': 'History',
        'pass.tabProfile': 'Profile',
        'pass.print': 'Print Pass',
        'common.close': 'Close',
      };
      return map[key] || (typeof fallback === 'string' ? fallback : key);
    },
    i18n: { language: 'en' },
  }),
}));

vi.mock('@phosphor-icons/react', () => ({
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  DownloadSimple: (props: any) => <span data-testid="icon-download" {...props} />,
  Printer: (props: any) => <span data-testid="icon-printer" {...props} />,
  NavigationArrow: (props: any) => <span data-testid="icon-nav" {...props} />,
  ClockCounterClockwise: (props: any) => <span data-testid="icon-extend" {...props} />,
  XCircle: (props: any) => <span data-testid="icon-xcircle" {...props} />,
  Ticket: (props: any) => <span data-testid="icon-ticket" {...props} />,
  MapTrifold: (props: any) => <span data-testid="icon-map" {...props} />,
  ClockClockwise: (props: any) => <span data-testid="icon-history" {...props} />,
  UserCircle: (props: any) => <span data-testid="icon-user" {...props} />,
}));

vi.mock('date-fns', () => ({
  format: (_date: Date, fmt: string) => {
    if (fmt === 'HH:mm') return '08:00';
    return '2026-03-21';
  },
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
    expect(screen.getByLabelText('Download QR Pass')).toBeInTheDocument();
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
});
