import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockGet = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    getAuditLog: (...a: unknown[]) => mockGet(...a),
  },
}));

vi.mock('@number-flow/react', () => ({
  default: ({ value }: { value: number }) => <span>{value}</span>,
}));

import { AuditV5 } from './Audit';

const E1 = { id: 'e1', timestamp: '2026-04-20T10:00:00Z', event_type: 'login', username: 'flo', details: 'OK' };
const E2 = { id: 'e2', timestamp: '2026-04-20T11:00:00Z', event_type: 'update', username: 'anna', details: 'role=admin' };

function renderScreen() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <AuditV5 navigate={vi.fn()} />
    </QueryClientProvider>
  );
}

describe('AuditV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state', async () => {
    mockGet.mockResolvedValue({ success: true, data: { entries: [], total: 0, page: 1, per_page: 25, total_pages: 1 } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine Einträge')).toBeInTheDocument());
  });

  it('renders error state when query fails', async () => {
    mockGet.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders entry rows', async () => {
    mockGet.mockResolvedValue({ success: true, data: { entries: [E1, E2], total: 2, page: 1, per_page: 25, total_pages: 1 } });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('audit-row')).toHaveLength(2));
    expect(screen.getByText('flo')).toBeInTheDocument();
    expect(screen.getByText('role=admin')).toBeInTheDocument();
  });

  it('applies filters and refetches', async () => {
    mockGet.mockResolvedValue({ success: true, data: { entries: [E1], total: 1, page: 1, per_page: 25, total_pages: 1 } });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('audit-row')).toHaveLength(1));
    fireEvent.change(screen.getByTestId('audit-action'), { target: { value: 'login' } });
    fireEvent.change(screen.getByTestId('audit-user'), { target: { value: 'flo' } });
    fireEvent.click(screen.getByTestId('audit-apply'));
    await waitFor(() => {
      expect(mockGet).toHaveBeenCalledWith(expect.objectContaining({ action: 'login', user: 'flo', page: 1 }));
    });
  });

  it('pagination next/prev moves page', async () => {
    mockGet.mockResolvedValue({ success: true, data: { entries: [E1], total: 50, page: 1, per_page: 25, total_pages: 2 } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('audit-next')).not.toBeDisabled());
    fireEvent.click(screen.getByTestId('audit-next'));
    await waitFor(() => {
      expect(mockGet).toHaveBeenCalledWith(expect.objectContaining({ page: 2 }));
    });
  });

  it('reset clears filters', async () => {
    mockGet.mockResolvedValue({ success: true, data: { entries: [E1], total: 1, page: 1, per_page: 25, total_pages: 1 } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('audit-action')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('audit-action'), { target: { value: 'login' } });
    fireEvent.click(screen.getByTestId('audit-reset'));
    await waitFor(() => {
      expect((screen.getByTestId('audit-action') as HTMLInputElement).value).toBe('');
    });
  });

  it('prev button disabled on first page', async () => {
    mockGet.mockResolvedValue({ success: true, data: { entries: [E1], total: 50, page: 1, per_page: 25, total_pages: 2 } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('audit-prev')).toBeDisabled());
  });
});
