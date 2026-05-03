import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    // Mirror i18next's real behavior: the second arg is the English fallback,
    // and an opts object's interpolation values replace `{{name}}` placeholders.
    t: (key: string, defaultOrOpts?: string | Record<string, unknown>, opts?: Record<string, unknown>) => {
      const isFallback = typeof defaultOrOpts === 'string';
      let text = isFallback ? defaultOrOpts : key;
      const interp = isFallback ? opts : (defaultOrOpts as Record<string, unknown> | undefined);
      if (interp) {
        for (const [k, v] of Object.entries(interp)) {
          text = text.replace(new RegExp(`{{\\s*${k}\\s*}}`, 'g'), String(v));
        }
      }
      return text;
    },
    i18n: { language: 'en', changeLanguage: vi.fn() },
  }),
}));

vi.mock('../api/client', () => ({
  getInMemoryToken: () => 'test-token',
}));

vi.mock('@phosphor-icons/react', () => ({
  ChartBarIcon: (p: any) => <span {...p} />,
  TrendUpIcon: (p: any) => <span {...p} />,
  UsersIcon: (p: any) => <span {...p} />,
  ClockIcon: (p: any) => <span {...p} />,
  CurrencyDollarIcon: (p: any) => <span {...p} />,
  ExportIcon: (p: any) => <span {...p} />,
  CalendarBlankIcon: (p: any) => <span {...p} />,
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

  it('exports CSV when clicking export button', async () => {
    const createObjectURL = vi.fn(() => 'blob:test');
    const revokeObjectURL = vi.fn();
    Object.defineProperty(URL, 'createObjectURL', { value: createObjectURL, writable: true });
    Object.defineProperty(URL, 'revokeObjectURL', { value: revokeObjectURL, writable: true });

    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve(mockData),
    });
    const { fireEvent } = await import('@testing-library/react');
    render(<AdminAnalyticsPage />);
    await waitFor(() => expect(screen.getByText('Total Bookings')).toBeTruthy());

    const csvBtn = screen.getByText('CSV');
    fireEvent.click(csvBtn);

    expect(createObjectURL).toHaveBeenCalled();
    expect(revokeObjectURL).toHaveBeenCalled();
  });

  it('changes date range when clicking range buttons', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve(mockData),
    });
    const { fireEvent } = await import('@testing-library/react');
    render(<AdminAnalyticsPage />);

    const btn90 = screen.getByText('90d');
    fireEvent.click(btn90);

    // Should refetch with new range
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledTimes(2); // initial + range change
    });
  });

  it('handles empty data series (MiniBarChart returns null)', async () => {
    const emptyData = {
      data: {
        ...mockData.data,
        daily_bookings: [],
        daily_revenue: [],
        peak_hours: [],
        user_growth: [],
      },
    };
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve(emptyData),
    });
    render(<AdminAnalyticsPage />);
    await waitFor(() => expect(screen.getByText('Total Bookings')).toBeTruthy());
  });

  it('CSV export does nothing when no data', async () => {
    global.fetch = vi.fn().mockReturnValue(new Promise(() => {}));
    const createObjectURL = vi.fn();
    Object.defineProperty(URL, 'createObjectURL', { value: createObjectURL, writable: true });

    const { fireEvent } = await import('@testing-library/react');
    render(<AdminAnalyticsPage />);

    const csvBtn = screen.getByText('CSV');
    fireEvent.click(csvBtn);

    expect(createObjectURL).not.toHaveBeenCalled();
  });
});
