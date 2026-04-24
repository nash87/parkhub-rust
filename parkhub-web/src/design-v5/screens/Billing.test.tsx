import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockCostCenter = vi.fn();
const mockDepartment = vi.fn();
const mockConfig = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    adminBillingByCostCenter: (...a: unknown[]) => mockCostCenter(...a),
    adminBillingByDepartment: (...a: unknown[]) => mockDepartment(...a),
    getStripeConfig: (...a: unknown[]) => mockConfig(...a),
  },
}));

vi.mock('@number-flow/react', () => ({
  default: ({ value }: { value: number }) => <span>{value}</span>,
}));

import { BillingV5 } from './Billing';

// Tenant-wide billing fixtures (admin view): aggregates per cost-center and
// per-department as returned by /api/v1/admin/billing/by-cost-center +
// /by-department. Amount is euros (f64), NOT cents as in personal payment
// history — that's the server-side schema from parkhub-server/src/api/billing.rs.
const CC_ROWS = [
  {
    cost_center: 'CC-100', department: 'Engineering',
    user_count: 12, total_bookings: 180,
    total_credits_used: 1800, total_amount: 3600.50,
    currency: 'EUR',
  },
  {
    cost_center: 'CC-200', department: 'Sales',
    user_count: 6, total_bookings: 40,
    total_credits_used: 400, total_amount: 800.00,
    currency: 'EUR',
  },
];

const DEPT_ROWS = [
  {
    department: 'Engineering', user_count: 12, total_bookings: 180,
    total_credits_used: 1800, total_amount: 3600.50, currency: 'EUR',
  },
  {
    department: 'Sales', user_count: 6, total_bookings: 40,
    total_credits_used: 400, total_amount: 800.00, currency: 'EUR',
  },
];

function renderScreen() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <BillingV5 navigate={vi.fn()} />
    </QueryClientProvider>
  );
}

describe('BillingV5 (admin)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockConfig.mockResolvedValue({ success: true, data: { configured: true } });
    mockCostCenter.mockResolvedValue({ success: true, data: CC_ROWS });
    mockDepartment.mockResolvedValue({ success: true, data: DEPT_ROWS });
  });

  it('renders empty state when no cost centers are configured', async () => {
    mockCostCenter.mockResolvedValue({ success: true, data: [] });
    mockDepartment.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText(/keine Kostenstellen/i)).toBeInTheDocument());
  });

  it('renders error state when cost-center query fails', async () => {
    mockCostCenter.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders one row per cost center', async () => {
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('billing-row')).toHaveLength(CC_ROWS.length));
    expect(screen.getByText('CC-100')).toBeInTheDocument();
    expect(screen.getByText('CC-200')).toBeInTheDocument();
  });

  it('shows aggregate totals across all cost centers', async () => {
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('billing-row')).toHaveLength(CC_ROWS.length));
    // Total = 3600.50 + 800.00 = 4400.50 EUR
    expect(screen.getAllByText(/4\.400,50/).length).toBeGreaterThan(0);
    // Total bookings = 220
    expect(screen.getAllByText(/220/).length).toBeGreaterThan(0);
  });

  it('calls the admin billing endpoints on mount (not the personal payment history)', async () => {
    renderScreen();
    await waitFor(() => expect(mockCostCenter).toHaveBeenCalled());
    expect(mockDepartment).toHaveBeenCalled();
  });

  it('surfaces Stripe configured badge from config', async () => {
    mockConfig.mockResolvedValue({ success: true, data: { configured: false } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Nicht konfiguriert')).toBeInTheDocument());
  });
});
