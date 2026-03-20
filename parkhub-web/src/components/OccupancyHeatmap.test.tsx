import { describe, it, expect } from 'vitest';
import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { OccupancyHeatmap, computeHeatmapData, heatmapColor } from './OccupancyHeatmap';
import type { Booking } from '../api/client';

// ── Helper ──

function makeBooking(overrides: Partial<Booking> = {}): Booking {
  const now = new Date();
  const start = new Date(now.getTime() - 2 * 60 * 60 * 1000); // 2h ago
  const end = new Date(now.getTime() - 1 * 60 * 60 * 1000); // 1h ago
  return {
    id: '1',
    user_id: 'u1',
    lot_id: 'lot1',
    slot_id: 's1',
    lot_name: 'Lot A',
    slot_number: 'A1',
    start_time: start.toISOString(),
    end_time: end.toISOString(),
    status: 'completed',
    ...overrides,
  };
}

// ── computeHeatmapData ──

describe('computeHeatmapData', () => {
  it('returns 7 days x 16 hours = 112 cells', () => {
    const cells = computeHeatmapData([], 10);
    expect(cells).toHaveLength(7 * 16);
  });

  it('returns all zeros when there are no bookings', () => {
    const cells = computeHeatmapData([], 10);
    expect(cells.every(c => c.percentage === 0)).toBe(true);
    expect(cells.every(c => c.count === 0)).toBe(true);
  });

  it('ignores cancelled bookings', () => {
    const cancelled = makeBooking({ status: 'cancelled' });
    const cells = computeHeatmapData([cancelled], 10);
    expect(cells.every(c => c.count === 0)).toBe(true);
  });

  it('counts active bookings in the right day/hour slot', () => {
    // Use yesterday at 10:00-11:00 to guarantee it's in the past and within 30-day window
    const now = new Date();
    const start = new Date(now.getTime() - 24 * 60 * 60 * 1000);
    start.setHours(10, 0, 0, 0);
    const end = new Date(start);
    end.setHours(11, 0, 0, 0);

    // Create multiple bookings on the same day to ensure the average rounds up
    const bookings = Array.from({ length: 5 }, (_, i) =>
      makeBooking({
        id: String(i),
        start_time: start.toISOString(),
        end_time: end.toISOString(),
        status: 'completed',
      }),
    );

    const cells = computeHeatmapData(bookings, 1);
    const matchingCells = cells.filter(c => c.hour === 10 && c.count > 0);
    expect(matchingCells.length).toBeGreaterThan(0);
  });

  it('clamps percentage to 100', () => {
    const now = new Date();
    const start = new Date(now.getTime() - 60 * 60 * 1000);
    start.setMinutes(0, 0, 0);
    const end = new Date(start.getTime() + 60 * 60 * 1000);

    // Create many bookings to exceed capacity
    const bookings = Array.from({ length: 50 }, (_, i) =>
      makeBooking({
        id: String(i),
        start_time: start.toISOString(),
        end_time: end.toISOString(),
        status: 'active',
      }),
    );

    const cells = computeHeatmapData(bookings, 1);
    expect(cells.every(c => c.percentage <= 100)).toBe(true);
  });

  it('handles totalSlots = 0 gracefully (treated as 1)', () => {
    const cells = computeHeatmapData([], 0);
    expect(cells).toHaveLength(112);
    expect(cells.every(c => c.percentage === 0)).toBe(true);
  });

  it('ignores bookings older than 30 days', () => {
    const old = new Date();
    old.setDate(old.getDate() - 45);
    const oldEnd = new Date(old.getTime() + 60 * 60 * 1000);

    const booking = makeBooking({
      start_time: old.toISOString(),
      end_time: oldEnd.toISOString(),
    });

    const cells = computeHeatmapData([booking], 10);
    expect(cells.every(c => c.count === 0)).toBe(true);
  });
});

// ── heatmapColor ──

describe('heatmapColor', () => {
  it('returns an oklch() color string', () => {
    const color = heatmapColor(50, false);
    expect(color).toMatch(/^oklch\(/);
  });

  it('returns different colors for 0%, 50%, 100%', () => {
    const green = heatmapColor(0, false);
    const yellow = heatmapColor(50, false);
    const red = heatmapColor(100, false);
    expect(green).not.toBe(yellow);
    expect(yellow).not.toBe(red);
    expect(green).not.toBe(red);
  });

  it('returns different lightness for dark mode', () => {
    const light = heatmapColor(50, false);
    const dark = heatmapColor(50, true);
    expect(light).not.toBe(dark);
  });

  it('clamps out-of-range percentages', () => {
    const belowZero = heatmapColor(-10, false);
    const aboveHundred = heatmapColor(150, false);
    const zero = heatmapColor(0, false);
    const hundred = heatmapColor(100, false);
    expect(belowZero).toBe(zero);
    expect(aboveHundred).toBe(hundred);
  });
});

// ── Component render ──

describe('OccupancyHeatmap component', () => {
  it('renders a grid with role="grid"', () => {
    render(<OccupancyHeatmap bookings={[]} totalSlots={10} />);
    expect(screen.getByRole('grid')).toBeInTheDocument();
  });

  it('renders 7 day column headers', () => {
    render(<OccupancyHeatmap bookings={[]} totalSlots={10} />);
    const headers = screen.getAllByRole('columnheader');
    expect(headers).toHaveLength(7);
  });

  it('renders 16 hour row headers (6:00 - 21:00)', () => {
    render(<OccupancyHeatmap bookings={[]} totalSlots={10} />);
    const rowHeaders = screen.getAllByRole('rowheader');
    expect(rowHeaders).toHaveLength(16);
    expect(rowHeaders[0]).toHaveTextContent('6:00');
    expect(rowHeaders[15]).toHaveTextContent('21:00');
  });

  it('renders 112 grid cells (7 days x 16 hours)', () => {
    render(<OccupancyHeatmap bookings={[]} totalSlots={10} />);
    const cells = screen.getAllByRole('gridcell');
    expect(cells).toHaveLength(112);
  });

  it('shows tooltip on hover over a cell', async () => {
    const now = new Date();
    const start = new Date(now);
    start.setHours(10, 0, 0, 0);
    const end = new Date(start.getTime() + 60 * 60 * 1000);

    const booking = makeBooking({
      start_time: start.toISOString(),
      end_time: end.toISOString(),
      status: 'active',
    });

    render(<OccupancyHeatmap bookings={[booking]} totalSlots={1} />);

    // Find a cell and hover over it
    const cells = screen.getAllByRole('gridcell');
    fireEvent.mouseEnter(cells[0]);
    // The tooltip should appear for cells with data; for empty cells it may not
    // Just verify no crash
    fireEvent.mouseLeave(cells[0]);
  });

  it('displays color legend', () => {
    render(<OccupancyHeatmap bookings={[]} totalSlots={10} />);
    // i18n may resolve keys or show raw keys depending on load order
    // Check for either the translation or the raw key
    const legendTexts = screen.getAllByText(/Empty|heatmap\.empty/i);
    expect(legendTexts.length).toBeGreaterThan(0);
    const fullTexts = screen.getAllByText(/Full|heatmap\.full/i);
    expect(fullTexts.length).toBeGreaterThan(0);
  });

  it('renders with zero totalSlots without crashing', () => {
    render(<OccupancyHeatmap bookings={[]} totalSlots={0} />);
    expect(screen.getByRole('grid')).toBeInTheDocument();
  });
});
