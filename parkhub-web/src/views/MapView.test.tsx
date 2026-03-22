import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

// ── Mocks ──

const mockGetMapMarkers = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    getMapMarkers: (...args: any[]) => mockGetMapMarkers(...args),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'map.title': 'Parking Map',
        'map.subtitle': 'Find available parking lots near you',
        'map.bookNow': 'Book Now',
        'map.available': 'Available',
        'map.noLots': 'No parking lots with location data available',
        'map.closed': 'Closed',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, variants, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
}));

vi.mock('@phosphor-icons/react', () => ({
  MapPin: (props: any) => <span data-testid="icon-map-pin" {...props} />,
  NavigationArrow: (props: any) => <span data-testid="icon-nav-arrow" {...props} />,
}));

vi.mock('../constants/animations', () => ({
  staggerSlow: { hidden: {}, show: {} },
  fadeUp: { hidden: {}, show: {} },
}));

// Mock react-leaflet to avoid DOM issues in test environment
vi.mock('react-leaflet', () => ({
  MapContainer: ({ children, ...props }: any) => (
    <div data-testid="leaflet-map" {...props}>{children}</div>
  ),
  TileLayer: () => <div data-testid="tile-layer" />,
  Marker: ({ children }: any) => <div data-testid="map-marker">{children}</div>,
  Popup: ({ children }: any) => <div data-testid="map-popup">{children}</div>,
  useMap: () => ({
    fitBounds: vi.fn(),
  }),
}));

vi.mock('leaflet', () => ({
  default: {
    Icon: {
      Default: {
        prototype: {},
        mergeOptions: vi.fn(),
      },
    },
    divIcon: vi.fn(() => ({})),
    latLngBounds: vi.fn(() => ({})),
  },
  divIcon: vi.fn(() => ({})),
  latLngBounds: vi.fn(() => ({})),
  Icon: {
    Default: {
      prototype: {},
      mergeOptions: vi.fn(),
    },
  },
}));

import { MapViewPage } from './MapView';

describe('MapViewPage', () => {
  beforeEach(() => {
    mockGetMapMarkers.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading skeleton initially', () => {
    mockGetMapMarkers.mockReturnValue(new Promise(() => {}));
    render(<MapViewPage />);
    const skeletons = document.querySelectorAll('.skeleton');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it('shows empty state when no lots have locations', async () => {
    mockGetMapMarkers.mockResolvedValue({
      success: true,
      data: [],
    });

    render(<MapViewPage />);

    await waitFor(() => {
      expect(screen.getByText('No parking lots with location data available')).toBeInTheDocument();
    });
  });

  it('renders map with markers when lots have coordinates', async () => {
    mockGetMapMarkers.mockResolvedValue({
      success: true,
      data: [
        {
          id: 'lot-1',
          name: 'Central Parking',
          address: '123 Main St',
          latitude: 48.1351,
          longitude: 11.582,
          available_slots: 42,
          total_slots: 100,
          status: 'open',
          color: 'green',
        },
        {
          id: 'lot-2',
          name: 'Airport Parking',
          address: '456 Airport Rd',
          latitude: 48.354,
          longitude: 11.786,
          available_slots: 5,
          total_slots: 200,
          status: 'open',
          color: 'red',
        },
      ],
    });

    render(<MapViewPage />);

    await waitFor(() => {
      expect(screen.getByText('Parking Map')).toBeInTheDocument();
    });

    expect(screen.getByTestId('leaflet-map')).toBeInTheDocument();
    const markers = screen.getAllByTestId('map-marker');
    expect(markers).toHaveLength(2);
  });

  it('renders page title and subtitle', async () => {
    mockGetMapMarkers.mockResolvedValue({
      success: true,
      data: [
        {
          id: 'lot-1',
          name: 'Test Lot',
          address: 'Test Address',
          latitude: 48.0,
          longitude: 11.0,
          available_slots: 10,
          total_slots: 20,
          status: 'open',
          color: 'green',
        },
      ],
    });

    render(<MapViewPage />);

    await waitFor(() => {
      expect(screen.getByText('Parking Map')).toBeInTheDocument();
    });
    expect(screen.getByText('Find available parking lots near you')).toBeInTheDocument();
  });

  it('shows marker popups with lot details', async () => {
    mockGetMapMarkers.mockResolvedValue({
      success: true,
      data: [
        {
          id: 'lot-1',
          name: 'Central Parking',
          address: '123 Main St',
          latitude: 48.1351,
          longitude: 11.582,
          available_slots: 42,
          total_slots: 100,
          status: 'open',
          color: 'green',
        },
      ],
    });

    render(<MapViewPage />);

    await waitFor(() => {
      expect(screen.getByText('Central Parking')).toBeInTheDocument();
    });
    expect(screen.getByText('123 Main St')).toBeInTheDocument();
    expect(screen.getByText('Book Now')).toBeInTheDocument();
  });

  it('handles API failure gracefully', async () => {
    mockGetMapMarkers.mockResolvedValue({
      success: false,
      data: null,
    });

    render(<MapViewPage />);

    await waitFor(() => {
      expect(screen.getByText('No parking lots with location data available')).toBeInTheDocument();
    });
  });
});
