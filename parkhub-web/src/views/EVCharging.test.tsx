import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'evCharging.title': 'EV Charging',
        'evCharging.subtitle': 'Manage your EV charging sessions',
        'evCharging.adminTitle': 'Charger Management',
        'evCharging.adminSubtitle': 'Charger utilization overview',
        'evCharging.empty': 'No chargers available',
        'evCharging.startCharging': 'Start Charging',
        'evCharging.stopCharging': 'Stop Charging',
        'evCharging.started': 'Charging started',
        'evCharging.stopped': 'Charging stopped',
        'evCharging.history': 'Session History',
        'evCharging.chargingSince': 'Charging since',
        'evCharging.help': 'Manage EV chargers and sessions.',
        'evCharging.aboutTitle': 'About EV Charging',
        'evCharging.totalChargers': 'Total Chargers',
        'evCharging.totalSessions': 'Sessions',
        'evCharging.totalKwh': 'Total kWh',
        'evCharging.status.available': 'Available',
        'evCharging.status.in_use': 'In Use',
        'evCharging.status.offline': 'Offline',
        'evCharging.status.maintenance': 'Maintenance',
        'common.error': 'Error',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Lightning: (props: any) => <span data-testid="icon-lightning" {...props} />,
  Play: (props: any) => <span data-testid="icon-play" {...props} />,
  Stop: (props: any) => <span data-testid="icon-stop" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  BatteryCharging: (props: any) => <span data-testid="icon-battery" {...props} />,
}));

vi.mock('../context/AuthContext', () => ({
  useAuth: () => ({ user: { id: 'user-1', role: 'admin' } }),
}));

import { EVChargingPage, AdminChargersPage } from './EVCharging';

const sampleChargers = [
  { id: 'ch1', lot_id: 'lot-1', label: 'Charger A1', connector_type: 'ccs' as const, power_kw: 50, status: 'available' as const, location_hint: 'Near entrance' },
  { id: 'ch2', lot_id: 'lot-1', label: 'Charger B2', connector_type: 'type2' as const, power_kw: 22, status: 'in_use' as const, location_hint: null },
];

const sampleLots = [{ id: 'lot-1', name: 'Main Lot' }];

describe('EVChargingPage', () => {
  beforeEach(() => {
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/chargers/sessions')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/chargers')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleChargers }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleLots }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
  });

  afterEach(() => vi.restoreAllMocks());

  it('renders EV charging page title', async () => {
    render(<EVChargingPage />);
    await waitFor(() => expect(screen.getByText('EV Charging')).toBeTruthy());
  });

  it('displays chargers after loading', async () => {
    render(<EVChargingPage />);
    await waitFor(() => {
      expect(screen.getByText('Charger A1')).toBeTruthy();
      expect(screen.getByText('Charger B2')).toBeTruthy();
    });
  });

  it('shows start button for available charger', async () => {
    render(<EVChargingPage />);
    await waitFor(() => {
      expect(screen.getByText('Start Charging')).toBeTruthy();
    });
  });

  it('shows connector type labels for chargers', async () => {
    render(<EVChargingPage />);
    await waitFor(() => {
      expect(screen.getByText(/CCS/)).toBeTruthy();
      expect(screen.getByText(/Type 2/)).toBeTruthy();
    });
  });
});

describe('AdminChargersPage', () => {
  beforeEach(() => {
    global.fetch = vi.fn(() =>
      Promise.resolve({ json: () => Promise.resolve({ success: true, data: { total_chargers: 8, available: 5, in_use: 2, offline: 1, total_sessions: 120, total_kwh: 1500.5 } }) } as Response)
    );
  });

  afterEach(() => vi.restoreAllMocks());

  it('renders admin stats', async () => {
    render(<AdminChargersPage />);
    await waitFor(() => {
      expect(screen.getByText('8')).toBeTruthy();
      expect(screen.getByText('1501 kWh')).toBeTruthy();
    });
  });
});
