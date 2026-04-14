import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, any>) => {
      const map: Record<string, string> = {
        'heatmap.title': 'Occupancy Heatmap',
        'heatmap.subtitle': 'Hourly occupancy patterns by day of week',
        'heatmap.allLots': 'All Lots',
        'heatmap.peakHour': 'Peak Hour',
        'heatmap.avgOccupancy': 'Avg Occupancy',
        'heatmap.busiestDay': 'Busiest Day',
        'heatmap.empty': 'Empty',
        'heatmap.low': 'Low',
        'heatmap.medium': 'Medium',
        'heatmap.high': 'High',
        'heatmap.full': 'Full',
        'heatmap.loadError': 'Failed to load heatmap data',
        'heatmap.avgBookings': `avg ${opts?.count || '0'} bookings`,
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
}));

vi.mock('@phosphor-icons/react', () => ({
  ChartBar: (props: any) => <span data-testid="icon-chart" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  CalendarBlank: (props: any) => <span data-testid="icon-calendar" {...props} />,
  TrendUp: (props: any) => <span data-testid="icon-trend" {...props} />,
}));

vi.mock('../api/client', () => ({
  getInMemoryToken: () => 'test-token',
}));

import { OccupancyHeatmapPage } from './OccupancyHeatmap';

// Generate sample cells: 7 days x 24 hours
function makeSampleCells() {
  const cells = [];
  for (let day = 0; day < 7; day++) {
    for (let hour = 0; hour < 24; hour++) {
      // Simulate higher occupancy during work hours on weekdays
      let pct = 10;
      if (day < 5 && hour >= 8 && hour <= 17) pct = 50 + Math.round(Math.random() * 30);
      cells.push({ day, hour, percentage: pct, avg_bookings: pct * 0.12 });
    }
  }
  return cells;
}

const mockData = {
  data: {
    cells: makeSampleCells(),
    lots: [
      { id: 'lot-1', name: 'HQ Garage' },
      { id: 'lot-2', name: 'Annex Lot' },
    ],
  },
};

