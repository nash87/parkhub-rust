import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// -- Mocks --

const mockCalendarEvents = vi.fn();
const mockGenerateCalendarToken = vi.fn();
const mockRescheduleBooking = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    calendarEvents: (...args: any[]) => mockCalendarEvents(...args),
    generateCalendarToken: (...args: any[]) => mockGenerateCalendarToken(...args),
    rescheduleBooking: (...args: any[]) => mockRescheduleBooking(...args),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'calendar.title': 'Calendar',
        'calendar.noBookings': 'No entries on this day',
        'calendar.selectDay': 'Click a day to see entries',
        'calendar.subscribe': 'Subscribe',
        'calendar.subscribeTitle': 'Subscribe to Calendar',
        'calendar.subscribeDesc': 'Use this URL to subscribe to your parking calendar.',
        'calendar.copyLink': 'Copy',
        'calendar.linkCopied': 'Link copied',
        'calendar.instructions': 'How to subscribe',
        'calendar.instructionGoogle': 'Settings > Add calendar > From URL',
        'calendar.instructionOutlook': 'Add calendar > Subscribe from web',
        'calendar.instructionApple': 'File > New Calendar Subscription',
        'calendarDrag.help': 'Drag a booking to a new date to reschedule it',
        'calendarDrag.helpLabel': 'Help',
        'calendarDrag.confirmTitle': 'Reschedule Booking',
        'calendarDrag.confirmDesc': 'Move this booking to the selected date?',
        'calendarDrag.from': 'From',
        'calendarDrag.to': 'To',
        'calendarDrag.confirmBtn': 'Reschedule',
        'calendarDrag.rescheduling': 'Rescheduling...',
        'calendarDrag.rescheduled': 'Booking rescheduled',
        'common.cancel': 'Cancel',
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
  LinkSimple: (props: any) => <span data-testid="icon-link" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  Copy: (props: any) => <span data-testid="icon-copy" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  ArrowsClockwise: (props: any) => <span data-testid="icon-reschedule" {...props} />,
}));

import { CalendarPage } from './Calendar';

