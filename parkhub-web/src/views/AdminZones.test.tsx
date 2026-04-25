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

const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('react-hot-toast', () => ({
  default: {
    success: (...a: any[]) => mockToastSuccess(...a),
    error: (...a: any[]) => mockToastError(...a),
  },
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
    mockToastSuccess.mockClear();
    mockToastError.mockClear();
  });

  it('renders title and subtitle', async () => {
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) }) as any;

    render(<AdminZonesPage />);
    expect(screen.getByText('Parking Zones')).toBeDefined();
    expect(screen.getByText('Manage zones with pricing tiers')).toBeDefined();
  });

  it('renders zone cards with tier badges', async () => {
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) }) as any;

    render(<AdminZonesPage />);
    await waitFor(() => {
      expect(screen.getByText('Economy Section')).toBeDefined();
      expect(screen.getByText('VIP Lounge')).toBeDefined();
    });
  });

  it('shows tier display names', async () => {
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) }) as any;

    render(<AdminZonesPage />);
    await waitFor(() => {
      expect(screen.getByText('Economy')).toBeDefined();
      expect(screen.getByText('VIP')).toBeDefined();
    });
  });

  it('shows multiplier values', async () => {
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) }) as any;

    render(<AdminZonesPage />);
    await waitFor(() => {
      expect(screen.getByText('0.8x')).toBeDefined();
      expect(screen.getByText('2.5x')).toBeDefined();
    });
  });

  it('shows empty state', async () => {
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: [] }) }) as any;

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

  it('shows description for zones that have one', async () => {
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) }) as any;

    render(<AdminZonesPage />);
    await waitFor(() => {
      expect(screen.getByText('Ground floor')).toBeDefined();
    });
  });

  it('shows max capacity values', async () => {
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) }) as any;

    render(<AdminZonesPage />);
    await waitFor(() => {
      expect(screen.getByText('100')).toBeDefined();
      expect(screen.getByText('20')).toBeDefined();
    });
  });

  it('renders zone names from API response', async () => {
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) }) as any;

    render(<AdminZonesPage />);
    await waitFor(() => {
      expect(screen.getByText('Economy Section')).toBeDefined();
    });
  });

  it('toggles help tooltip on click', async () => {
    const user = (await import('@testing-library/user-event')).default.setup();
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) }) as any;

    render(<AdminZonesPage />);
    await waitFor(() => expect(screen.getByText('Parking Zones')).toBeInTheDocument());

    // Click help button
    const helpBtn = screen.getByTitle('Help');
    await user.click(helpBtn);
    expect(screen.getByText('Configure pricing tiers for zones to manage pricing automatically.')).toBeInTheDocument();

    // Click again to hide
    await user.click(helpBtn);
    await waitFor(() => {
      expect(screen.queryByText('Configure pricing tiers for zones to manage pricing automatically.')).not.toBeInTheDocument();
    });
  });

  it('opens edit form when edit button is clicked', async () => {
    const user = (await import('@testing-library/user-event')).default.setup();
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) }) as any;

    render(<AdminZonesPage />);
    await waitFor(() => expect(screen.getByText('Economy Section')).toBeInTheDocument());

    const editBtns = screen.getAllByTitle('Edit Pricing');
    await user.click(editBtns[0]!);

    await waitFor(() => {
      expect(screen.getByText('Tier')).toBeInTheDocument();
      expect(screen.getByText('Multiplier')).toBeInTheDocument();
      expect(screen.getByText('Max Capacity')).toBeInTheDocument();
    });
  });

  it('saves zone pricing successfully', async () => {
    const user = (await import('@testing-library/user-event')).default.setup();
    // Use mockImplementation to handle any number of calls
    const lotsResp = { ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) };
    const zonesResp = { ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) };
    const putResp = { ok: true, json: () => Promise.resolve({ success: true }) };

    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'PUT') return Promise.resolve(putResp);
      if (typeof url === 'string' && url.includes('/zones/pricing')) return Promise.resolve(zonesResp);
      return Promise.resolve(lotsResp);
    });

    render(<AdminZonesPage />);
    await waitFor(() => expect(screen.getByText('Economy Section')).toBeInTheDocument());

    const editBtns = screen.getAllByTitle('Edit Pricing');
    await user.click(editBtns[0]!);

    await waitFor(() => expect(screen.getByText('Save')).toBeInTheDocument());
    await user.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith('Pricing updated');
    });
  });

  it('shows error on save pricing failure', async () => {
    const user = (await import('@testing-library/user-event')).default.setup();
    const lotsResp = { ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) };
    const zonesResp = { ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) };

    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'PUT') return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: false, error: { message: 'Invalid' } }) }) as any;
      if (typeof url === 'string' && url.includes('/zones/pricing')) return Promise.resolve(zonesResp);
      return Promise.resolve(lotsResp);
    });

    render(<AdminZonesPage />);
    await waitFor(() => expect(screen.getByText('Economy Section')).toBeInTheDocument());

    const editBtns = screen.getAllByTitle('Edit Pricing');
    await user.click(editBtns[0]!);

    await user.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Invalid');
    });
  });

  it('cancels edit form', async () => {
    const user = (await import('@testing-library/user-event')).default.setup();
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) }) as any;

    render(<AdminZonesPage />);
    await waitFor(() => expect(screen.getByText('Economy Section')).toBeInTheDocument());

    const editBtns = screen.getAllByTitle('Edit Pricing');
    await user.click(editBtns[0]!);

    await waitFor(() => expect(screen.getByText('Cancel')).toBeInTheDocument());
    await user.click(screen.getByText('Cancel'));

    await waitFor(() => {
      expect(screen.queryByText('Multiplier')).not.toBeInTheDocument();
    });
  });

  it('selects different tier in edit form', async () => {
    const user = (await import('@testing-library/user-event')).default.setup();
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) }) as any;

    render(<AdminZonesPage />);
    await waitFor(() => expect(screen.getByText('Economy Section')).toBeInTheDocument());

    const editBtns = screen.getAllByTitle('Edit Pricing');
    await user.click(editBtns[0]!);

    await waitFor(() => expect(screen.getByText('PREMIUM')).toBeInTheDocument());
    await user.click(screen.getByText('PREMIUM'));
    // Should not crash, tier should be selected
  });

  it('handles zones without max_capacity', async () => {
    const zonesNoCapacity = [{
      ...sampleZones[0],
      max_capacity: null,
    }];
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: zonesNoCapacity }) }) as any;

    render(<AdminZonesPage />);
    await waitFor(() => expect(screen.getByText('Economy Section')).toBeInTheDocument());
    // Capacity bar should not be rendered
    expect(screen.queryByText('Capacity')).not.toBeInTheDocument();
  });

  it('handles save pricing network error', async () => {
    const user = (await import('@testing-library/user-event')).default.setup();
    globalThis.fetch = vi.fn()
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleLots }) })
      .mockResolvedValueOnce({ ok: true, json: () => Promise.resolve({ success: true, data: sampleZones }) })
      .mockRejectedValueOnce(new Error('Network'));

    render(<AdminZonesPage />);
    await waitFor(() => expect(screen.getByText('Economy Section')).toBeInTheDocument());

    const editBtns = screen.getAllByTitle('Edit Pricing');
    await user.click(editBtns[0]!);
    await user.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Error');
    });
  });

  it('shows loading state', () => {
    globalThis.fetch = vi.fn().mockReturnValue(new Promise(() => {}));
    render(<AdminZonesPage />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });
});
