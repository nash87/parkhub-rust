import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'parkingZones.title': 'Parking Zones',
        'parkingZones.subtitle': 'Manage zones with pricing tiers',
        'parkingZones.help': 'Configure pricing tiers for zones to manage pricing automatically.',
        'parkingZones.helpLabel': 'Help',
        'parkingZones.empty': 'No zones with pricing configured',
        'parkingZones.editPricing': 'Edit Pricing',
        'parkingZones.tier': 'Tier',
        'parkingZones.multiplier': 'Multiplier',
        'parkingZones.maxCapacity': 'Max Capacity',
        'parkingZones.capacity': 'Capacity',
        'parkingZones.save': 'Save',
        'parkingZones.pricingUpdated': 'Pricing updated',
        'parkingZones.optional': 'Optional',
        'common.cancel': 'Cancel',
        'common.error': 'Error',
        'common.loading': 'Loading...',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, layout, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  MapTrifold: (props: any) => <span data-testid="icon-map" {...props} />,
  Pencil: (props: any) => <span data-testid="icon-pencil" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  Tag: (props: any) => <span data-testid="icon-tag" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: { success: vi.fn(), error: vi.fn() },
}));

import { AdminZonesPage } from './AdminZones';

const sampleLots = [{ id: 'lot-1', name: 'Main Lot' }];
const sampleZones = [
  {
    id: 'zone-1',
    lot_id: 'lot-1',
    name: 'Economy Section',
    description: 'Ground floor',
    color: '#22c55e',
    tier: 'economy',
    tier_display: 'Economy',
    tier_color: '#22c55e',
    pricing_multiplier: 0.8,
    max_capacity: 100,
  },
  {
    id: 'zone-2',
    lot_id: 'lot-1',
    name: 'VIP Lounge',
    description: null,
    color: '#a855f7',
    tier: 'vip',
    tier_display: 'VIP',
    tier_color: '#a855f7',
    pricing_multiplier: 2.5,
    max_capacity: 20,
  },
];

describe('AdminZonesPage', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('renders title and subtitle', async () => {
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) });

    render(<AdminZonesPage />);
    expect(screen.getByText('Parking Zones')).toBeDefined();
    expect(screen.getByText('Manage zones with pricing tiers')).toBeDefined();
  });

  it('renders zone cards with tier badges', async () => {
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) });

    render(<AdminZonesPage />);
    await waitFor(() => {
      expect(screen.getByText('Economy Section')).toBeDefined();
      expect(screen.getByText('VIP Lounge')).toBeDefined();
    });
  });

  it('shows tier display names', async () => {
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) });

    render(<AdminZonesPage />);
    await waitFor(() => {
      expect(screen.getByText('Economy')).toBeDefined();
      expect(screen.getByText('VIP')).toBeDefined();
    });
  });

  it('shows multiplier values', async () => {
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) });

    render(<AdminZonesPage />);
    await waitFor(() => {
      expect(screen.getByText('0.8x')).toBeDefined();
      expect(screen.getByText('2.5x')).toBeDefined();
    });
  });

  it('shows empty state', async () => {
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: [] }) });

    render(<AdminZonesPage />);
    await waitFor(() => {
      expect(screen.getByText('No zones with pricing configured')).toBeDefined();
    });
  });

  it('handles API errors', async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new Error('Network error'));

    render(<AdminZonesPage />);
    await waitFor(() => {
      expect(screen.getByText('Parking Zones')).toBeDefined();
    });
  });
});
