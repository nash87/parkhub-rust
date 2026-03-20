import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockCalendarEvents = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    calendarEvents: (...args: any[]) => mockCalendarEvents(...args),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'calendar.title': 'Calendar',
        'calendar.noBookings': 'No entries on this day',
        'calendar.selectDay': 'Click a day to see entries',
      };
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
  CaretLeft: (props: any) => <span data-testid="icon-caret-left" {...props} />,
  CaretRight: (props: any) => <span data-testid="icon-caret-right" {...props} />,
  CalendarBlank: (props: any) => <span data-testid="icon-calendar-blank" {...props} />,
}));

import { CalendarPage } from './Calendar';

describe('CalendarPage', () => {
  beforeEach(() => {
    mockCalendarEvents.mockClear();
    mockCalendarEvents.mockResolvedValue({ success: true, data: [] });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the calendar title after loading', async () => {
    render(<CalendarPage />);

    await waitFor(() => {
      expect(screen.getByText('Calendar')).toBeInTheDocument();
    });
  });

  it('renders month navigation buttons', async () => {
    render(<CalendarPage />);

    await waitFor(() => {
      expect(screen.getByTestId('icon-caret-left')).toBeInTheDocument();
      expect(screen.getByTestId('icon-caret-right')).toBeInTheDocument();
    });
  });

  it('renders the current month label', async () => {
    render(<CalendarPage />);

    await waitFor(() => {
      const now = new Date();
      const monthLabel = now.toLocaleDateString(undefined, { month: 'long', year: 'numeric' });
      expect(screen.getByText(monthLabel)).toBeInTheDocument();
    });
  });

  it('renders a 7-column day grid', async () => {
    render(<CalendarPage />);

    await waitFor(() => {
      // Should have day buttons (at least 28 for shortest month)
      const buttons = screen.getAllByRole('button');
      // Filter out navigation buttons — day buttons contain single numbers
      const dayButtons = buttons.filter(b => /^\d{1,2}$/.test(b.textContent?.trim() || ''));
      expect(dayButtons.length).toBeGreaterThanOrEqual(28);
    });
  });

  it('shows "Click a day to see entries" initially', async () => {
    render(<CalendarPage />);

    await waitFor(() => {
      expect(screen.getByText('Click a day to see entries')).toBeInTheDocument();
    });
  });

  it('navigates to previous month on left arrow click', async () => {
    const user = userEvent.setup();
    render(<CalendarPage />);

    await waitFor(() => {
      expect(screen.getByText('Calendar')).toBeInTheDocument();
    });

    const now = new Date();
    const prevMonth = new Date(now.getFullYear(), now.getMonth() - 1, 1);
    const prevLabel = prevMonth.toLocaleDateString(undefined, { month: 'long', year: 'numeric' });

    await user.click(screen.getByTestId('icon-caret-left').closest('button')!);

    await waitFor(() => {
      expect(screen.getByText(prevLabel)).toBeInTheDocument();
    });
  });

  it('navigates to next month on right arrow click', async () => {
    const user = userEvent.setup();
    render(<CalendarPage />);

    await waitFor(() => {
      expect(screen.getByText('Calendar')).toBeInTheDocument();
    });

    const now = new Date();
    const nextMonth = new Date(now.getFullYear(), now.getMonth() + 1, 1);
    const nextLabel = nextMonth.toLocaleDateString(undefined, { month: 'long', year: 'numeric' });

    await user.click(screen.getByTestId('icon-caret-right').closest('button')!);

    await waitFor(() => {
      expect(screen.getByText(nextLabel)).toBeInTheDocument();
    });
  });

  it('calls calendarEvents API on mount', async () => {
    render(<CalendarPage />);

    await waitFor(() => {
      expect(mockCalendarEvents).toHaveBeenCalled();
    });
  });

  it('shows "No entries on this day" when clicking a day with no events', async () => {
    const user = userEvent.setup();
    render(<CalendarPage />);

    await waitFor(() => {
      expect(screen.getByText('Calendar')).toBeInTheDocument();
    });

    // Click on day 15 (should exist in any month)
    const buttons = screen.getAllByRole('button');
    const day15 = buttons.find(b => b.textContent?.trim() === '15');
    expect(day15).toBeDefined();
    await user.click(day15!);

    await waitFor(() => {
      expect(screen.getByText('No entries on this day')).toBeInTheDocument();
    });
  });

  it('refetches events when month changes', async () => {
    const user = userEvent.setup();
    render(<CalendarPage />);

    await waitFor(() => {
      expect(mockCalendarEvents).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByTestId('icon-caret-right').closest('button')!);

    await waitFor(() => {
      expect(mockCalendarEvents).toHaveBeenCalledTimes(2);
    });
  });
});
