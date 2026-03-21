import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

// -- Mocks --

const mockGetFavorites = vi.fn();
const mockGetLots = vi.fn();
const mockGetLotSlots = vi.fn();
const mockRemoveFavorite = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    getFavorites: (...args: any[]) => mockGetFavorites(...args),
    getLots: (...args: any[]) => mockGetLots(...args),
    getLotSlots: (...args: any[]) => mockGetLotSlots(...args),
    removeFavorite: (...args: any[]) => mockRemoveFavorite(...args),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => {
      const map: Record<string, string> = {
        'favorites.title': 'Favorites',
        'favorites.subtitle': 'Your saved parking spots',
        'favorites.empty': 'No favorites yet',
        'favorites.emptyHint': 'Star a parking slot to save it here',
        'favorites.slot': 'Slot',
        'favorites.unknownLot': 'Unknown Lot',
        'favorites.available': 'Available',
        'favorites.occupied': 'Occupied',
        'favorites.count': `${opts?.count ?? 0} favorites`,
        'favorites.remove': `Remove ${opts?.slot ?? ''}`,
        'favorites.removed': 'Favorite removed',
        'favorites.addedOn': `Added on ${opts?.date ?? ''}`,
        'common.error': 'Error',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, variants, initial, animate, exit, transition, whileHover, whileTap, layout, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Star: (props: any) => <span data-testid="icon-star" {...props} />,
  Trash: (props: any) => <span data-testid="icon-trash" {...props} />,
  MapPin: (props: any) => <span data-testid="icon-map-pin" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock('../constants/animations', () => ({
  stagger: { hidden: {}, show: {} },
  fadeUp: { hidden: {}, show: {} },
}));

import { FavoritesPage } from './Favorites';

describe('FavoritesPage', () => {
  beforeEach(() => {
    mockGetFavorites.mockClear();
    mockGetLots.mockClear();
    mockGetLotSlots.mockClear();
    mockRemoveFavorite.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders loading skeleton initially', () => {
    mockGetFavorites.mockReturnValue(new Promise(() => {}));
    mockGetLots.mockReturnValue(new Promise(() => {}));

    render(<FavoritesPage />);

    // The loading state renders skeleton divs
    const skeletons = document.querySelectorAll('.skeleton');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it('renders empty state when no favorites', async () => {
    mockGetFavorites.mockResolvedValue({ success: true, data: [] });
    mockGetLots.mockResolvedValue({ success: true, data: [] });

    render(<FavoritesPage />);

    await waitFor(() => {
      expect(screen.getByText('No favorites yet')).toBeInTheDocument();
    });
    expect(screen.getByText('Star a parking slot to save it here')).toBeInTheDocument();
    expect(screen.getByText('Favorites')).toBeInTheDocument();
  });

  it('renders favorite cards when data exists', async () => {
    mockGetFavorites.mockResolvedValue({
      success: true,
      data: [
        { user_id: 'u-1', slot_id: 's-1', lot_id: 'l-1', created_at: '2026-03-20T10:00:00Z' },
        { user_id: 'u-1', slot_id: 's-2', lot_id: 'l-1', created_at: '2026-03-19T08:00:00Z' },
      ],
    });
    mockGetLots.mockResolvedValue({
      success: true,
      data: [
        { id: 'l-1', name: 'Garage Alpha', total_slots: 20, available_slots: 10, status: 'open' },
      ],
    });
    mockGetLotSlots.mockResolvedValue({
      success: true,
      data: [
        { id: 's-1', slot_number: 'A1', status: 'available', lot_id: 'l-1' },
        { id: 's-2', slot_number: 'B3', status: 'occupied', lot_id: 'l-1' },
      ],
    });

    render(<FavoritesPage />);

    await waitFor(() => {
      expect(screen.getByText(/Slot A1/)).toBeInTheDocument();
    });
    expect(screen.getByText(/Slot B3/)).toBeInTheDocument();
    // Lot name appears for each card
    expect(screen.getAllByText('Garage Alpha').length).toBe(2);
    // Status badges
    expect(screen.getByText('Available')).toBeInTheDocument();
    expect(screen.getByText('Occupied')).toBeInTheDocument();
    // Favorite count
    expect(screen.getByText('2 favorites')).toBeInTheDocument();
  });
});
