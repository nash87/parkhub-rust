import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockGetTeam = vi.fn();
const mockGetAdminStats = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    getTeam: (...a: unknown[]) => mockGetTeam(...a),
    getAdminStatsExtended: (...a: unknown[]) => mockGetAdminStats(...a),
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

import { RanglisteV5 } from './Rangliste';

function renderScreen(navigate = vi.fn()) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <RanglisteV5 navigate={navigate} />
    </QueryClientProvider>
  );
}

const TEAM_ALICE = { id: 'u1', username: 'alice', name: 'Alice', role: 'user' };
const TEAM_BOB = { id: 'u2', username: 'bob', name: 'Bob', role: 'user' };

const STATS_BASE = {
  total_users: 2,
  total_lots: 1,
  total_bookings: 20,
  active_bookings: 5,
};

const STATS_WITH_USERS = {
  ...STATS_BASE,
  bookings_by_user: {
    u1: { total: 20, this_month: 12, ev_count: 10, morning_count: 8, swaps_accepted: 3, no_shows: 0, avg_duration_hours: 4 },
    u2: { total: 10, this_month: 3, ev_count: 0, morning_count: 0, swaps_accepted: 0, no_shows: 2, avg_duration_hours: 2 },
  },
};

describe('RanglisteV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state when no members', async () => {
    mockGetTeam.mockResolvedValue({ success: true, data: [] });
    mockGetAdminStats.mockResolvedValue({ success: true, data: STATS_BASE });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine Rangliste verfügbar')).toBeInTheDocument());
  });

  it('renders error when team query fails', async () => {
    mockGetTeam.mockRejectedValue(new Error('network'));
    mockGetAdminStats.mockResolvedValue({ success: true, data: STATS_BASE });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('surfaces error when team success:false', async () => {
    mockGetTeam.mockResolvedValue({ success: false, data: null, error: { code: 'FORBIDDEN', message: 'denied' } });
    mockGetAdminStats.mockResolvedValue({ success: true, data: STATS_BASE });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('surfaces error when stats success:false', async () => {
    mockGetTeam.mockResolvedValue({ success: true, data: [TEAM_ALICE] });
    mockGetAdminStats.mockResolvedValue({ success: false, data: null, error: { code: 'FORBIDDEN', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders leaderboard rows sorted by eco score', async () => {
    mockGetTeam.mockResolvedValue({ success: true, data: [TEAM_BOB, TEAM_ALICE] });
    mockGetAdminStats.mockResolvedValue({ success: true, data: STATS_WITH_USERS });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('rank-row')).toHaveLength(2));
    const rows = screen.getAllByTestId('rank-row');
    // Alice has higher eco score due to EV + bookings
    expect(rows[0]).toHaveTextContent('Alice');
    expect(rows[1]).toHaveTextContent('Bob');
  });

  it('renders badges for EV / early / team / frequent stats', async () => {
    mockGetTeam.mockResolvedValue({ success: true, data: [TEAM_ALICE] });
    mockGetAdminStats.mockResolvedValue({ success: true, data: STATS_WITH_USERS });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('rank-row')).toBeInTheDocument());
    expect(screen.getByText('EV')).toBeInTheDocument();
    expect(screen.getByText('Früh')).toBeInTheDocument();
    expect(screen.getByText('Teamplayer')).toBeInTheDocument();
    expect(screen.getByText('Vielparker')).toBeInTheDocument();
  });

  it('shows highlight cards for aktivster / grünster / zuverlässigster', async () => {
    mockGetTeam.mockResolvedValue({ success: true, data: [TEAM_ALICE, TEAM_BOB] });
    mockGetAdminStats.mockResolvedValue({ success: true, data: STATS_WITH_USERS });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('highlights')).toBeInTheDocument());
    expect(screen.getByText('Aktivster')).toBeInTheDocument();
    expect(screen.getByText('Grünster')).toBeInTheDocument();
    expect(screen.getByText('Zuverlässigster')).toBeInTheDocument();
  });
});
