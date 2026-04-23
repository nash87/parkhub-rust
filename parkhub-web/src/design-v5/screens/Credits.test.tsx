import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockGetUserCredits = vi.fn();
const mockCreateCheckout = vi.fn();

vi.mock('../../api/client', () => ({
  api: {
    getUserCredits: (...a: unknown[]) => mockGetUserCredits(...a),
    createCheckout: (...a: unknown[]) => mockCreateCheckout(...a),
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

import { CreditsV5 } from './Credits';

function renderScreen(navigate = vi.fn()) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <CreditsV5 navigate={navigate} />
    </QueryClientProvider>
  );
}

const CREDITS = {
  enabled: true,
  balance: 7,
  monthly_quota: 10,
  last_refilled: '2026-04-01T00:00:00Z',
  transactions: [
    { id: 'tx-1', amount: 10, type: 'monthly_refill' as const, description: undefined, created_at: '2026-04-01T00:00:00Z' },
    { id: 'tx-2', amount: -1, type: 'deduction' as const, description: 'Buchung BK-123', created_at: '2026-04-05T00:00:00Z' },
    { id: 'tx-3', amount: -2, type: 'deduction' as const, description: undefined, created_at: '2026-04-10T00:00:00Z' },
  ],
};

describe('CreditsV5', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    Object.defineProperty(window, 'location', { writable: true, value: { href: '' } });
  });

  it('renders error state when query fails', async () => {
    mockGetUserCredits.mockRejectedValue(new Error('network'));
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders hero balance + stat cards', async () => {
    mockGetUserCredits.mockResolvedValue({ success: true, data: CREDITS });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Monatl. Kontingent')).toBeInTheDocument());
    expect(screen.getByText('Verbraucht')).toBeInTheDocument();
    expect(screen.getByText('Letzte Aufladung')).toBeInTheDocument();
  });

  it('renders transactions with correct sign prefix', async () => {
    mockGetUserCredits.mockResolvedValue({ success: true, data: CREDITS });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Monatliche Aufladung')).toBeInTheDocument());
    expect(screen.getByText('+10')).toBeInTheDocument();
    expect(screen.getByText('-1')).toBeInTheDocument();
    expect(screen.getByText('-2')).toBeInTheDocument();
  });

  it('renders empty transactions state', async () => {
    mockGetUserCredits.mockResolvedValue({ success: true, data: { ...CREDITS, transactions: [] } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine Transaktionen')).toBeInTheDocument());
  });

  it('buy button triggers createCheckout + redirect', async () => {
    mockGetUserCredits.mockResolvedValue({ success: true, data: CREDITS });
    mockCreateCheckout.mockResolvedValue({
      success: true,
      data: { id: 'cs_1', checkout_url: 'https://checkout.stripe.com/test', amount: 10, credits: 10, currency: 'EUR' },
    });
    renderScreen();
    await waitFor(() => screen.getByText('+ Credits kaufen'));
    fireEvent.click(screen.getByText('+ Credits kaufen'));
    await waitFor(() => {
      expect(mockCreateCheckout).toHaveBeenCalledWith(10);
      expect(mockToast).toHaveBeenCalledWith('Weiterleitung zur Kasse…', 'info');
    });
  });

  it('meter has ARIA valuenow/valuemin/valuemax', async () => {
    mockGetUserCredits.mockResolvedValue({ success: true, data: CREDITS });
    renderScreen();
    await waitFor(() => {
      const meter = screen.getByRole('meter');
      expect(meter).toHaveAttribute('aria-valuenow', '7');
      expect(meter).toHaveAttribute('aria-valuemin', '0');
      expect(meter).toHaveAttribute('aria-valuemax', '10');
    });
  });
});
