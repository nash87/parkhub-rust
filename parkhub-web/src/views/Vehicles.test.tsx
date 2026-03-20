import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockGetVehicles = vi.fn();
const mockCreateVehicle = vi.fn();
const mockDeleteVehicle = vi.fn();
const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    getVehicles: (...args: any[]) => mockGetVehicles(...args),
    createVehicle: (...args: any[]) => mockCreateVehicle(...args),
    deleteVehicle: (...args: any[]) => mockDeleteVehicle(...args),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'vehicles.title': 'Meine Fahrzeuge',
        'vehicles.subtitle': 'Fahrzeuge verwalten',
        'vehicles.add': 'Hinzufügen',
        'vehicles.newVehicle': 'Neues Fahrzeug',
        'vehicles.plate': 'Kennzeichen',
        'vehicles.make': 'Marke',
        'vehicles.model': 'Modell',
        'vehicles.color': 'Farbe',
        'vehicles.noVehicles': 'Noch keine Fahrzeuge angelegt',
        'vehicles.added': 'Fahrzeug hinzugefügt',
        'vehicles.removed': 'Fahrzeug entfernt',
        'vehicles.isDefault': 'Standardfahrzeug',
        'common.save': 'Speichern',
        'common.cancel': 'Abbrechen',
      };
      return map[key] || fallback || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, variants, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    button: React.forwardRef(({ children, variants, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <button ref={ref} {...props}>{children}</button>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Car: (props: any) => <span data-testid="icon-car" {...props} />,
  Plus: (props: any) => <span data-testid="icon-plus" {...props} />,
  Trash: (props: any) => <span data-testid="icon-trash" {...props} />,
  Star: (props: any) => <span data-testid="icon-star" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
}));

vi.mock('../components/Skeleton', () => ({
  VehiclesSkeleton: () => <div data-testid="vehicles-skeleton">Loading...</div>,
}));

vi.mock('react-hot-toast', () => ({
  default: {
    success: (...args: any[]) => mockToastSuccess(...args),
    error: (...args: any[]) => mockToastError(...args),
  },
}));

vi.mock('../constants/animations', () => ({
  stagger: { hidden: {}, show: {} },
  fadeUp: { hidden: {}, show: {} },
}));

import { VehiclesPage } from './Vehicles';

describe('VehiclesPage', () => {
  beforeEach(() => {
    mockGetVehicles.mockClear();
    mockCreateVehicle.mockClear();
    mockDeleteVehicle.mockClear();
    mockToastSuccess.mockClear();
    mockToastError.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading skeleton initially', () => {
    mockGetVehicles.mockReturnValue(new Promise(() => {}));
    render(<VehiclesPage />);
    expect(screen.getByTestId('vehicles-skeleton')).toBeInTheDocument();
  });

  it('shows empty state when no vehicles', async () => {
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    render(<VehiclesPage />);

    await waitFor(() => {
      expect(screen.getByText('Noch keine Fahrzeuge angelegt')).toBeInTheDocument();
    });
  });

  it('renders vehicle list after loading', async () => {
    mockGetVehicles.mockResolvedValue({
      success: true,
      data: [
        { id: 'v-1', plate: 'M-AB 1234', make: 'BMW', model: '320i', color: 'Schwarz', is_default: true },
        { id: 'v-2', plate: 'M-CD 5678', make: 'Audi', model: 'A4', is_default: false },
      ],
    });

    render(<VehiclesPage />);

    await waitFor(() => {
      expect(screen.getByText('M-AB 1234')).toBeInTheDocument();
    });
    expect(screen.getByText('M-CD 5678')).toBeInTheDocument();
    expect(screen.getByText('BMW 320i')).toBeInTheDocument();
    expect(screen.getByText('Audi A4')).toBeInTheDocument();
    expect(screen.getByText('Schwarz')).toBeInTheDocument();
    expect(screen.getByText('Standardfahrzeug')).toBeInTheDocument();
  });

  it('renders heading and add button', async () => {
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    render(<VehiclesPage />);

    await waitFor(() => {
      expect(screen.getByText('Meine Fahrzeuge')).toBeInTheDocument();
    });
    expect(screen.getByText('Fahrzeuge verwalten')).toBeInTheDocument();
  });

  it('opens add vehicle modal', async () => {
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    const user = userEvent.setup();
    render(<VehiclesPage />);

    await waitFor(() => {
      expect(screen.getByText('Noch keine Fahrzeuge angelegt')).toBeInTheDocument();
    });

    // Click the inline add button in the empty state
    const addButtons = screen.getAllByText('Hinzufügen');
    await user.click(addButtons[0]);

    expect(screen.getByText('Neues Fahrzeug')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('M-AB 1234')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('BMW')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('3er')).toBeInTheDocument();
  });

  it('creates a vehicle via the form', async () => {
    mockGetVehicles.mockResolvedValue({ success: true, data: [] });
    mockCreateVehicle.mockResolvedValue({
      success: true,
      data: { id: 'v-new', plate: 'M-XX 9999', make: 'VW', model: 'Golf', is_default: false },
    });
    const user = userEvent.setup();
    render(<VehiclesPage />);

    await waitFor(() => {
      expect(screen.getByText('Noch keine Fahrzeuge angelegt')).toBeInTheDocument();
    });

    const addButtons = screen.getAllByText('Hinzufügen');
    await user.click(addButtons[0]);

    await user.type(screen.getByPlaceholderText('M-AB 1234'), 'M-XX 9999');
    await user.type(screen.getByPlaceholderText('BMW'), 'VW');
    await user.type(screen.getByPlaceholderText('3er'), 'Golf');
    await user.click(screen.getByText('Speichern'));

    await waitFor(() => {
      expect(mockCreateVehicle).toHaveBeenCalledWith(
        expect.objectContaining({ plate: 'M-XX 9999', make: 'VW', model: 'Golf' })
      );
    });
    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith('Fahrzeug hinzugefügt');
    });
  });

  it('deletes a vehicle', async () => {
    mockGetVehicles.mockResolvedValue({
      success: true,
      data: [{ id: 'v-1', plate: 'M-AB 1234', is_default: false }],
    });
    mockDeleteVehicle.mockResolvedValue({ success: true });
    const user = userEvent.setup();
    render(<VehiclesPage />);

    await waitFor(() => {
      expect(screen.getByText('M-AB 1234')).toBeInTheDocument();
    });

    const deleteBtn = screen.getByTestId('icon-trash').closest('button');
    await user.click(deleteBtn!);

    await waitFor(() => {
      expect(mockDeleteVehicle).toHaveBeenCalledWith('v-1');
    });
    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith('Fahrzeug entfernt');
    });
  });
});