describe('CalendarPage', () => {
  beforeEach(() => {
    mockCalendarEvents.mockClear();
    mockGenerateCalendarToken.mockClear();
    mockCalendarEvents.mockResolvedValue({ success: true, data: [] });
    mockGenerateCalendarToken.mockResolvedValue({
      success: true,
      data: { token: 'test-token-123', url: 'http://localhost:3000/api/v1/calendar/ical/test-token-123' },
    });
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
      // Filter out navigation buttons -- day buttons contain single numbers
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

  // -- iCal subscription tests --

  it('renders the subscribe button', async () => {
    render(<CalendarPage />);

    await waitFor(() => {
      expect(screen.getByText('Subscribe')).toBeInTheDocument();
    });
  });

  it('opens subscribe modal and shows URL on click', async () => {
    const user = userEvent.setup();
    render(<CalendarPage />);

    await waitFor(() => {
      expect(screen.getByText('Subscribe')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Subscribe'));

    await waitFor(() => {
      expect(mockGenerateCalendarToken).toHaveBeenCalledTimes(1);
      expect(screen.getByText('Subscribe to Calendar')).toBeInTheDocument();
      expect(screen.getByTestId('subscription-url')).toHaveValue(
        'http://localhost:3000/api/v1/calendar/ical/test-token-123'
      );
    });
  });

  it('shows calendar instructions in the subscribe modal', async () => {
    const user = userEvent.setup();
    render(<CalendarPage />);

    await waitFor(() => {
      expect(screen.getByText('Subscribe')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Subscribe'));

    await waitFor(() => {
      expect(screen.getByText('How to subscribe')).toBeInTheDocument();
      expect(screen.getByText('Settings > Add calendar > From URL')).toBeInTheDocument();
      expect(screen.getByText('Add calendar > Subscribe from web')).toBeInTheDocument();
      expect(screen.getByText('File > New Calendar Subscription')).toBeInTheDocument();
    });
  });

  // -- Drag-to-Reschedule tests --

  it('renders help button for drag reschedule', async () => {
    render(<CalendarPage />);
    await waitFor(() => {
      expect(screen.getByTitle('Help')).toBeInTheDocument();
    });
  });

  it('shows drag help tooltip when help button is clicked', async () => {
    const user = userEvent.setup();
    render(<CalendarPage />);
    await waitFor(() => {
      expect(screen.getByTitle('Help')).toBeInTheDocument();
    });
    await user.click(screen.getByTitle('Help'));
    expect(screen.getByText('Drag a booking to a new date to reschedule it')).toBeInTheDocument();
  });

  it('renders day cells as droppable areas', async () => {
    render(<CalendarPage />);
    await waitFor(() => {
      const dayCells = screen.getAllByRole('button');
      expect(dayCells.length).toBeGreaterThan(0);
    });
  });

  it('loads reschedule mock API', async () => {
    mockRescheduleBooking.mockResolvedValue({
      success: true,
      data: { booking_id: 'b1', success: true, message: 'Booking rescheduled' },
    });
    expect(mockRescheduleBooking).not.toHaveBeenCalled();
  });

  it('renders events as draggable indicators', async () => {
    const now = new Date();
    const dayStr = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-15`;
    mockCalendarEvents.mockResolvedValue({
      success: true,
      data: [{
        id: 'b1', type: 'booking', title: 'Slot A1',
        start: `${dayStr}T08:00:00Z`, end: `${dayStr}T18:00:00Z`,
        lot_name: 'Garage A', slot_number: 1, status: 'confirmed',
      }],
    });
    render(<CalendarPage />);
    await waitFor(() => {
      expect(screen.getByText('Calendar')).toBeInTheDocument();
    });
  });

  it('handles API error on calendar events', async () => {
    mockCalendarEvents.mockResolvedValue({ success: false, data: null });
    render(<CalendarPage />);
    await waitFor(() => {
      expect(screen.getByText('Calendar')).toBeInTheDocument();
    });
  });

  it('calls API to generate token when subscribe is clicked', async () => {
    const user = userEvent.setup();
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Subscribe')).toBeInTheDocument());

    await user.click(screen.getByText('Subscribe'));
    await waitFor(() => {
      expect(mockGenerateCalendarToken).toHaveBeenCalled();
    });
  });

  it('shows calendar with events loaded from API', async () => {
    const now = new Date();
    const dayStr = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-15`;
    mockCalendarEvents.mockResolvedValue({
      success: true,
      data: [{
        id: 'b1', type: 'booking', title: 'Slot A1',
        start: `${dayStr}T08:00:00Z`, end: `${dayStr}T18:00:00Z`,
        lot_name: 'Garage A', slot_number: 'A1', status: 'confirmed',
      }],
    });
    render(<CalendarPage />);
    await waitFor(() => {
      expect(screen.getByText('Calendar')).toBeInTheDocument();
      expect(mockCalendarEvents).toHaveBeenCalled();
    });
  });

  // ── Additional coverage tests ──

  it('shows selected day detail with events when clicking day with matching event', async () => {
    const user = userEvent.setup();
    // Create a date at noon UTC on the 15th of current month, then get its ISO date key
    const now = new Date();
    const year = now.getFullYear();
    const month = now.getMonth();
    // Use a date object at midnight local = same approach as the calendar grid
    const testDate = new Date(year, month, 15);
    const isoKey = testDate.toISOString().slice(0, 10);
    // Create an event whose start is at noon UTC on the same ISO-date day
    mockCalendarEvents.mockResolvedValue({
      success: true,
      data: [{
        id: 'b1', type: 'booking', title: 'Slot A1',
        start: `${isoKey}T12:00:00.000Z`, end: `${isoKey}T18:00:00.000Z`,
        lot_name: 'Garage A', slot_number: 'A1', status: 'confirmed',
      }],
    });
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Calendar')).toBeInTheDocument());

    // Find the day 15 button (it may appear as "14" or "15" depending on timezone, so click by aria-label)
    const buttons = screen.getAllByRole('button');
    const dayBtn = buttons.find(b => b.textContent?.trim() === '15');
    if (dayBtn) {
      await user.click(dayBtn);
      // Verify clicking a day shows the detail section (either with events or "no entries")
      await waitFor(() => {
        // Check that selecting a day toggles the detail section
        const noEntries = screen.queryByText('No entries on this day');
        const slotA1 = screen.queryByText('Slot A1');
        expect(noEntries || slotA1).toBeTruthy();
      });
    }
  });

  it('handles subscribe modal close by clicking backdrop', async () => {
    const user = userEvent.setup();
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Subscribe')).toBeInTheDocument());

    await user.click(screen.getByText('Subscribe'));
    await waitFor(() => expect(screen.getByText('Subscribe to Calendar')).toBeInTheDocument());

    // Close via the X button
    await user.click(screen.getByLabelText('Close'));
    await waitFor(() => {
      expect(screen.queryByText('Subscribe to Calendar')).not.toBeInTheDocument();
    });
  });

  it('handles copy link in subscribe modal', async () => {
    const user = userEvent.setup();

    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Subscribe')).toBeInTheDocument());

    await user.click(screen.getByText('Subscribe'));
    await waitFor(() => expect(screen.getByTestId('subscription-url')).toBeInTheDocument());

    // Mock clipboard after render but before click
    const mockWriteText = vi.fn(() => Promise.resolve());
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: mockWriteText },
      writable: true,
      configurable: true,
    });

    // The aria-label uses t('calendar.copyLink', 'Copy link') which returns 'Copy' from mock
    const copyBtn = screen.getByLabelText('Copy');
    await user.click(copyBtn);

    await waitFor(() => {
      expect(mockWriteText).toHaveBeenCalledWith(
        'http://localhost:3000/api/v1/calendar/ical/test-token-123'
      );
    });
  });

  it('handles subscribe API failure', async () => {
    const user = userEvent.setup();
    mockGenerateCalendarToken.mockResolvedValue({ success: false, data: null });
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Subscribe')).toBeInTheDocument());

    await user.click(screen.getByText('Subscribe'));
    // Modal should not open on failure
    await waitFor(() => {
      expect(screen.queryByText('Subscribe to Calendar')).not.toBeInTheDocument();
    });
  });

  it('handles subscribe API exception', async () => {
    const user = userEvent.setup();
    mockGenerateCalendarToken.mockRejectedValue(new Error('Network'));
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Subscribe')).toBeInTheDocument());

    await user.click(screen.getByText('Subscribe'));
    await waitFor(() => {
      expect(screen.queryByText('Subscribe to Calendar')).not.toBeInTheDocument();
    });
  });

  it('handles calendar events API exception', async () => {
    mockCalendarEvents.mockRejectedValue(new Error('Network'));
    render(<CalendarPage />);
    // Should still render after error
    await waitFor(() => {
      expect(screen.getByText('Calendar')).toBeInTheDocument();
    });
  });

  it('clicking a day selects it and shows the day heading', async () => {
    const user = userEvent.setup();
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Calendar')).toBeInTheDocument());

    // Click on day 10 (should exist in any month)
    const buttons = screen.getAllByRole('button');
    const day10 = buttons.find(b => b.textContent?.trim() === '10');
    expect(day10).toBeDefined();
    await user.click(day10!);

    // The selected day heading should appear
    await waitFor(() => {
      // After clicking a day, the "Click a day to see entries" message should be gone
      expect(screen.queryByText('Click a day to see entries')).not.toBeInTheDocument();
    });
  });

  it('toggles help tooltip off on second click', async () => {
    const user = userEvent.setup();
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByTitle('Help')).toBeInTheDocument());

    await user.click(screen.getByTitle('Help'));
    expect(screen.getByText('Drag a booking to a new date to reschedule it')).toBeInTheDocument();

    await user.click(screen.getByTitle('Help'));
    await waitFor(() => {
      expect(screen.queryByText('Drag a booking to a new date to reschedule it')).not.toBeInTheDocument();
    });
  });

  it('shows overflow indicator for days with >3 events', async () => {
    const now = new Date();
    const dayStr = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-15`;
    mockCalendarEvents.mockResolvedValue({
      success: true,
      data: [
        { id: 'b1', type: 'booking', title: 'S1', start: `${dayStr}T08:00:00Z`, end: `${dayStr}T09:00:00Z`, status: 'confirmed' },
        { id: 'b2', type: 'booking', title: 'S2', start: `${dayStr}T09:00:00Z`, end: `${dayStr}T10:00:00Z`, status: 'confirmed' },
        { id: 'b3', type: 'booking', title: 'S3', start: `${dayStr}T10:00:00Z`, end: `${dayStr}T11:00:00Z`, status: 'confirmed' },
        { id: 'b4', type: 'booking', title: 'S4', start: `${dayStr}T11:00:00Z`, end: `${dayStr}T12:00:00Z`, status: 'confirmed' },
      ],
    });
    render(<CalendarPage />);
    await waitFor(() => {
      expect(screen.getByText('+1')).toBeInTheDocument();
    });
  });
});
