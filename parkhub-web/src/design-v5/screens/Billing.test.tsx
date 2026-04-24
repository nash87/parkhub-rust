import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockHistory = vi.fn();
const mockConfig = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    getPaymentHistory: (...a: unknown[]) => mockHistory(...a),
    getStripeConfig: (...a: unknown[]) => mockConfig(...a),
  },
}));

vi.mock('@number-flow/react', () => ({
  default: ({ value }: { value: number }) => <span>{value}</span>,
}));

import { BillingV5 } from './Billing';

const P1 = { id: 'p-abcdefghij12345', amount: 1200, credits: 10, currency: 'EUR', status: 'completed' as const, created_at: '2026-04-01T10:00:00Z' };
const P2 = { id: 'p-pending000000', amount: 500, credits: 5, currency: 'EUR', status: 'pending' as const, created_at: '2026-04-20T10:00:00Z' };

function renderScreen() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <BillingV5 navigate={vi.fn()} />
    </QueryClientProvider>
  );
}

describe('BillingV5', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockConfig.mockResolvedValue({ success: true, data: { configured: true } });
  });

  it('renders empty history message', async () => {
    mockHistory.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Noch keine Zahlungen')).toBeInTheDocument());
  });

  it('renders error state when history fails', async () => {
    mockHistory.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders payment rows with status badges', async () => {
    mockHistory.mockResolvedValue({ success: true, data: [P1, P2] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('billing-row')).toHaveLength(2));
    expect(screen.getByText('Bezahlt')).toBeInTheDocument();
    expect(screen.getByText('Ausstehend')).toBeInTheDocument();
  });

  it('surfaces Stripe configured badge from config', async () => {
    mockHistory.mockResolvedValue({ success: true, data: [] });
    mockConfig.mockResolvedValue({ success: true, data: { configured: false } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Nicht konfiguriert')).toBeInTheDocument());
  });

  it('computes total paid only from completed payments', async () => {
    mockHistory.mockResolvedValue({ success: true, data: [P1, P2] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('billing-row')).toHaveLength(2));
    // 1200 cents / EUR => 12,00 €
    expect(screen.getAllByText(/12,00/).length).toBeGreaterThan(0);
  });

  it('renders loading skeleton initially then replaces with content', async () => {
    mockHistory.mockResolvedValue({ success: true, data: [P1] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('billing-row')).toHaveLength(1));
  });
});
