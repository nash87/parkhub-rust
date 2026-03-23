import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key, i18n: { language: 'en', changeLanguage: vi.fn() } }),
}));

vi.mock('../api/client', () => ({
  getInMemoryToken: () => 'test-token',
}));

vi.mock('@phosphor-icons/react', () => ({
  ChartBar: (p: any) => <span {...p} />,
  TrendUp: (p: any) => <span {...p} />,
  Users: (p: any) => <span {...p} />,
  Clock: (p: any) => <span {...p} />,
  CurrencyDollar: (p: any) => <span {...p} />,
  Export: (p: any) => <span {...p} />,
  CalendarBlank: (p: any) => <span {...p} />,
}));

import { AdminAnalyticsPage } from './AdminAnalytics';

const mockData = {
  data: {
    daily_bookings: [
      { date: '2026-03-20', value: 5 },
      { date: '2026-03-21', value: 8 },
    ],
    daily_revenue: [
      { date: '2026-03-20', value: 25 },
      { date: '2026-03-21', value: 40 },
    ],
    peak_hours: Array.from({ length: 24 }, (_, h) => ({ hour: h, count: h * 3 })),
    top_lots: [
      { lot_id: 'lot-1', lot_name: 'HQ Garage', total_slots: 50, bookings: 120, utilization_percent: 80.0 },
      { lot_id: 'lot-2', lot_name: 'Annex', total_slots: 20, bookings: 30, utilization_percent: 50.0 },
    ],
    user_growth: [
      { month: '2026-01', count: 5 },
      { month: '2026-02', count: 8 },
      { month: '2026-03', count: 12 },
    ],
    avg_booking_duration_minutes: 180,
    total_bookings: 13,
    total_revenue: 65,
    active_users: 7,
  },
};

beforeEach(() => {
  vi.restoreAllMocks();
});

describe('AdminAnalyticsPage', () => {
  it('renders analytics page with title', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve(mockData),
    });
    render(<AdminAnalyticsPage />);
    expect(screen.getByText('Analytics')).toBeTruthy();
  });

  it('shows loading skeletons initially', () => {
    global.fetch = vi.fn().mockReturnValue(new Promise(() => {})); // never resolves
    render(<AdminAnalyticsPage />);
    expect(screen.getByTestId('admin-analytics')).toBeTruthy();
  });

  it('renders stat cards after data loads', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve(mockData),
    });
    render(<AdminAnalyticsPage />);
    await waitFor(() => expect(screen.getByText('Total Bookings')).toBeTruthy());
    expect(screen.getByText('Total Revenue')).toBeTruthy();
    expect(screen.getByText('Avg Duration')).toBeTruthy();
    expect(screen.getByText('Active Users')).toBeTruthy();
  });

  it('renders date range buttons', () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve(mockData),
    });
    render(<AdminAnalyticsPage />);
    expect(screen.getByText('7d')).toBeTruthy();
    expect(screen.getByText('30d')).toBeTruthy();
    expect(screen.getByText('90d')).toBeTruthy();
    expect(screen.getByText('1y')).toBeTruthy();
  });

  it('renders top lots after load', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve(mockData),
    });
    render(<AdminAnalyticsPage />);
    await waitFor(() => expect(screen.getByText('HQ Garage')).toBeTruthy());
    expect(screen.getByText('Annex')).toBeTruthy();
  });

  it('renders CSV export button', () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve(mockData),
    });
    render(<AdminAnalyticsPage />);
    expect(screen.getByText('CSV')).toBeTruthy();
  });

  it('handles API error gracefully', async () => {
    global.fetch = vi.fn().mockRejectedValue(new Error('Network error'));
    render(<AdminAnalyticsPage />);
    await waitFor(() =>
      expect(screen.getByText('Failed to load analytics data')).toBeTruthy()
    );
  });
});
