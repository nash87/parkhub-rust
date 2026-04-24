import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockList = vi.fn();
const mockUpdate = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    adminUsers: (...a: unknown[]) => mockList(...a),
    adminUpdateUser: (...a: unknown[]) => mockUpdate(...a),
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

import { NutzerV5 } from './Nutzer';

const U1 = { id: 'u1', username: 'flo', email: 'flo@e', name: 'Flo', role: 'admin' as const, preferences: {}, is_active: true, credits_balance: 0, credits_monthly_quota: 0 };
const U2 = { id: 'u2', username: 'anna', email: 'anna@e', name: 'Anna', role: 'user' as const, preferences: {}, is_active: false, credits_balance: 0, credits_monthly_quota: 0 };

function renderScreen() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <NutzerV5 navigate={vi.fn()} />
    </QueryClientProvider>
  );
}

describe('NutzerV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state', async () => {
    mockList.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine Nutzer gefunden')).toBeInTheDocument());
  });

  it('renders error state when query fails', async () => {
    mockList.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders rows with role and status badges', async () => {
    mockList.mockResolvedValue({ success: true, data: [U1, U2] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('nutzer-row')).toHaveLength(2));
    expect(screen.getByText('Flo')).toBeInTheDocument();
    // "Gesperrt" appears as badge + filter chip — ensure at least one occurrence
    expect(screen.getAllByText('Gesperrt').length).toBeGreaterThan(0);
  });

  it('filters to admins', async () => {
    mockList.mockResolvedValue({ success: true, data: [U1, U2] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('nutzer-row')).toHaveLength(2));
    fireEvent.click(screen.getByRole('button', { name: 'Admins' }));
    await waitFor(() => expect(screen.getAllByTestId('nutzer-row')).toHaveLength(1));
  });

  it('suspend button calls update with is_active:false', async () => {
    mockList.mockResolvedValue({ success: true, data: [U1] });
    mockUpdate.mockResolvedValue({ success: true, data: { ...U1, is_active: false } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Sperren')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Sperren'));
    await waitFor(() => {
      expect(mockUpdate).toHaveBeenCalledWith('u1', { is_active: false });
      expect(mockToast).toHaveBeenCalledWith('Nutzer gesperrt', 'success');
    });
  });

  it('activate mutation error surfaces via toast', async () => {
    mockList.mockResolvedValue({ success: true, data: [U2] });
    mockUpdate.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'nope' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Aktivieren')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Aktivieren'));
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('nope', 'error'));
  });

  it('search filters by name', async () => {
    mockList.mockResolvedValue({ success: true, data: [U1, U2] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('nutzer-row')).toHaveLength(2));
    fireEvent.change(screen.getByTestId('nutzer-search'), { target: { value: 'anna' } });
    await waitFor(() => expect(screen.getAllByTestId('nutzer-row')).toHaveLength(1));
  });
});
