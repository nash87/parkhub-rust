import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

// ── Mocks ──

const mockGetUserCredits = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    getUserCredits: (...args: any[]) => mockGetUserCredits(...args),
  },
}));

vi.mock('../context/AuthContext', () => ({
  useAuth: () => ({
    user: {
      id: 'u-1',
      username: 'jdoe',
      name: 'John',
      email: 'john@test.com',
      role: 'user',
      credits_balance: 7,
      credits_monthly_quota: 10,
    },
  }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => {
      const map: Record<string, string> = {
        'credits.title': 'Credits',
        'credits.subtitle': 'Your credit balance',
        'credits.balance': 'Balance',
        'credits.monthlyQuota': 'Monthly Quota',
        'credits.used': 'Used',
        'credits.lastRefill': 'Last Refill',
        'credits.history': 'History',
        'credits.noTransactions': 'No transactions yet',
        'credits.creditsPerBooking': `${opts?.count ?? 1} credit per booking`,
        'credits.grant': 'Grant',
        'credits.deduction': 'Deduction',
        'credits.monthly_refill': 'Monthly Refill',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, variants, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
}));

vi.mock('@phosphor-icons/react', () => ({
  Coins: (props: any) => <span data-testid="icon-coins" {...props} />,
  ArrowDown: (props: any) => <span data-testid="icon-arrow-down" {...props} />,
  ArrowUp: (props: any) => <span data-testid="icon-arrow-up" {...props} />,
  ArrowClockwise: (props: any) => <span data-testid="icon-clockwise" {...props} />,
  TrendUp: (props: any) => <span data-testid="icon-trend-up" {...props} />,
  Sparkle: (props: any) => <span data-testid="icon-sparkle" {...props} />,
}));

vi.mock('../constants/animations', () => ({
  staggerSlow: { hidden: {}, show: {} },
  fadeUp: { hidden: {}, show: {} },
}));

import { CreditsPage } from './Credits';

describe('CreditsPage', () => {
  beforeEach(() => {
    mockGetUserCredits.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading skeleton initially', () => {
    mockGetUserCredits.mockReturnValue(new Promise(() => {}));
    render(<CreditsPage />);
    // Skeleton uses className "skeleton"
    const skeletons = document.querySelectorAll('.skeleton');
    expect(skeletons.length).toBeGreaterThan(0);
  });

  it('renders credits page with balance', async () => {
    mockGetUserCredits.mockResolvedValue({
      success: true,
      data: {
        enabled: true,
        balance: 7,
        monthly_quota: 10,
        last_refilled: '2026-03-01T00:00:00Z',
        transactions: [],
      },
    });

    render(<CreditsPage />);

    await waitFor(() => {
      expect(screen.getByText('Credits')).toBeInTheDocument();
    });
    expect(screen.getByText('Balance')).toBeInTheDocument();
    expect(screen.getByText('7')).toBeInTheDocument();
    expect(screen.getByText('/ 10')).toBeInTheDocument();
  });

  it('renders stat cards with quota and used', async () => {
    mockGetUserCredits.mockResolvedValue({
      success: true,
      data: {
        enabled: true,
        balance: 6,
        monthly_quota: 10,
        last_refilled: '2026-03-01T00:00:00Z',
        transactions: [],
      },
    });

    render(<CreditsPage />);

    await waitFor(() => {
      expect(screen.getByText('Monthly Quota')).toBeInTheDocument();
    });
    expect(screen.getByText('Used')).toBeInTheDocument();
    expect(screen.getByText('Last Refill')).toBeInTheDocument();
    // Quota = 10, Used = 10 - 6 = 4
    expect(screen.getByText('10')).toBeInTheDocument();
    expect(screen.getByText('4')).toBeInTheDocument();
  });

  it('shows empty transaction history', async () => {
    mockGetUserCredits.mockResolvedValue({
      success: true,
      data: {
        enabled: true,
        balance: 5,
        monthly_quota: 10,
        transactions: [],
      },
    });

    render(<CreditsPage />);

    await waitFor(() => {
      expect(screen.getByText('No transactions yet')).toBeInTheDocument();
    });
  });

  it('renders transaction list when transactions exist', async () => {
    mockGetUserCredits.mockResolvedValue({
      success: true,
      data: {
        enabled: true,
        balance: 8,
        monthly_quota: 10,
        last_refilled: '2026-03-01T00:00:00Z',
        transactions: [
          { id: 't-1', amount: 10, type: 'monthly_refill', description: 'Monthly refill', created_at: '2026-03-01T00:00:00Z' },
          { id: 't-2', amount: -2, type: 'deduction', description: 'Booking #42', created_at: '2026-03-05T00:00:00Z' },
        ],
      },
    });

    render(<CreditsPage />);

    await waitFor(() => {
      expect(screen.getByText('History')).toBeInTheDocument();
    });
    expect(screen.getByText('Monthly Refill')).toBeInTheDocument();
    expect(screen.getByText('Deduction')).toBeInTheDocument();
    expect(screen.getByText('+10')).toBeInTheDocument();
    expect(screen.getByText('-2')).toBeInTheDocument();
  });

  it('falls back to user context when API returns no data', async () => {
    mockGetUserCredits.mockResolvedValue({ success: false, data: null });

    render(<CreditsPage />);

    await waitFor(() => {
      expect(screen.getByText('Credits')).toBeInTheDocument();
    });
    // Falls back to user.credits_balance = 7
    expect(screen.getByText('7')).toBeInTheDocument();
  });
});
