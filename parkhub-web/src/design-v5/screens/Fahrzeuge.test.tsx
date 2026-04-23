import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockGetVehicles = vi.fn();
const mockCreateVehicle = vi.fn();
const mockDeleteVehicle = vi.fn();

vi.mock('../../api/client', () => ({
  api: {
    getVehicles: (...a: unknown[]) => mockGetVehicles(...a),
    createVehicle: (...a: unknown[]) => mockCreateVehicle(...a),
    deleteVehicle: (...a: unknown[]) => mockDeleteVehicle(...a),
  },
}));

const mockToast = vi.fn();
vi.mock('../Toast', () => ({
  useV5Toast: () => mockToast,
  V5ToastProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

import { FahrzeugeV5 } from './Fahrzeuge';

function renderScreen(navigate = vi.fn()) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <FahrzeugeV5 navigate={navigate} />
    </QueryClientProvider>
  );
}

const VEHICLE_BMW = {
  id: 'v-001',
  plate: 'M-AB 123',
  make: 'BMW',
  model: '3er',
  color: 'blue',
  is_default: true,
};
const VEHICLE_EV = {
  id: 'v-002',
  plate: 'M-EV 001',
  make: 'Tesla',
  model: 'Model 3',
  color: 'white',
  is_default: false,
};

describe('FahrzeugeV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state with CTA', async () => {
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Noch keine Fahrzeuge')).toBeInTheDocument());
    expect(screen.getByText('+ Fahrzeug hinzufügen')).toBeInTheDocument();
  });

  it('renders vehicle card with Standard badge', async () => {
    mockGetVehicles.mockResolvedValue({ success: true, data: [VEHICLE_BMW] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('M-AB 123')).toBeInTheDocument());
    expect(screen.getByText('BMW 3er')).toBeInTheDocument();
    expect(screen.getByText('Standard')).toBeInTheDocument();
  });

  it('renders EV badge for Tesla vehicles', async () => {
    mockGetVehicles.mockResolvedValue({ success: true, data: [VEHICLE_EV] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('M-EV 001')).toBeInTheDocument());
    expect(screen.getByText('EV')).toBeInTheDocument();
  });

  it('opens add modal on Hinzufügen click', async () => {
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => screen.getByText('Noch keine Fahrzeuge'));
    fireEvent.click(screen.getAllByText(/Hinzufügen|hinzufügen/).pop()!);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('createVehicle fires on Speichern with plate + toast success', async () => {
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    mockCreateVehicle.mockResolvedValue({ success: true, data: VEHICLE_BMW });
    renderScreen();
    await waitFor(() => screen.getByText('Noch keine Fahrzeuge'));
    fireEvent.click(screen.getAllByText(/Hinzufügen|hinzufügen/).pop()!);
    fireEvent.change(screen.getByPlaceholderText('M-AB 1234'), { target: { value: 'M-AB 123' } });
    fireEvent.click(screen.getByText('Speichern'));
    await waitFor(() => {
      expect(mockCreateVehicle).toHaveBeenCalledWith(expect.objectContaining({ plate: 'M-AB 123' }));
      expect(mockToast).toHaveBeenCalledWith('Fahrzeug hinzugefügt', 'success');
    });
  });

  it('deleteVehicle fires + toast on delete click', async () => {
    mockGetVehicles.mockResolvedValue({ success: true, data: [VEHICLE_BMW] });
    mockDeleteVehicle.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => screen.getByText('M-AB 123'));
    fireEvent.click(screen.getByLabelText('Fahrzeug M-AB 123 löschen'));
    await waitFor(() => {
      expect(mockDeleteVehicle).toHaveBeenCalledWith('v-001');
      expect(mockToast).toHaveBeenCalledWith('Fahrzeug entfernt', 'success');
    });
  });
});
