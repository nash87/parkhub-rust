import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

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

const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();
vi.mock('react-hot-toast', () => ({
  default: { success: (...a: any[]) => mockToastSuccess(...a), error: (...a: any[]) => mockToastError(...a) },
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

  it('shows help text on admin page', async () => {
    render(<AdminChargersPage />);
    await waitFor(() => expect(screen.getByText('Charger Management')).toBeTruthy());
    const helpBtn = screen.getByLabelText('Help');
    fireEvent.click(helpBtn);
    await waitFor(() => expect(screen.getByText('About EV Charging')).toBeTruthy());
  });
});

describe('EVChargingPage - extended', () => {
  afterEach(() => vi.restoreAllMocks());

  it('shows empty chargers state', async () => {
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/chargers/sessions')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/chargers')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'lot-1', name: 'Main Lot' }] }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
    render(<EVChargingPage />);
    await waitFor(() => expect(screen.getByText('No chargers available')).toBeTruthy());
  });

  it('shows help text', async () => {
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'lot-1', name: 'Main Lot' }] }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
    render(<EVChargingPage />);
    await waitFor(() => expect(screen.getByText('EV Charging')).toBeTruthy());
    const helpBtn = screen.getByLabelText('Help');
    fireEvent.click(helpBtn);
    await waitFor(() => expect(screen.getByText('About EV Charging')).toBeTruthy());
  });

  it('starts charging', async () => {
    const mockFetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/start')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/chargers/sessions')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/chargers')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [
          { id: 'ch1', lot_id: 'lot-1', label: 'C1', connector_type: 'ccs', power_kw: 50, status: 'available', location_hint: null },
        ] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'lot-1', name: 'Main Lot' }] }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
    global.fetch = mockFetch;
    render(<EVChargingPage />);
    await waitFor(() => expect(screen.getByText('Start Charging')).toBeTruthy());
    fireEvent.click(screen.getByText('Start Charging'));
    await waitFor(() => expect(mockFetch).toHaveBeenCalledWith(expect.stringContaining('/start'), expect.anything()));
  });

  it('start charging failure', async () => {

    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/start')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Busy' } }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/chargers/sessions')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/chargers')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [
          { id: 'ch1', lot_id: 'lot-1', label: 'C1', connector_type: 'ccs', power_kw: 50, status: 'available', location_hint: null },
        ] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'lot-1', name: 'Lot' }] }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
    render(<EVChargingPage />);
    await waitFor(() => expect(screen.getByText('Start Charging')).toBeTruthy());
    fireEvent.click(screen.getByText('Start Charging'));
    await waitFor(() => expect(mockToastError).toHaveBeenCalledWith('Busy'));
  });

  it('start charging network error', async () => {

    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/start')) {
        return Promise.reject(new Error('net'));
      }
      if (typeof url === 'string' && url.includes('/chargers/sessions')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/chargers')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [
          { id: 'ch1', lot_id: 'lot-1', label: 'C1', connector_type: 'ccs', power_kw: 50, status: 'available', location_hint: null },
        ] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'lot-1', name: 'Lot' }] }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
    render(<EVChargingPage />);
    await waitFor(() => expect(screen.getByText('Start Charging')).toBeTruthy());
    fireEvent.click(screen.getByText('Start Charging'));
    await waitFor(() => expect(mockToastError).toHaveBeenCalledWith('Error'));
  });

  it('stops charging with active session', async () => {
    const sessions = [{ id: 'ses1', charger_id: 'ch1', user_id: 'u1', start_time: '2026-04-10T08:00:00Z', end_time: null, kwh_consumed: 5.5, status: 'active' }];
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/stop')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/chargers/sessions')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sessions }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/chargers')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [
          { id: 'ch1', lot_id: 'lot-1', label: 'Charger A1', connector_type: 'ccs', power_kw: 50, status: 'in_use', location_hint: null },
        ] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'lot-1', name: 'Main' }] }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
    render(<EVChargingPage />);
    await waitFor(() => expect(screen.getByText('Stop Charging')).toBeTruthy());
    expect(screen.getByText(/Charging since/)).toBeTruthy();
    fireEvent.click(screen.getByText('Stop Charging'));
  });

  it('stop charging failure', async () => {

    const sessions = [{ id: 'ses1', charger_id: 'ch1', user_id: 'u1', start_time: '2026-04-10T08:00:00Z', end_time: null, kwh_consumed: 5, status: 'active' }];
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/stop')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Cannot stop' } }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/chargers/sessions')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sessions }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/chargers')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [
          { id: 'ch1', lot_id: 'lot-1', label: 'C1', connector_type: 'type2', power_kw: 22, status: 'in_use', location_hint: 'Floor 2' },
        ] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'lot-1', name: 'Lot' }] }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
    render(<EVChargingPage />);
    await waitFor(() => expect(screen.getByText('Stop Charging')).toBeTruthy());
    fireEvent.click(screen.getByText('Stop Charging'));
    await waitFor(() => expect(mockToastError).toHaveBeenCalledWith('Cannot stop'));
  });

  it('shows session history', async () => {
    const sessions = [
      { id: 'ses1', charger_id: 'ch1', user_id: 'u1', start_time: '2026-04-08T08:00:00Z', end_time: '2026-04-08T10:00:00Z', kwh_consumed: 15.3, status: 'completed' },
    ];
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/chargers/sessions')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sessions }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/chargers')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'lot-1', name: 'Lot' }] }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
    render(<EVChargingPage />);
    await waitFor(() => expect(screen.getByText('Session History')).toBeTruthy());
    expect(screen.getByText('15.3 kWh')).toBeTruthy();
  });

  it('lot selector changes chargers', async () => {
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/chargers/sessions')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/chargers')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'lot-1', name: 'Lot 1' }, { id: 'lot-2', name: 'Lot 2' }] }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
    render(<EVChargingPage />);
    await waitFor(() => expect(screen.getByText('EV Charging')).toBeTruthy());
    const select = screen.getByRole('combobox');
    fireEvent.change(select, { target: { value: 'lot-2' } });
    // Verify that chargers get reloaded (fetch called multiple times)
    await waitFor(() => {
      const fetchCalls = (global.fetch as any).mock.calls.map((c: any) => c[0]);
      expect(fetchCalls.some((url: string) => url.includes('lot-2'))).toBe(true);
    });
  });

  it('shows location hint when present', async () => {
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/chargers/sessions')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/chargers')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [
          { id: 'ch1', lot_id: 'lot-1', label: 'C1', connector_type: 'ccs', power_kw: 50, status: 'available', location_hint: 'Near entrance' },
        ] }) } as Response);
      }
      if (typeof url === 'string' && url.includes('/lots')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [{ id: 'lot-1', name: 'Lot' }] }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
    });
    render(<EVChargingPage />);
    await waitFor(() => expect(screen.getByText('Near entrance')).toBeTruthy());
  });
});
