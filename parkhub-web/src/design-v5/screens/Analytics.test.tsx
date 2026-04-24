import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockStats = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    getAdminStatsExtended: (...a: unknown[]) => mockStats(...a),
  },
}));

vi.mock('@number-flow/react', () => ({
  default: ({ value }: { value: number }) => <span>{value}</span>,
}));

import { AnalyticsV5 } from './Analytics';

const STATS = {
  total_users: 42,
  total_lots: 5,
  total_bookings: 1200,
  active_bookings: 7,
  occupancy_by_hour: { '08': 40, '09': 60, '10': 75 },
  occupancy_by_day: {
    Mo: { avg_percentage: 55, peak_hour: 10, peak_percentage: 80, bookings: 120 },
    Di: { avg_percentage: 65, peak_hour: 9, peak_percentage: 85, bookings: 140 },
  },
};

function renderScreen() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <AnalyticsV5 navigate={vi.fn()} />
    </QueryClientProvider>
  );
}

describe('AnalyticsV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders error state when stats fail', async () => {
    mockStats.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders KPI cards from stats', async () => {
    mockStats.mockResolvedValue({ success: true, data: STATS });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Nutzer')).toBeInTheDocument());
    expect(screen.getByText('42')).toBeInTheDocument();
    expect(screen.getByText('Aktive Buchungen')).toBeInTheDocument();
  });

  it('renders both bar charts', async () => {
    mockStats.mockResolvedValue({ success: true, data: STATS });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('analytics-chart')).toHaveLength(2));
  });

  it('hour chart renders 24 bars', async () => {
    mockStats.mockResolvedValue({ success: true, data: STATS });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('analytics-chart').length).toBeGreaterThan(0));
    const charts = screen.getAllByTestId('analytics-chart');
    // hour chart should have rects for 24 hours
    const hourRects = charts[0].querySelectorAll('rect');
    expect(hourRects.length).toBe(24);
  });

  it('gracefully handles missing occupancy data', async () => {
    mockStats.mockResolvedValue({ success: true, data: { total_users: 1, total_lots: 1, total_bookings: 1, active_bookings: 1 } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Nutzer')).toBeInTheDocument());
    // Should still render chart containers (with 0 values)
    expect(screen.getAllByTestId('analytics-chart')).toHaveLength(2);
  });

  it('renders title', async () => {
    mockStats.mockResolvedValue({ success: true, data: STATS });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Analytics')).toBeInTheDocument());
  });
});
