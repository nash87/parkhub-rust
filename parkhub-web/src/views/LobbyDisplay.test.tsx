import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, act } from '@testing-library/react';

// ── Mocks ──

const mockUseParams = vi.fn().mockReturnValue({ lotId: 'lot-1' });

vi.mock('react-router-dom', () => ({
  useParams: () => mockUseParams(),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'lobby.available': 'Available',
        'lobby.total': 'Total',
        'lobby.floor': 'Floor',
        'lobby.lastUpdated': 'Last updated',
        'lobby.occupancy': 'Occupancy',
        'lobby.error': 'Lot not found',
        'lobby.networkError': 'Network error',
      };
      return map[key] || fallback || key;
    },
  }),
}));

import { LobbyDisplayPage } from './LobbyDisplay';

const MOCK_DISPLAY_DATA = {
  success: true,
  data: {
    lot_id: 'lot-1',
    lot_name: 'Downtown Garage',
    total_slots: 200,
    available_slots: 80,
    occupancy_percent: 60,
    color_status: 'yellow' as const,
    floors: [
      { floor_name: 'B1', floor_number: -1, total_slots: 100, available_slots: 40, occupancy_percent: 60 },
      { floor_name: 'B2', floor_number: -2, total_slots: 100, available_slots: 40, occupancy_percent: 60 },
    ],
    timestamp: '2026-03-22T12:00:00Z',
  },
};

describe('LobbyDisplayPage', () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    mockUseParams.mockReturnValue({ lotId: 'lot-1' });
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('shows loading state initially', () => {
    global.fetch = vi.fn().mockReturnValue(new Promise(() => {}));
    render(<LobbyDisplayPage />);
    expect(screen.getByTestId('lobby-loading')).toBeInTheDocument();
  });

  it('renders lot name and availability after loading', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve(MOCK_DISPLAY_DATA),
    });

    render(<LobbyDisplayPage />);

    await waitFor(() => {
      expect(screen.getByTestId('lobby-display')).toBeInTheDocument();
    });
    expect(screen.getByTestId('lobby-lot-name')).toHaveTextContent('Downtown Garage');
    expect(screen.getByTestId('lobby-available')).toHaveTextContent('80');
    expect(screen.getByTestId('lobby-total')).toHaveTextContent('200');
  });

  it('renders floor breakdown cards', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve(MOCK_DISPLAY_DATA),
    });

    render(<LobbyDisplayPage />);

    await waitFor(() => {
      expect(screen.getByTestId('lobby-floors')).toBeInTheDocument();
    });
    const floorCards = screen.getAllByTestId('lobby-floor-card');
    expect(floorCards).toHaveLength(2);
    expect(screen.getByText(/B1/)).toBeInTheDocument();
    expect(screen.getByText(/B2/)).toBeInTheDocument();
  });

  it('shows error state when lot is not found', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({ success: false, error: { code: 'NOT_FOUND', message: 'Parking lot not found' } }),
    });

    render(<LobbyDisplayPage />);

    await waitFor(() => {
      expect(screen.getByTestId('lobby-error')).toBeInTheDocument();
    });
    expect(screen.getByText('Parking lot not found')).toBeInTheDocument();
  });

  it('displays occupancy bar with correct aria attributes', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve(MOCK_DISPLAY_DATA),
    });

    render(<LobbyDisplayPage />);

    await waitFor(() => {
      expect(screen.getByTestId('lobby-bar')).toBeInTheDocument();
    });
    const bar = screen.getByRole('progressbar');
    expect(bar).toHaveAttribute('aria-valuenow', '60');
    expect(bar).toHaveAttribute('aria-valuemin', '0');
    expect(bar).toHaveAttribute('aria-valuemax', '100');
  });

  it('shows last updated timestamp', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve(MOCK_DISPLAY_DATA),
    });

    render(<LobbyDisplayPage />);

    await waitFor(() => {
      expect(screen.getByTestId('lobby-last-updated')).toBeInTheDocument();
    });
    expect(screen.getByTestId('lobby-last-updated').textContent).toContain('Last updated');
  });
});
