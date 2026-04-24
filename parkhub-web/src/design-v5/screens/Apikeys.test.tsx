import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockList = vi.fn();
const mockCreate = vi.fn();
const mockRotate = vi.fn();
const mockRevoke = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    getApiKeys: (...a: unknown[]) => mockList(...a),
    createApiKey: (...a: unknown[]) => mockCreate(...a),
    rotateApiKey: (...a: unknown[]) => mockRotate(...a),
    revokeApiKey: (...a: unknown[]) => mockRevoke(...a),
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

import { ApikeysV5 } from './Apikeys';

const K1 = { id: 'k1', label: 'CI', masked_key: 'ph_abc***xyz', last_used_at: '2026-04-20T10:00:00Z', created_at: '2026-01-01T00:00:00Z' };

function renderScreen() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <ApikeysV5 navigate={vi.fn()} />
    </QueryClientProvider>
  );
}

describe('ApikeysV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state', async () => {
    mockList.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine Schlüssel')).toBeInTheDocument());
  });

  it('renders error state', async () => {
    mockList.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'fail' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders key rows with masked value', async () => {
    mockList.mockResolvedValue({ success: true, data: [K1] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('apikeys-row')).toHaveLength(1));
    expect(screen.getByText('CI')).toBeInTheDocument();
    expect(screen.getByText('ph_abc***xyz')).toBeInTheDocument();
  });

  it('create shows revealed token banner', async () => {
    mockList.mockResolvedValue({ success: true, data: [] });
    mockCreate.mockResolvedValue({ success: true, data: { ...K1, id: 'new', token: 'ph_FULL_TOKEN_123' } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('apikeys-label')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('apikeys-label'), { target: { value: 'Deploy' } });
    fireEvent.click(screen.getByTestId('apikeys-create'));
    await waitFor(() => expect(screen.getByTestId('apikeys-revealed')).toBeInTheDocument());
    expect(screen.getByText('ph_FULL_TOKEN_123')).toBeInTheDocument();
    expect(mockToast).toHaveBeenCalledWith('API-Schlüssel erstellt', 'success');
  });

  it('create error surfaces via toast', async () => {
    mockList.mockResolvedValue({ success: true, data: [] });
    mockCreate.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'limit' } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('apikeys-label')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('apikeys-label'), { target: { value: 'Deploy' } });
    fireEvent.click(screen.getByTestId('apikeys-create'));
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('limit', 'error'));
  });

  it('rotate calls rotateApiKey', async () => {
    mockList.mockResolvedValue({ success: true, data: [K1] });
    mockRotate.mockResolvedValue({ success: true, data: { ...K1, token: 'ph_NEW' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Rotieren')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Rotieren'));
    await waitFor(() => {
      expect(mockRotate).toHaveBeenCalledWith('k1');
      expect(mockToast).toHaveBeenCalledWith('Schlüssel rotiert', 'success');
    });
  });

  it('revoke calls revokeApiKey', async () => {
    mockList.mockResolvedValue({ success: true, data: [K1] });
    mockRevoke.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Widerrufen')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Widerrufen'));
    await waitFor(() => {
      expect(mockRevoke).toHaveBeenCalledWith('k1');
      expect(mockToast).toHaveBeenCalledWith('Schlüssel widerrufen', 'success');
    });
  });

  it('revoke error surfaces via toast', async () => {
    mockList.mockResolvedValue({ success: true, data: [K1] });
    mockRevoke.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'forbidden' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Widerrufen')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Widerrufen'));
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('forbidden', 'error'));
  });
});
