import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Hoisted API mocks ──
const { mockGetLoginHistory, mockGetSessions, mockRevokeSession } = vi.hoisted(() => ({
  mockGetLoginHistory: vi.fn(),
  mockGetSessions: vi.fn(),
  mockRevokeSession: vi.fn(),
}));

vi.mock('../api/client', () => ({
  api: {
    getLoginHistory: mockGetLoginHistory,
    getSessions: mockGetSessions,
    revokeSession: mockRevokeSession,
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock('react-hot-toast', () => ({
  default: { success: vi.fn(), error: vi.fn() },
}));

vi.mock('@phosphor-icons/react', () => ({
  ClockCounterClockwise: (props: any) => <span data-testid="icon-ClockCounterClockwise" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-SpinnerGap" {...props} />,
  Desktop: (props: any) => <span data-testid="icon-Desktop" {...props} />,
  Globe: (props: any) => <span data-testid="icon-Globe" {...props} />,
  ShieldWarning: (props: any) => <span data-testid="icon-ShieldWarning" {...props} />,
  Check: (props: any) => <span data-testid="icon-Check" {...props} />,
}));

import { LoginHistoryComponent } from './LoginHistory';
import toast from 'react-hot-toast';

const sampleHistory = [
  { timestamp: '2026-04-14T10:00:00Z', ip_address: '192.168.1.1', user_agent: 'Mozilla/5.0 Chrome/120', success: true },
  { timestamp: '2026-04-13T08:00:00Z', ip_address: '10.0.0.1', user_agent: 'Mozilla/5.0 Firefox/115', success: false },
];

const sampleSessions = [
  { id: 'sess-1', username: 'alice', role: 'user', created_at: '2026-04-14T08:00:00Z', expires_at: '2026-04-15T08:00:00Z', is_current: true },
  { id: 'sess-2', username: 'alice', role: 'user', created_at: '2026-04-13T08:00:00Z', expires_at: '2026-04-14T08:00:00Z', is_current: false },
];

describe('LoginHistoryComponent', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading state initially', () => {
    // Never resolve to keep loading
    mockGetLoginHistory.mockReturnValue(new Promise(() => {}));
    mockGetSessions.mockReturnValue(new Promise(() => {}));

    render(<LoginHistoryComponent />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('renders history tab with entries', async () => {
    mockGetLoginHistory.mockResolvedValue({ success: true, data: sampleHistory });
    mockGetSessions.mockResolvedValue({ success: true, data: sampleSessions });

    render(<LoginHistoryComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading...')).not.toBeInTheDocument();
    });

    // History tab is default
    expect(screen.getByText('Successful login')).toBeInTheDocument();
    expect(screen.getByText('Failed login attempt')).toBeInTheDocument();
    expect(screen.getByText('192.168.1.1')).toBeInTheDocument();
    expect(screen.getByText('Chrome')).toBeInTheDocument();
    expect(screen.getByText('Firefox')).toBeInTheDocument();
  });

  it('shows empty history message', async () => {
    mockGetLoginHistory.mockResolvedValue({ success: true, data: [] });
    mockGetSessions.mockResolvedValue({ success: true, data: [] });

    render(<LoginHistoryComponent />);

    await waitFor(() => {
      expect(screen.getByText('No login history')).toBeInTheDocument();
    });
  });

  it('switches to sessions tab', async () => {
    const user = userEvent.setup();
    mockGetLoginHistory.mockResolvedValue({ success: true, data: sampleHistory });
    mockGetSessions.mockResolvedValue({ success: true, data: sampleSessions });

    render(<LoginHistoryComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading...')).not.toBeInTheDocument();
    });

    // Click sessions tab
    await user.click(screen.getByText(/Active Sessions/));

    expect(screen.getByText('Session sess-1')).toBeInTheDocument();
    expect(screen.getByText('Session sess-2')).toBeInTheDocument();
    expect(screen.getByText('Current')).toBeInTheDocument();
  });

  it('shows session count in tab label', async () => {
    mockGetLoginHistory.mockResolvedValue({ success: true, data: [] });
    mockGetSessions.mockResolvedValue({ success: true, data: sampleSessions });

    render(<LoginHistoryComponent />);

    await waitFor(() => {
      expect(screen.getByText(/Active Sessions \(2\)/)).toBeInTheDocument();
    });
  });

  it('does not show revoke button for current session', async () => {
    const user = userEvent.setup();
    mockGetLoginHistory.mockResolvedValue({ success: true, data: [] });
    mockGetSessions.mockResolvedValue({ success: true, data: [sampleSessions[0]] }); // only current

    render(<LoginHistoryComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading...')).not.toBeInTheDocument();
    });

    await user.click(screen.getByText(/Active Sessions/));

    expect(screen.queryByText('Revoke')).not.toBeInTheDocument();
  });

  it('shows revoke button for non-current sessions', async () => {
    const user = userEvent.setup();
    mockGetLoginHistory.mockResolvedValue({ success: true, data: [] });
    mockGetSessions.mockResolvedValue({ success: true, data: sampleSessions });

    render(<LoginHistoryComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading...')).not.toBeInTheDocument();
    });

    await user.click(screen.getByText(/Active Sessions/));
    expect(screen.getByText('Revoke')).toBeInTheDocument();
  });

  it('revokes a session successfully', async () => {
    const user = userEvent.setup();
    mockGetLoginHistory.mockResolvedValue({ success: true, data: [] });
    mockGetSessions.mockResolvedValue({ success: true, data: sampleSessions });
    mockRevokeSession.mockResolvedValue({ success: true });

    render(<LoginHistoryComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading...')).not.toBeInTheDocument();
    });

    await user.click(screen.getByText(/Active Sessions/));
    await user.click(screen.getByText('Revoke'));

    await waitFor(() => {
      expect(mockRevokeSession).toHaveBeenCalledWith('sess-2');
    });
    expect(toast.success).toHaveBeenCalledWith('Session revoked');
  });

  it('shows error toast on revoke failure', async () => {
    const user = userEvent.setup();
    mockGetLoginHistory.mockResolvedValue({ success: true, data: [] });
    mockGetSessions.mockResolvedValue({ success: true, data: sampleSessions });
    mockRevokeSession.mockResolvedValue({ success: false, error: { message: 'Unauthorized' } });

    render(<LoginHistoryComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading...')).not.toBeInTheDocument();
    });

    await user.click(screen.getByText(/Active Sessions/));
    await user.click(screen.getByText('Revoke'));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('Unauthorized');
    });
  });

  it('handles API failure gracefully', async () => {
    mockGetLoginHistory.mockRejectedValue(new Error('Network error'));
    mockGetSessions.mockRejectedValue(new Error('Network error'));

    render(<LoginHistoryComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading...')).not.toBeInTheDocument();
    });

    // Should render without crashing, showing empty state
    expect(screen.getByText('No login history')).toBeInTheDocument();
  });

  it('shows empty sessions message', async () => {
    const user = userEvent.setup();
    mockGetLoginHistory.mockResolvedValue({ success: true, data: [] });
    mockGetSessions.mockResolvedValue({ success: true, data: [] });

    render(<LoginHistoryComponent />);

    await waitFor(() => {
      expect(screen.queryByText('Loading...')).not.toBeInTheDocument();
    });

    await user.click(screen.getByText(/Active Sessions/));
    expect(screen.getByText('No active sessions')).toBeInTheDocument();
  });

  it('renders Security heading', async () => {
    mockGetLoginHistory.mockResolvedValue({ success: true, data: [] });
    mockGetSessions.mockResolvedValue({ success: true, data: [] });

    render(<LoginHistoryComponent />);

    await waitFor(() => {
      expect(screen.getByText('Security')).toBeInTheDocument();
    });
  });
});
