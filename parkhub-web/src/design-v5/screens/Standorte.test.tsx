import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockList = vi.fn();
const mockCreate = vi.fn();
const mockUpdate = vi.fn();
const mockDelete = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    getLots: (...a: unknown[]) => mockList(...a),
    createLot: (...a: unknown[]) => mockCreate(...a),
    updateLot: (...a: unknown[]) => mockUpdate(...a),
    deleteLot: (...a: unknown[]) => mockDelete(...a),
  },
}));

vi.mock('@number-flow/react', () => ({
  default: ({ value }: { value: number }) => <span>{value}</span>,
}));

const mockToast = vi.fn();
vi.mock('../Toast', () => ({
  useV5Toast: () => mockToast,
  V5ToastProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

import { StandorteV5 } from './Standorte';

const L1 = { id: 'l1', name: 'Parkhaus Nord', total_slots: 100, available_slots: 60, status: 'open' };
const L2 = { id: 'l2', name: 'Garage Süd', total_slots: 50, available_slots: 0, status: 'full' };

function renderScreen() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <StandorteV5 navigate={vi.fn()} />
    </QueryClientProvider>
  );
}

describe('StandorteV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state', async () => {
    mockList.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine Standorte')).toBeInTheDocument());
  });

  it('renders error state', async () => {
    mockList.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'fail' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('lists lots with status badges', async () => {
    mockList.mockResolvedValue({ success: true, data: [L1, L2] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('standorte-row')).toHaveLength(2));
    expect(screen.getByText('Parkhaus Nord')).toBeInTheDocument();
    // "Voll" appears as badge + option — at least one occurrence is required
    expect(screen.getAllByText('Voll').length).toBeGreaterThan(0);
  });

  it('create mutation calls createLot', async () => {
    mockList.mockResolvedValue({ success: true, data: [] });
    mockCreate.mockResolvedValue({ success: true, data: { ...L1, id: 'new', name: 'Neu', total_slots: 30, available_slots: 30 } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('standorte-toggle-form')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('standorte-toggle-form'));
    fireEvent.change(screen.getByTestId('standorte-name'), { target: { value: 'Neu' } });
    fireEvent.change(screen.getByTestId('standorte-slots'), { target: { value: '30' } });
    fireEvent.click(screen.getByTestId('standorte-create'));
    await waitFor(() => {
      expect(mockCreate).toHaveBeenCalledWith({ name: 'Neu', total_slots: 30 });
      expect(mockToast).toHaveBeenCalledWith('Standort angelegt', 'success');
    });
  });

  it('create error surfaces via toast', async () => {
    mockList.mockResolvedValue({ success: true, data: [] });
    mockCreate.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'duplicate' } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('standorte-toggle-form')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('standorte-toggle-form'));
    fireEvent.change(screen.getByTestId('standorte-name'), { target: { value: 'Neu' } });
    fireEvent.change(screen.getByTestId('standorte-slots'), { target: { value: '30' } });
    fireEvent.click(screen.getByTestId('standorte-create'));
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('duplicate', 'error'));
  });

  it('status change calls updateLot', async () => {
    mockList.mockResolvedValue({ success: true, data: [L1] });
    mockUpdate.mockResolvedValue({ success: true, data: { ...L1, status: 'closed' } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('standorte-status')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('standorte-status'), { target: { value: 'closed' } });
    await waitFor(() => {
      expect(mockUpdate).toHaveBeenCalledWith('l1', { status: 'closed' });
      expect(mockToast).toHaveBeenCalledWith('Status aktualisiert', 'success');
    });
  });

  it('delete calls deleteLot', async () => {
    mockList.mockResolvedValue({ success: true, data: [L1] });
    mockDelete.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Löschen')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Löschen'));
    await waitFor(() => {
      expect(mockDelete).toHaveBeenCalledWith('l1');
      expect(mockToast).toHaveBeenCalledWith('Standort gelöscht', 'success');
    });
  });
});
