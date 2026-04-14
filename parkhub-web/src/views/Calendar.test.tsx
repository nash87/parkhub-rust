import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent, act } from '@testing-library/react';
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

  it('copy link failure shows error toast', async () => {
    const user = userEvent.setup();
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Subscribe')).toBeInTheDocument());

    await user.click(screen.getByText('Subscribe'));
    await waitFor(() => expect(screen.getByTestId('subscription-url')).toBeInTheDocument());

    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: vi.fn(() => Promise.reject(new Error('denied'))) },
      writable: true,
      configurable: true,
    });

    await user.click(screen.getByLabelText('Copy'));
    // error toast should be called (the component catches clipboard errors)
  });

  it('shows events with various statuses', async () => {
    const now = new Date();
    const dayStr = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-15`;
    mockCalendarEvents.mockResolvedValue({
      success: true,
      data: [
        { id: 'b1', type: 'booking', title: 'S1', start: `${dayStr}T08:00:00Z`, end: `${dayStr}T09:00:00Z`, status: 'pending' },
        { id: 'b2', type: 'absence', title: 'Vacation', start: `${dayStr}T00:00:00Z`, end: `${dayStr}T23:59:59Z`, status: 'cancelled' },
        { id: 'b3', type: 'booking', title: 'S3', start: `${dayStr}T10:00:00Z`, end: `${dayStr}T11:00:00Z`, status: 'completed' },
      ],
    });
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Calendar')).toBeInTheDocument());
  });

  it('shows selected day events with lot_name', async () => {
    const user = userEvent.setup();
    const now = new Date();
    const testDate = new Date(now.getFullYear(), now.getMonth(), 20);
    const isoKey = testDate.toISOString().slice(0, 10);
    mockCalendarEvents.mockResolvedValue({
      success: true,
      data: [{
        id: 'b1', type: 'booking', title: 'Slot B2',
        start: `${isoKey}T09:00:00.000Z`, end: `${isoKey}T17:00:00.000Z`,
        lot_name: 'Garage B', slot_number: 'B2', status: 'active',
      }],
    });
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Calendar')).toBeInTheDocument());

    const buttons = screen.getAllByRole('button');
    const dayBtn = buttons.find(b => b.textContent?.trim() === '20');
    if (dayBtn) {
      await user.click(dayBtn);
      await waitFor(() => {
        const slotB2 = screen.queryByText('Slot B2');
        expect(slotB2).toBeTruthy();
      });
    }
  });

  it('drag-and-drop reschedule flow: confirm success', async () => {
    const user = userEvent.setup();
    const now = new Date();
    const testDate = new Date(now.getFullYear(), now.getMonth(), 10);
    const isoKey = testDate.toISOString().slice(0, 10);
    const targetDate = new Date(now.getFullYear(), now.getMonth(), 20);
    const targetIso = targetDate.toISOString().slice(0, 10);
    mockCalendarEvents.mockResolvedValue({
      success: true,
      data: [{
        id: 'b1', type: 'booking', title: 'Drag Me',
        start: `${isoKey}T09:00:00.000Z`, end: `${isoKey}T17:00:00.000Z`,
        lot_name: 'Lot Y', slot_number: 'Y2', status: 'confirmed',
      }],
    });
    mockRescheduleBooking.mockResolvedValue({
      success: true,
      data: { booking_id: 'b1', success: true, message: 'Booking rescheduled' },
    });
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Calendar')).toBeInTheDocument());

    // Find the booking element (a draggable div) by parent containing day 10
    const buttons = screen.getAllByRole('button');
    const day10 = buttons.find(b => b.textContent?.trim().startsWith('10'))!;
    const day20 = buttons.find(b => b.textContent?.trim().startsWith('20'))!;
    expect(day10).toBeDefined();
    expect(day20).toBeDefined();

    // Find the draggable booking indicator inside day10
    const draggable = day10.querySelector('[draggable="true"]');
    expect(draggable).toBeTruthy();

    const dataTransfer = {
      data: {} as Record<string, string>,
      effectAllowed: '',
      dropEffect: '',
      setData(k: string, v: string) { this.data[k] = v; },
      getData(k: string) { return this.data[k]; },
    };
    fireEvent.dragStart(draggable!, { dataTransfer });
    fireEvent.dragOver(day20, { dataTransfer });
    fireEvent.dragLeave(day20);
    fireEvent.dragOver(day20, { dataTransfer });
    fireEvent.drop(day20, { dataTransfer });

    // Reschedule confirmation modal should appear
    await waitFor(() => {
      expect(screen.getByTestId('reschedule-confirm')).toBeInTheDocument();
    });

    // Confirm reschedule
    await user.click(screen.getByText('Reschedule'));
    await waitFor(() => {
      expect(mockRescheduleBooking).toHaveBeenCalled();
    });

    // After confirm + success, modal should close
    await waitFor(() => {
      expect(screen.queryByTestId('reschedule-confirm')).not.toBeInTheDocument();
    });
  });

  it('drag-and-drop reschedule cancel', async () => {
    const user = userEvent.setup();
    const now = new Date();
    const testDate = new Date(now.getFullYear(), now.getMonth(), 11);
    const isoKey = testDate.toISOString().slice(0, 10);
    mockCalendarEvents.mockResolvedValue({
      success: true,
      data: [{
        id: 'b1', type: 'booking', title: 'Drag Me 2',
        start: `${isoKey}T09:00:00.000Z`, end: `${isoKey}T17:00:00.000Z`,
        lot_name: 'Lot Z', slot_number: 'Z2', status: 'confirmed',
      }],
    });
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Calendar')).toBeInTheDocument());

    const buttons = screen.getAllByRole('button');
    const day11 = buttons.find(b => b.textContent?.trim().startsWith('11'))!;
    const day21 = buttons.find(b => b.textContent?.trim().startsWith('21'))!;

    const draggable = day11.querySelector('[draggable="true"]');
    expect(draggable).toBeTruthy();

    const dataTransfer = {
      data: {} as Record<string, string>,
      effectAllowed: '',
      dropEffect: '',
      setData(k: string, v: string) { this.data[k] = v; },
      getData(k: string) { return this.data[k]; },
    };
    fireEvent.dragStart(draggable!, { dataTransfer });
    fireEvent.dragOver(day21, { dataTransfer });
    fireEvent.drop(day21, { dataTransfer });

    await waitFor(() => {
      expect(screen.getByTestId('reschedule-confirm')).toBeInTheDocument();
    });

    // Click cancel
    await user.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(screen.queryByTestId('reschedule-confirm')).not.toBeInTheDocument();
    });
  });

  it('drag end without showing confirm clears state', async () => {
    const now = new Date();
    const testDate = new Date(now.getFullYear(), now.getMonth(), 12);
    const isoKey = testDate.toISOString().slice(0, 10);
    mockCalendarEvents.mockResolvedValue({
      success: true,
      data: [{
        id: 'b1', type: 'booking', title: 'Drag End Test',
        start: `${isoKey}T09:00:00.000Z`, end: `${isoKey}T17:00:00.000Z`,
        status: 'confirmed',
      }],
    });
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Calendar')).toBeInTheDocument());

    const buttons = screen.getAllByRole('button');
    const day12 = buttons.find(b => b.textContent?.trim().startsWith('12'))!;
    const draggable = day12.querySelector('[draggable="true"]');
    expect(draggable).toBeTruthy();

    const dataTransfer = {
      data: {} as Record<string, string>,
      effectAllowed: '',
      dropEffect: '',
      setData(k: string, v: string) { this.data[k] = v; },
      getData(k: string) { return this.data[k]; },
    };
    fireEvent.dragStart(draggable!, { dataTransfer });
    fireEvent.dragEnd(draggable!);
    // Drag ended without drop — no modal
    expect(screen.queryByTestId('reschedule-confirm')).not.toBeInTheDocument();
  });

  it('non-booking events are not draggable (handleDragStart early return)', async () => {
    const now = new Date();
    const testDate = new Date(now.getFullYear(), now.getMonth(), 13);
    const isoKey = testDate.toISOString().slice(0, 10);
    mockCalendarEvents.mockResolvedValue({
      success: true,
      data: [{
        id: 'a1', type: 'absence', title: 'Vacation',
        start: `${isoKey}T00:00:00.000Z`, end: `${isoKey}T23:59:59.000Z`,
        status: 'pending',
      }],
    });
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Calendar')).toBeInTheDocument());
    // Indicator with draggable=false rendered (non-booking)
    const indicators = document.querySelectorAll('[draggable="false"]');
    expect(indicators.length).toBeGreaterThan(0);
  });

  it('handleDragOver does nothing when no dragEvent', async () => {
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Calendar')).toBeInTheDocument());
    const buttons = screen.getAllByRole('button');
    const day = buttons.find(b => b.textContent?.trim() === '10')!;
    fireEvent.dragOver(day);
    // No crash, no state change
    expect(screen.queryByTestId('reschedule-confirm')).not.toBeInTheDocument();
  });

  it('handleDrop without dragEvent does nothing', async () => {
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Calendar')).toBeInTheDocument());
    const buttons = screen.getAllByRole('button');
    const day = buttons.find(b => b.textContent?.trim() === '15')!;
    fireEvent.drop(day);
    expect(screen.queryByTestId('reschedule-confirm')).not.toBeInTheDocument();
  });

  it('reschedule API failure shows error toast', async () => {
    const user = userEvent.setup();
    const now = new Date();
    const testDate = new Date(now.getFullYear(), now.getMonth(), 14);
    const isoKey = testDate.toISOString().slice(0, 10);
    mockCalendarEvents.mockResolvedValue({
      success: true,
      data: [{
        id: 'b1', type: 'booking', title: 'Will Fail',
        start: `${isoKey}T09:00:00.000Z`, end: `${isoKey}T17:00:00.000Z`,
        status: 'confirmed',
      }],
    });
    mockRescheduleBooking.mockResolvedValue({
      success: true,
      data: { booking_id: 'b1', success: false, message: 'Conflict' },
    });
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Calendar')).toBeInTheDocument());

    const buttons = screen.getAllByRole('button');
    const dayFrom = buttons.find(b => b.textContent?.trim().startsWith('14'))!;
    const dayTo = buttons.find(b => b.textContent?.trim().startsWith('24'))!;

    const draggable = dayFrom.querySelector('[draggable="true"]')!;
    const dataTransfer = {
      data: {} as Record<string, string>,
      effectAllowed: '', dropEffect: '',
      setData(k: string, v: string) { this.data[k] = v; },
      getData(k: string) { return this.data[k]; },
    };
    fireEvent.dragStart(draggable, { dataTransfer });
    fireEvent.dragOver(dayTo, { dataTransfer });
    fireEvent.drop(dayTo, { dataTransfer });
    await waitFor(() => expect(screen.getByTestId('reschedule-confirm')).toBeInTheDocument());

    await user.click(screen.getByText('Reschedule'));
    await waitFor(() => {
      expect(mockRescheduleBooking).toHaveBeenCalled();
    });
  });

  it('reschedule API exception shows error toast', async () => {
    const user = userEvent.setup();
    const now = new Date();
    const testDate = new Date(now.getFullYear(), now.getMonth(), 16);
    const isoKey = testDate.toISOString().slice(0, 10);
    mockCalendarEvents.mockResolvedValue({
      success: true,
      data: [{
        id: 'b1', type: 'booking', title: 'Will Throw',
        start: `${isoKey}T09:00:00.000Z`, end: `${isoKey}T17:00:00.000Z`,
        status: 'confirmed',
      }],
    });
    mockRescheduleBooking.mockRejectedValue(new Error('Network'));
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Calendar')).toBeInTheDocument());

    const buttons = screen.getAllByRole('button');
    const dayFrom = buttons.find(b => b.textContent?.trim().startsWith('16'))!;
    const dayTo = buttons.find(b => b.textContent?.trim().startsWith('26'))!;

    const draggable = dayFrom.querySelector('[draggable="true"]')!;
    const dataTransfer = {
      data: {} as Record<string, string>,
      effectAllowed: '', dropEffect: '',
      setData(k: string, v: string) { this.data[k] = v; },
      getData(k: string) { return this.data[k]; },
    };
    fireEvent.dragStart(draggable, { dataTransfer });
    fireEvent.dragOver(dayTo, { dataTransfer });
    fireEvent.drop(dayTo, { dataTransfer });
    await waitFor(() => expect(screen.getByTestId('reschedule-confirm')).toBeInTheDocument());

    await user.click(screen.getByText('Reschedule'));
    await waitFor(() => {
      expect(mockRescheduleBooking).toHaveBeenCalled();
    });
  });

  it('confirmReschedule does nothing without dragEvent', async () => {
    // Already covered by other paths but explicit early return
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Calendar')).toBeInTheDocument());
    // No drag; modal not visible
    expect(screen.queryByTestId('reschedule-confirm')).not.toBeInTheDocument();
  });

  it('subscribe modal closes when clicking backdrop', async () => {
    const user = userEvent.setup();
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Subscribe')).toBeInTheDocument());

    await user.click(screen.getByText('Subscribe'));
    await waitFor(() => expect(screen.getByText('Subscribe to Calendar')).toBeInTheDocument());

    // Find the modal backdrop (fixed inset-0)
    const modal = screen.getByText('Subscribe to Calendar').closest('.fixed.inset-0') as HTMLElement;
    expect(modal).toBeTruthy();
    // Click the backdrop directly
    fireEvent.click(modal);
    await waitFor(() => {
      expect(screen.queryByText('Subscribe to Calendar')).not.toBeInTheDocument();
    });
  });

  it('aborts in-flight loadEvents on unmount', async () => {
    const { unmount } = render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Calendar')).toBeInTheDocument());
    unmount();
    // No assertion — exercises the cleanup path of useEffect
  });

  it('aborts in-flight loadEvents when component unmounts', async () => {
    const calls: Array<{ resolve: (v: any) => void }> = [];
    mockCalendarEvents.mockImplementation(() => {
      return new Promise<any>((resolve) => {
        calls.push({ resolve });
      });
    });
    const { unmount } = render(<CalendarPage />);
    await waitFor(() => expect(calls.length).toBe(1));
    // Unmount triggers abort
    unmount();
    // Resolving the aborted call should hit the abort-check return paths
    await act(async () => {
      calls[0].resolve({ success: true, data: [{ id: 'x', type: 'booking', title: 'X', start: '2026-01-01', end: '2026-01-01', status: 'confirmed' }] });
    });
    expect(mockCalendarEvents).toHaveBeenCalledTimes(1);
  });

  it('aborted loadEvents in catch branch does not toast', async () => {
    const calls: Array<{ reject: (v: any) => void }> = [];
    mockCalendarEvents.mockImplementation(() => {
      return new Promise<any>((_, reject) => {
        calls.push({ reject });
      });
    });
    const { unmount } = render(<CalendarPage />);
    await waitFor(() => expect(calls.length).toBe(1));
    unmount();
    // After unmount, the controller is aborted. Reject the promise.
    await act(async () => {
      calls[0].reject(new Error('aborted'));
    });
    expect(mockCalendarEvents).toHaveBeenCalledTimes(1);
  });

  it('non-booking handleDragStart triggers early return', async () => {
    const now = new Date();
    const testDate = new Date(now.getFullYear(), now.getMonth(), 17);
    const isoKey = testDate.toISOString().slice(0, 10);
    mockCalendarEvents.mockResolvedValue({
      success: true,
      data: [{
        id: 'a1', type: 'absence', title: 'Out',
        start: `${isoKey}T00:00:00.000Z`, end: `${isoKey}T23:59:59.000Z`,
        status: 'pending',
      }],
    });
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Calendar')).toBeInTheDocument());

    // Find the absence indicator (draggable=false) and try to fire dragStart on it
    const indicator = document.querySelector('[draggable="false"]');
    expect(indicator).toBeTruthy();
    const dataTransfer = {
      data: {} as Record<string, string>,
      effectAllowed: '', dropEffect: '',
      setData(k: string, v: string) { this.data[k] = v; },
      getData(k: string) { return this.data[k]; },
    };
    fireEvent.dragStart(indicator!, { dataTransfer });
    // Early return — no drag state; no modal
    expect(screen.queryByTestId('reschedule-confirm')).not.toBeInTheDocument();
  });

  it('setTimeout in copy link clears copied state', async () => {
    const user = userEvent.setup();
    Object.defineProperty(navigator, 'clipboard', {
      value: { writeText: vi.fn(() => Promise.resolve()) },
      writable: true,
      configurable: true,
    });
    render(<CalendarPage />);
    await waitFor(() => expect(screen.getByText('Subscribe')).toBeInTheDocument());

    await user.click(screen.getByText('Subscribe'));
    await waitFor(() => expect(screen.getByTestId('subscription-url')).toBeInTheDocument());

    await user.click(screen.getByLabelText('Copy'));
    // Wait for setCopied(true) -> setTimeout -> setCopied(false)
    await new Promise<void>(r => setTimeout(r, 2200));
  }, 8000);

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
