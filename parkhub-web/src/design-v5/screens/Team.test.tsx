import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockTeamAbsences = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    teamAbsences: (...a: unknown[]) => mockTeamAbsences(...a),
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

import { TeamV5 } from './Team';

function renderScreen(navigate = vi.fn()) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <TeamV5 navigate={navigate} />
    </QueryClientProvider>
  );
}

const TODAY = new Date().toISOString().slice(0, 10);
const FAR_FUTURE = '2099-01-01';
const PAST = '2000-01-01';

const ABSENCE_TODAY = { user_name: 'Alice', absence_type: 'vacation', start_date: TODAY, end_date: TODAY };
const ABSENCE_HOMEOFFICE_TODAY = { user_name: 'Bob', absence_type: 'homeoffice', start_date: TODAY, end_date: TODAY };
const ABSENCE_UPCOMING = { user_name: 'Bob', absence_type: 'homeoffice', start_date: FAR_FUTURE, end_date: FAR_FUTURE };
const ABSENCE_PAST = { user_name: 'Carol', absence_type: 'sick', start_date: PAST, end_date: PAST };

describe('TeamV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state when no members', async () => {
    mockTeamAbsences.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Noch keine Teamdaten')).toBeInTheDocument());
  });

  it('renders error state when query fails', async () => {
    mockTeamAbsences.mockRejectedValue(new Error('network'));
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('surfaces query error when success:false', async () => {
    mockTeamAbsences.mockResolvedValue({ success: false, data: null, error: { code: 'FORBIDDEN', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders team rows with name and status badge', async () => {
    mockTeamAbsences.mockResolvedValue({ success: true, data: [ABSENCE_TODAY, ABSENCE_HOMEOFFICE_TODAY] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('team-row')).toHaveLength(2));
    // Alice appears in roster row + today-absences panel, so allow multiple matches
    expect(screen.getAllByText('Alice').length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText('Bob').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('Abwesend')).toBeInTheDocument();
    expect(screen.getByText('Home Office')).toBeInTheDocument();
  });

  it('shows today absence panel when someone is out today', async () => {
    mockTeamAbsences.mockResolvedValue({ success: true, data: [ABSENCE_TODAY] });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('today-absences')).toBeInTheDocument());
    // "Heute abwesend" appears in the stat tile + panel heading, both are fine
    expect(screen.getAllByText(/Heute abwesend/).length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('Urlaub')).toBeInTheDocument();
  });

  it('derives "Gebucht" status for members without today/past absences active', async () => {
    mockTeamAbsences.mockResolvedValue({ success: true, data: [ABSENCE_PAST] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Carol')).toBeInTheDocument());
    expect(screen.getByText('Gebucht')).toBeInTheDocument();
  });

  it('renders correctly with mixed absence types', async () => {
    mockTeamAbsences.mockResolvedValue({
      success: true,
      data: [ABSENCE_TODAY, ABSENCE_UPCOMING, ABSENCE_PAST],
    });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('team-row')).toHaveLength(3));
    // members sorted alphabetically: Alice, Bob, Carol
    const rows = screen.getAllByTestId('team-row');
    expect(rows[0]).toHaveTextContent('Alice');
    expect(rows[1]).toHaveTextContent('Bob');
    expect(rows[2]).toHaveTextContent('Carol');
  });
});
