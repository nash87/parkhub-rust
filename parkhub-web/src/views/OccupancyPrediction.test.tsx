import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';

// ── Mocks ──

vi.mock('../api/client', () => ({
  api: {},
  getInMemoryToken: () => 'test-token',
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'prediction.title': 'Smart Predictions',
        'prediction.subtitle': 'AI-powered occupancy forecasts',
        'prediction.bestTime': 'Best Time to Book',
        'prediction.recommendReason': 'Based on your booking patterns and lot availability',
        'prediction.weeklyForecast': '7-Day Forecast',
        'prediction.peak': 'Peak',
        'prediction.offPeak': 'Off-peak',
        'prediction.confidence': 'confidence',
        'prediction.disclaimer': 'Predictions based on historical patterns. Accuracy improves over time.',
        'prediction.level.low': 'low',
        'prediction.level.medium': 'medium',
        'prediction.level.high': 'high',
      };
      return map[key] || fallback || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, variants, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
}));

vi.mock('@phosphor-icons/react', () => ({
  Sparkle: (props: any) => <span data-testid="icon-sparkle" {...props} />,
  Brain: (props: any) => <span data-testid="icon-brain" {...props} />,
  CalendarBlank: (props: any) => <span data-testid="icon-calendar" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  TrendUp: (props: any) => <span data-testid="icon-trend" {...props} />,
  CaretDown: (props: any) => <span data-testid="icon-caret" {...props} />,
}));

vi.mock('../constants/animations', () => ({
  staggerSlow: { hidden: { opacity: 0 }, show: { opacity: 1 } },
  fadeUp: { hidden: { opacity: 0 }, show: { opacity: 1 } },
}));

import { OccupancyPredictionPage } from './OccupancyPrediction';

const mockLots = [
  { id: 'lot-1', name: 'HQ Garage', total_spots: 50 },
  { id: 'lot-2', name: 'Annex Lot', total_spots: 30 },
];

const mockStats = {
  total_bookings: 200,
  occupancy_by_day: {
    '0': { avg_percentage: 75, peak_hour: 9, peak_percentage: 95, bookings: 40 },
    '1': { avg_percentage: 70, peak_hour: 10, peak_percentage: 90, bookings: 35 },
    '2': { avg_percentage: 65, peak_hour: 9, peak_percentage: 85, bookings: 32 },
    '3': { avg_percentage: 60, peak_hour: 11, peak_percentage: 80, bookings: 28 },
    '4': { avg_percentage: 50, peak_hour: 9, peak_percentage: 70, bookings: 25 },
    '5': { avg_percentage: 20, peak_hour: 11, peak_percentage: 35, bookings: 10 },
    '6': { avg_percentage: 15, peak_hour: 10, peak_percentage: 25, bookings: 5 },
  },
  occupancy_by_hour: {
    '7': 20, '8': 45, '9': 80, '10': 75, '11': 70,
    '12': 50, '13': 55, '14': 40, '15': 35, '16': 30,
  },
};

describe('OccupancyPredictionPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading skeleton initially', () => {
    global.fetch = vi.fn().mockReturnValue(new Promise(() => {}));
    render(<OccupancyPredictionPage />);
    expect(screen.getByTestId('loading')).toBeInTheDocument();
  });

  it('renders page with title', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockLots }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockStats }) });

    render(<OccupancyPredictionPage />);
    await waitFor(() => {
      expect(screen.getByText('Smart Predictions')).toBeInTheDocument();
    });
    expect(screen.getByTestId('prediction-page')).toBeInTheDocument();
  });

  it('renders recommendation card', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockLots }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockStats }) });

    render(<OccupancyPredictionPage />);
    await waitFor(() => {
      expect(screen.getByTestId('recommendation-card')).toBeInTheDocument();
      expect(screen.getByText('Best Time to Book')).toBeInTheDocument();
    });
  });

  it('renders 7 day columns in forecast grid', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockLots }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockStats }) });

    render(<OccupancyPredictionPage />);
    await waitFor(() => {
      const columns = screen.getAllByTestId('day-column');
      expect(columns).toHaveLength(7);
    });
  });

  it('shows lot selector when multiple lots', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockLots }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockStats }) });

    render(<OccupancyPredictionPage />);
    await waitFor(() => {
      expect(screen.getByTestId('lot-selector')).toBeInTheDocument();
    });
  });

  it('does not show lot selector for single lot', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: [mockLots[0]] }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockStats }) });

    render(<OccupancyPredictionPage />);
    await waitFor(() => {
      expect(screen.getByTestId('prediction-page')).toBeInTheDocument();
    });
    expect(screen.queryByTestId('lot-selector')).not.toBeInTheDocument();
  });

  it('displays day names in forecast columns', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockLots }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockStats }) });

    render(<OccupancyPredictionPage />);
    await waitFor(() => {
      expect(screen.getByText('Mon')).toBeInTheDocument();
      expect(screen.getByText('Tue')).toBeInTheDocument();
      expect(screen.getByText('Sun')).toBeInTheDocument();
    });
  });

  it('shows disclaimer text', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockLots }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockStats }) });

    render(<OccupancyPredictionPage />);
    await waitFor(() => {
      expect(screen.getByTestId('disclaimer')).toBeInTheDocument();
      expect(screen.getByText('Predictions based on historical patterns. Accuracy improves over time.')).toBeInTheDocument();
    });
  });

  it('fetches with auth headers and credentials', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: [] }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: {} }) });

    render(<OccupancyPredictionPage />);
    await waitFor(() => screen.getByTestId('prediction-page'));

    expect(global.fetch).toHaveBeenCalledWith(
      '/api/v1/lots',
      expect.objectContaining({
        credentials: 'include',
        headers: expect.objectContaining({ Authorization: 'Bearer test-token' }),
      }),
    );
    expect(global.fetch).toHaveBeenCalledWith(
      '/api/v1/admin/stats',
      expect.objectContaining({ credentials: 'include' }),
    );
  });

  it('shows prediction levels with color coding', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockLots }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockStats }) });

    render(<OccupancyPredictionPage />);
    await waitFor(() => {
      // Monday is 75% = high, Sunday is 15% = low
      expect(screen.getByText('75%')).toBeInTheDocument();
      expect(screen.getByText('15%')).toBeInTheDocument();
    });
  });
});
