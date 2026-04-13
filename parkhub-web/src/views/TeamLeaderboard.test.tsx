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
        'leaderboard.title': 'Team Leaderboard',
        'leaderboard.subtitle': 'See how your team is doing',
        'leaderboard.empty': 'No team data available',
        'leaderboard.mostActive': 'Most Active',
        'leaderboard.greenest': 'Greenest (EV)',
        'leaderboard.mostReliable': 'Most Reliable',
        'leaderboard.bookings': 'bookings',
        'leaderboard.noShows': 'no-shows',
        'leaderboard.ecoScore': 'Eco Score',
        'leaderboard.badgeEv': 'EV Driver',
        'leaderboard.badgeEarly': 'Early Bird',
        'leaderboard.badgeTeam': 'Team Player',
        'leaderboard.badgeFrequent': 'Frequent Parker',
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
  Trophy: (props: any) => <span data-testid="icon-trophy" {...props} />,
  Medal: (props: any) => <span data-testid="icon-medal" {...props} />,
  Lightning: (props: any) => <span data-testid="icon-lightning" {...props} />,
  UserCircle: (props: any) => <span data-testid="icon-user" {...props} />,
  Star: (props: any) => <span data-testid="icon-star" {...props} />,
  Sun: (props: any) => <span data-testid="icon-sun" {...props} />,
  UsersThree: (props: any) => <span data-testid="icon-users" {...props} />,
  Car: (props: any) => <span data-testid="icon-car" {...props} />,
}));

vi.mock('../constants/animations', () => ({
  staggerSlow: { hidden: { opacity: 0 }, show: { opacity: 1 } },
  fadeUp: { hidden: { opacity: 0 }, show: { opacity: 1 } },
}));

import { TeamLeaderboardPage } from './TeamLeaderboard';

const mockTeam = [
  { id: 'u1', username: 'alice', name: 'Alice Mueller', role: 'user' },
  { id: 'u2', username: 'bob', name: 'Bob Schmidt', role: 'user' },
  { id: 'u3', username: 'carol', name: 'Carol Braun', role: 'user' },
  { id: 'u4', username: 'dave', name: 'Dave Fischer', role: 'user' },
];

const mockStats = {
  total_bookings: 100,
  active_bookings: 5,
  total_users: 4,
  bookings_by_user: {
    u1: { total: 30, this_month: 12, ev_count: 10, morning_count: 8, swaps_accepted: 3, no_shows: 0, avg_duration_hours: 4 },
    u2: { total: 20, this_month: 8, ev_count: 0, morning_count: 2, swaps_accepted: 0, no_shows: 2, avg_duration_hours: 3 },
    u3: { total: 15, this_month: 5, ev_count: 15, morning_count: 0, swaps_accepted: 1, no_shows: 0, avg_duration_hours: 6 },
    u4: { total: 5, this_month: 2, ev_count: 1, morning_count: 0, swaps_accepted: 0, no_shows: 1, avg_duration_hours: 2 },
  },
};

describe('TeamLeaderboardPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading skeleton initially', () => {
    global.fetch = vi.fn().mockReturnValue(new Promise(() => {}));
    render(<TeamLeaderboardPage />);
    expect(screen.getByTestId('loading')).toBeInTheDocument();
  });

  it('renders page with title', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockTeam }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockStats }) });

    render(<TeamLeaderboardPage />);
    await waitFor(() => {
      expect(screen.getByText('Team Leaderboard')).toBeInTheDocument();
    });
    expect(screen.getByTestId('leaderboard-page')).toBeInTheDocument();
  });

  it('shows empty state when no team members', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: [] }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: {} }) });

    render(<TeamLeaderboardPage />);
    await waitFor(() => {
      expect(screen.getByTestId('empty-state')).toBeInTheDocument();
    });
    expect(screen.getByText('No team data available')).toBeInTheDocument();
  });

  it('renders leaderboard rows for each team member', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockTeam }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockStats }) });

    render(<TeamLeaderboardPage />);
    await waitFor(() => {
      const rows = screen.getAllByTestId('leaderboard-row');
      expect(rows).toHaveLength(4);
    });
  });

  it('displays badges for qualifying members', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockTeam }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockStats }) });

    render(<TeamLeaderboardPage />);
    await waitFor(() => {
      // Alice has EV, Early Bird, Team Player, Frequent Parker
      expect(screen.getAllByTestId('badge-ev').length).toBeGreaterThan(0);
      expect(screen.getAllByTestId('badge-early').length).toBeGreaterThan(0);
      expect(screen.getAllByTestId('badge-team').length).toBeGreaterThan(0);
      expect(screen.getAllByTestId('badge-frequent').length).toBeGreaterThan(0);
    });
  });

  it('renders highlight cards for most active, greenest, most reliable', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockTeam }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockStats }) });

    render(<TeamLeaderboardPage />);
    await waitFor(() => {
      expect(screen.getByTestId('highlight-cards')).toBeInTheDocument();
      expect(screen.getByTestId('most-active')).toBeInTheDocument();
      expect(screen.getByTestId('greenest')).toBeInTheDocument();
      expect(screen.getByTestId('most-reliable')).toBeInTheDocument();
    });
  });

  it('renders podium for top 3', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockTeam }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockStats }) });

    render(<TeamLeaderboardPage />);
    await waitFor(() => {
      expect(screen.getByTestId('podium')).toBeInTheDocument();
      expect(screen.getByTestId('podium-1')).toBeInTheDocument();
      expect(screen.getByTestId('podium-2')).toBeInTheDocument();
      expect(screen.getByTestId('podium-3')).toBeInTheDocument();
    });
  });

  it('shows medal icons for top 3 in leaderboard rows', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockTeam }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: mockStats }) });

    render(<TeamLeaderboardPage />);
    await waitFor(() => {
      expect(screen.getByTestId('medal-1')).toBeInTheDocument();
      expect(screen.getByTestId('medal-2')).toBeInTheDocument();
      expect(screen.getByTestId('medal-3')).toBeInTheDocument();
    });
  });

  it('fetches with auth headers and credentials', async () => {
    global.fetch = vi.fn()
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: [] }) })
      .mockResolvedValueOnce({ json: () => Promise.resolve({ success: true, data: {} }) });

    render(<TeamLeaderboardPage />);
    await waitFor(() => screen.getByTestId('leaderboard-page'));

    expect(global.fetch).toHaveBeenCalledWith(
      '/api/v1/team',
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
});
