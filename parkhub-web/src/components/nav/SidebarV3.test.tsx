/**
 * SidebarV3 smoke tests.
 *
 * The component is heavy on real-time state (ticker + async API fetches +
 * slot/floor derivation), so we only assert the non-crashing base cases:
 *   1. Renders with empty API responses and shows the "No active booking"
 *      empty-state CTA.
 *   2. Renders the Live Pass card when the API returns an active booking.
 *
 * Visual tuning is left to manual QA / Playwright — unit tests here are a
 * guardrail against regressions, not a pixel-perfect harness.
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import type { Booking, ParkingLot, ParkingSlot } from '../../api/client';

const { getBookingsMock, getLotsMock, getLotSlotsMock } = vi.hoisted(() => ({
  getBookingsMock: vi.fn(),
  getLotsMock: vi.fn(),
  getLotSlotsMock: vi.fn(),
}));

vi.mock('../../api/client', () => ({
  api: {
    getBookings: getBookingsMock,
    getLots: getLotsMock,
    getLotSlots: getLotSlotsMock,
  },
}));

vi.mock('../../context/AuthContext', () => ({
  useAuth: () => ({
    user: {
      id: 'u1',
      username: 'florian',
      email: 'florian@example.com',
      name: 'Florian Bauer',
      role: 'user',
    },
    loading: false,
    logout: vi.fn(),
  }),
}));

function ok<T>(data: T) {
  return Promise.resolve({ success: true, data } as const);
}

describe('SidebarV3', () => {
  beforeEach(() => {
    getBookingsMock.mockReset();
    getLotsMock.mockReset();
    getLotSlotsMock.mockReset();
    vi.spyOn(window, 'setInterval').mockImplementation(() => 1 as unknown as ReturnType<typeof window.setInterval>);
    vi.spyOn(window, 'clearInterval').mockImplementation(() => undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders without crashing when all API responses are empty', async () => {
    getBookingsMock.mockReturnValue(ok<Booking[]>([]));
    getLotsMock.mockReturnValue(ok<ParkingLot[]>([]));
    getLotSlotsMock.mockReturnValue(ok<ParkingSlot[]>([]));

    const { SidebarV3 } = await import('./SidebarV3');

    render(
      <MemoryRouter>
        <SidebarV3 />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(screen.getByText(/No active booking/i)).toBeInTheDocument();
    }, { timeout: 10_000 });
    expect(screen.getByText(/Book now/i)).toBeInTheDocument();
    // Primary nav labels appear.
    expect(screen.getByText('Today')).toBeInTheDocument();
    expect(screen.getByText('Book')).toBeInTheDocument();
  }, 15_000);

  it('renders the Live Pass card for an active booking', async () => {
    const now = new Date();
    const start = new Date(now.getTime() - 60 * 60_000).toISOString();
    const end = new Date(now.getTime() + 2 * 60 * 60_000).toISOString();

    const booking: Booking = {
      id: 'b1',
      user_id: 'u1',
      lot_id: 'lot-hq',
      slot_id: 's1',
      lot_name: 'HQ — Linden St',
      slot_number: 'L1-17',
      vehicle_plate: 'M-AB 7823',
      start_time: start,
      end_time: end,
      status: 'active',
    };

    const lot: ParkingLot = {
      id: 'lot-hq',
      name: 'HQ — Linden St',
      total_slots: 184,
      available_slots: 27,
      status: 'open',
    };

    getBookingsMock.mockReturnValue(ok<Booking[]>([booking]));
    getLotsMock.mockReturnValue(ok<ParkingLot[]>([lot]));
    getLotSlotsMock.mockReturnValue(ok<ParkingSlot[]>([]));

    const { SidebarV3 } = await import('./SidebarV3');

    render(
      <MemoryRouter>
        <SidebarV3 />
      </MemoryRouter>,
    );

    expect(await screen.findByText('Parked', {}, { timeout: 10_000 })).toBeInTheDocument();
    expect(await screen.findByText('Show QR', {}, { timeout: 10_000 })).toBeInTheDocument();
    expect(screen.getAllByText(/M-AB 7823/).length).toBeGreaterThan(0);
  }, 15_000);
});
