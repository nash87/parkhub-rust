import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

// -- Mocks --

const mockGetRateLimitStats = vi.fn();
const mockGetRateLimitHistory = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    getRateLimitStats: (...args: any[]) => mockGetRateLimitStats(...args),
    getRateLimitHistory: (...args: any[]) => mockGetRateLimitHistory(...args),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string, opts?: any) => {
      const map: Record<string, string> = {
        'rateLimits.title': 'Rate Limits',
        'rateLimits.allClear': 'No blocked requests',
        'rateLimits.blockedHistory': 'Blocked Requests (24h)',
        'rateLimits.perMinute': 'min',
        'rateLimits.blocked': 'blocked',
        'rateLimits.now': 'now',
      };
      if (key === 'rateLimits.blockedTotal' && opts?.count !== undefined) {
        return `${opts.count} blocked (last hour)`;
      }
      return map[key] || fallback || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  ShieldCheck: (props: any) => <span data-testid="icon-shield" {...props} />,
  Warning: (props: any) => <span data-testid="icon-warning" {...props} />,
}));

import { AdminRateLimitsPage } from './AdminRateLimits';

const sampleStats = {
  groups: [
    { group: 'auth', limit_per_minute: 5, description: 'Authentication', current_count: 2, reset_seconds: 60, blocked_last_hour: 0 },
    { group: 'api', limit_per_minute: 100, description: 'General API', current_count: 45, reset_seconds: 60, blocked_last_hour: 3 },
    { group: 'public', limit_per_minute: 30, description: 'Public endpoints', current_count: 0, reset_seconds: 60, blocked_last_hour: 0 },
    { group: 'webhook', limit_per_minute: 50, description: 'Webhooks', current_count: 0, reset_seconds: 60, blocked_last_hour: 0 },
  ],
  total_blocked_last_hour: 3,
};

const sampleHistory = {
  bins: Array.from({ length: 24 }, (_, i) => ({
    hour: `2026-03-22T${String(i).padStart(2, '0')}:00`,
    count: i === 14 ? 5 : 0,
  })),
};

describe('AdminRateLimitsPage', () => {
  beforeEach(() => {
    mockGetRateLimitStats.mockClear();
    mockGetRateLimitHistory.mockClear();
    mockGetRateLimitStats.mockResolvedValue({ success: true, data: sampleStats });
    mockGetRateLimitHistory.mockResolvedValue({ success: true, data: sampleHistory });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the title after loading', async () => {
    render(<AdminRateLimitsPage />);
    await waitFor(() => {
      expect(screen.getByText('Rate Limits')).toBeInTheDocument();
    });
  });

  it('renders all four rate limit group cards', async () => {
    render(<AdminRateLimitsPage />);
    await waitFor(() => {
      expect(screen.getByTestId('rate-group-auth')).toBeInTheDocument();
      expect(screen.getByTestId('rate-group-api')).toBeInTheDocument();
      expect(screen.getByTestId('rate-group-public')).toBeInTheDocument();
      expect(screen.getByTestId('rate-group-webhook')).toBeInTheDocument();
    });
  });

  it('shows blocked count when there are blocked requests', async () => {
    render(<AdminRateLimitsPage />);
    await waitFor(() => {
      expect(screen.getByText('3 blocked (last hour)')).toBeInTheDocument();
    });
  });

  it('renders the 24h blocked requests chart', async () => {
    render(<AdminRateLimitsPage />);
    await waitFor(() => {
      expect(screen.getByText('Blocked Requests (24h)')).toBeInTheDocument();
      expect(screen.getByTestId('blocked-chart')).toBeInTheDocument();
    });
  });

  it('shows "No blocked requests" when total is 0', async () => {
    mockGetRateLimitStats.mockResolvedValue({
      success: true,
      data: { ...sampleStats, total_blocked_last_hour: 0 },
    });
    render(<AdminRateLimitsPage />);
    await waitFor(() => {
      expect(screen.getByText('No blocked requests')).toBeInTheDocument();
    });
  });
});