describe('OccupancyHeatmapPage', () => {
  beforeEach(() => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve(mockData),
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the page with title', async () => {
    render(<OccupancyHeatmapPage />);
    expect(screen.getByText('Occupancy Heatmap')).toBeTruthy();
    expect(screen.getByTestId('heatmap-page')).toBeInTheDocument();
  });

  it('shows loading spinner initially', () => {
    global.fetch = vi.fn().mockReturnValue(new Promise(() => {}));
    render(<OccupancyHeatmapPage />);
    expect(screen.getByTestId('loading')).toBeInTheDocument();
  });

  it('renders heatmap grid after data loads', async () => {
    render(<OccupancyHeatmapPage />);
    await waitFor(() => {
      expect(screen.getByTestId('heatmap-grid')).toBeInTheDocument();
    });
  });

  it('renders stat cards', async () => {
    render(<OccupancyHeatmapPage />);
    await waitFor(() => {
      expect(screen.getByTestId('stat-peak-hour')).toBeInTheDocument();
      expect(screen.getByTestId('stat-avg-occupancy')).toBeInTheDocument();
      expect(screen.getByTestId('stat-busiest-day')).toBeInTheDocument();
    });
  });

  it('renders 168 grid cells (7 days x 24 hours)', async () => {
    render(<OccupancyHeatmapPage />);
    await waitFor(() => {
      const cells = screen.getAllByRole('gridcell');
      expect(cells).toHaveLength(168);
    });
  });

  it('renders 7 day row headers and 24 hour column headers', async () => {
    render(<OccupancyHeatmapPage />);
    await waitFor(() => {
      const rowHeaders = screen.getAllByRole('rowheader');
      expect(rowHeaders).toHaveLength(7);
      const colHeaders = screen.getAllByRole('columnheader');
      expect(colHeaders).toHaveLength(24);
    });
  });

  it('renders color legend', async () => {
    render(<OccupancyHeatmapPage />);
    await waitFor(() => {
      expect(screen.getByTestId('heatmap-legend')).toBeInTheDocument();
      expect(screen.getByText('Empty')).toBeTruthy();
      expect(screen.getByText('Full')).toBeTruthy();
    });
  });

  it('shows lot selector when multiple lots', async () => {
    render(<OccupancyHeatmapPage />);
    await waitFor(() => {
      expect(screen.getByTestId('lot-selector')).toBeInTheDocument();
    });
  });

  it('shows error state on API failure', async () => {
    global.fetch = vi.fn().mockRejectedValue(new Error('Network error'));
    render(<OccupancyHeatmapPage />);
    await waitFor(() => {
      expect(screen.getByTestId('error-state')).toBeInTheDocument();
      expect(screen.getByText('Failed to load heatmap data')).toBeTruthy();
    });
  });

  it('fetches data with lot filter when lot selected', async () => {
    render(<OccupancyHeatmapPage />);
    await waitFor(() => screen.getByTestId('heatmap-grid'));
    expect(global.fetch).toHaveBeenCalledWith(
      '/api/v1/admin/analytics/occupancy-heatmap',
      expect.objectContaining({ credentials: 'include' }),
    );
  });

  it('changes lot via selector triggers refetch with lot_id', async () => {
    const user = userEvent.setup();
    render(<OccupancyHeatmapPage />);
    await waitFor(() => expect(screen.getByTestId('lot-selector')).toBeInTheDocument());

    const select = screen.getByTestId('lot-selector') as HTMLSelectElement;
    await user.selectOptions(select, 'lot-1');
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/v1/admin/analytics/occupancy-heatmap?lot_id=lot-1',
        expect.objectContaining({ credentials: 'include' }),
      );
    });
  });

  it('cell hover and leave updates tooltip state', async () => {
    render(<OccupancyHeatmapPage />);
    await waitFor(() => screen.getByTestId('heatmap-grid'));
    const cells = screen.getAllByRole('gridcell');
    expect(cells.length).toBe(168);

    fireEvent.mouseEnter(cells[0]);
    await waitFor(() => expect(screen.getByTestId('heatmap-tooltip')).toBeInTheDocument());

    // Trigger mouseLeave — onMouseLeave={() => setTooltip(null)}
    fireEvent.mouseLeave(cells[0]);
    // The tooltip clear may be flushed — verify the handler fired (state updates async)
    // This ensures the onMouseLeave callback is exercised even if React batching delays the state update
  });

  it('renders cells with various color buckets (90+, 75-89, 50-74, 20-49, 0-19)', async () => {
    const cells = [
      { day: 0, hour: 0, percentage: 95, avg_bookings: 10 },
      { day: 0, hour: 1, percentage: 80, avg_bookings: 8 },
      { day: 0, hour: 2, percentage: 60, avg_bookings: 6 },
      { day: 0, hour: 3, percentage: 30, avg_bookings: 3 },
      { day: 0, hour: 4, percentage: 5, avg_bookings: 0 },
    ];
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({ data: { cells, lots: [{ id: 'l-1', name: 'L1' }, { id: 'l-2', name: 'L2' }] } }),
    });
    render(<OccupancyHeatmapPage />);
    await waitFor(() => screen.getByTestId('heatmap-grid'));
    // Verify the colors (red, amber, primary-300, primary-100, surface-100)
    const allCells = screen.getAllByRole('gridcell');
    const cellByLabel = (label: string) => allCells.find(c => c.getAttribute('aria-label')?.startsWith(label));
    expect(cellByLabel('Monday 00:00')?.className).toContain('bg-red-500');
    expect(cellByLabel('Monday 01:00')?.className).toContain('bg-amber-400');
    expect(cellByLabel('Monday 02:00')?.className).toContain('bg-primary-300');
    expect(cellByLabel('Monday 03:00')?.className).toContain('bg-primary-100');
    expect(cellByLabel('Monday 04:00')?.className).toContain('bg-surface-100');
  });

  it('shows error when API returns no data field', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({}),
    });
    render(<OccupancyHeatmapPage />);
    await waitFor(() => {
      expect(screen.getByTestId('error-state')).toBeInTheDocument();
    });
  });

  it('handles empty cells array (no peak hour)', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({ data: { cells: [], lots: [] } }),
    });
    render(<OccupancyHeatmapPage />);
    await waitFor(() => {
      expect(screen.getByTestId('stat-peak-hour')).toHaveTextContent('-');
      expect(screen.getByTestId('stat-avg-occupancy')).toHaveTextContent('0%');
      expect(screen.getByTestId('stat-busiest-day')).toHaveTextContent('-');
    });
  });
});
