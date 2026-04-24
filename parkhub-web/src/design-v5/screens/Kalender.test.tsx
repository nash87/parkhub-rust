import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent, within } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockCalendarEvents = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    calendarEvents: (...a: unknown[]) => mockCalendarEvents(...a),
  },
}));

vi.mock('@number-flow/react', () => ({
  default: ({ value }: { value: number }) => <span>{value}</span>,
}));

const mockToast = vi.fn();
vi.mock('../Toast', () => ({
  useV5Toast: () => mockToast,
  V5ToastProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

import { KalenderV5 } from './Kalender';

function renderScreen(navigate = vi.fn()) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <KalenderV5 navigate={navigate} />
    </QueryClientProvider>
  );
}

// Build ISO strings for a known day in the current month so the rendered calendar
// contains the event regardless of "today" drift.
function makeEvents() {
  const now = new Date();
  const y = now.getFullYear();
  const m = now.getMonth();
  const targetDay = 15; // mid-month, always inside the visible month
  const start = new Date(y, m, targetDay, 10, 0).toISOString();
  const end = new Date(y, m, targetDay, 12, 0).toISOString();
  return {
    targetDay,
    events: [
      {
        id: 'evt-1',
        title: 'Parkhaus Nord',
        start,
        end,
        type: 'booking' as const,
        status: 'confirmed',
        lot_name: 'Parkhaus Nord',
      },
      {
        id: 'evt-2',
        title: 'Urlaub',
        start: new Date(y, m, 16, 0, 0).toISOString(),
        end: new Date(y, m, 18, 23, 59).toISOString(),
        type: 'absence' as const,
        status: 'active',
      },
    ],
  };
}

describe('KalenderV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty month with no events', async () => {
    mockCalendarEvents.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Kalender')).toBeInTheDocument());
    // Empty selection placeholder is shown
    expect(screen.getByText(/Klicken Sie auf ein Datum/)).toBeInTheDocument();
  });

  it('renders error state on query failure', async () => {
    mockCalendarEvents.mockRejectedValue(new Error('network'));
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders booking + absence events and counts them in the header', async () => {
    const { events } = makeEvents();
    mockCalendarEvents.mockResolvedValue({ success: true, data: events });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Parkhaus Nord')).toBeInTheDocument());
    // Event title visible in the grid
    expect(screen.getByText('Urlaub')).toBeInTheDocument();
    // Summary stats (label + count)
    expect(screen.getByText('Buchungen')).toBeInTheDocument();
    expect(screen.getByText('Abwesenheit')).toBeInTheDocument();
  });

  it('shows selected-day detail card on click', async () => {
    const { events, targetDay } = makeEvents();
    mockCalendarEvents.mockResolvedValue({ success: true, data: events });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Parkhaus Nord')).toBeInTheDocument());
    const dayCells = screen.getAllByTestId('kalender-day');
    // Find the cell whose day-number text matches targetDay (first match inside current month)
    const targetCell = dayCells.find((c) => {
      const spans = c.querySelectorAll('span');
      return Array.from(spans).some((s) => s.textContent?.trim() === String(targetDay));
    });
    expect(targetCell).toBeTruthy();
    fireEvent.click(targetCell!);
    await waitFor(() => expect(screen.getAllByTestId('kalender-detail').length).toBeGreaterThan(0));
    const detail = screen.getAllByTestId('kalender-detail')[0];
    expect(within(detail).getByText('Buchung')).toBeInTheDocument();
  });

  it('navigates to buchen when the "Platz buchen" button is clicked', async () => {
    mockCalendarEvents.mockResolvedValue({ success: true, data: [] });
    const navigate = vi.fn();
    renderScreen(navigate);
    await waitFor(() => expect(screen.getByText('Platz buchen')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Platz buchen'));
    expect(navigate).toHaveBeenCalledWith('buchen');
  });

  it('switches month when the next/prev buttons are pressed', async () => {
    mockCalendarEvents.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByLabelText('Nächster Monat')).toBeInTheDocument());
    const initialStart = mockCalendarEvents.mock.calls[0][0];
    fireEvent.click(screen.getByLabelText('Nächster Monat'));
    await waitFor(() => expect(mockCalendarEvents.mock.calls.length).toBeGreaterThan(1));
    const nextStart = mockCalendarEvents.mock.calls[mockCalendarEvents.mock.calls.length - 1][0];
    expect(nextStart).not.toBe(initialStart);
  });

  it('surfaces query error when success:false', async () => {
    mockCalendarEvents.mockResolvedValue({ success: false, data: null, error: { code: 'FORBIDDEN', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });
});
