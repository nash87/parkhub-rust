import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// ── Unit tests for the pure utility functions ──
// These are not exported, so we replicate them here to test the logic.
// (They're small pure functions — testing the exact logic from the component.)

function formatRelativeTime(isoString: string): string {
  const diff = Math.floor((Date.now() - new Date(isoString).getTime()) / 1000);
  if (diff < 60) return `${diff}s ago`;
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  const h = Math.floor(diff / 3600);
  const m = Math.floor((diff % 3600) / 60);
  return m > 0 ? `${h}h ${m}m ago` : `${h}h ago`;
}

function formatCountdown(isoString: string): string {
  const diff = Math.max(0, Math.floor((new Date(isoString).getTime() - Date.now()) / 1000));
  if (diff === 0) return 'now';
  const h = Math.floor(diff / 3600);
  const m = Math.floor((diff % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
}

describe('formatRelativeTime', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-03-19T12:00:00Z'));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('shows seconds for < 60s', () => {
    expect(formatRelativeTime('2026-03-19T11:59:30Z')).toBe('30s ago');
  });

  it('shows minutes for < 1 hour', () => {
    expect(formatRelativeTime('2026-03-19T11:45:00Z')).toBe('15m ago');
  });

  it('shows hours and minutes for > 1 hour', () => {
    expect(formatRelativeTime('2026-03-19T09:30:00Z')).toBe('2h 30m ago');
  });

  it('shows hours only when minutes are 0', () => {
    expect(formatRelativeTime('2026-03-19T10:00:00Z')).toBe('2h ago');
  });

  it('shows 0s ago for current time', () => {
    expect(formatRelativeTime('2026-03-19T12:00:00Z')).toBe('0s ago');
  });
});

describe('formatCountdown', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-03-19T12:00:00Z'));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('shows "now" for past timestamps', () => {
    expect(formatCountdown('2026-03-19T11:00:00Z')).toBe('now');
  });

  it('shows "now" for current time', () => {
    expect(formatCountdown('2026-03-19T12:00:00Z')).toBe('now');
  });

  it('shows minutes only when < 1 hour', () => {
    expect(formatCountdown('2026-03-19T12:30:00Z')).toBe('30m');
  });

  it('shows hours and minutes when >= 1 hour', () => {
    expect(formatCountdown('2026-03-19T14:15:00Z')).toBe('2h 15m');
  });

  it('shows hours with 0 minutes', () => {
    expect(formatCountdown('2026-03-19T15:00:00Z')).toBe('3h 0m');
  });
});

// ── Integration tests for DemoOverlay component ──

import React from 'react';
import { render, screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

const mockGetDemoConfig = vi.fn();
const mockGetDemoStatus = vi.fn();
const mockVoteDemoReset = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    getDemoConfig: (...a: any[]) => mockGetDemoConfig(...a),
    getDemoStatus: (...a: any[]) => mockGetDemoStatus(...a),
    voteDemoReset: (...a: any[]) => mockVoteDemoReset(...a),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallbackOrOpts?: any, opts?: any) => {
      const map: Record<string, string> = {
        'demo.badge': 'DEMO',
        'demo.voteReset': 'Vote to Reset',
        'demo.resetting': 'Resetting...',
        'demo.lastReset': 'Last reset',
        'demo.nextReset': 'Next reset',
      };
      if (key === 'demo.votesNeeded') {
        const o = (typeof fallbackOrOpts === 'object' ? fallbackOrOpts : opts) || {};
        return `${o.current ?? 0}/${o.needed ?? 0}`;
      }
      return map[key] || (typeof fallbackOrOpts === 'string' ? fallbackOrOpts : key);
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Sparkle: (props: any) => <span data-testid="icon-sparkle" {...props} />,
  Eye: (props: any) => <span data-testid="icon-eye" {...props} />,
  Timer: (props: any) => <span data-testid="icon-timer" {...props} />,
  ArrowsClockwise: (props: any) => <span data-testid="icon-refresh" {...props} />,
  CaretDown: (props: any) => <span data-testid="icon-caret-down" {...props} />,
  CaretUp: (props: any) => <span data-testid="icon-caret-up" {...props} />,
}));

import { DemoOverlay } from './DemoOverlay';

const DEMO_STATUS = {
  timer_seconds: 1800,
  viewers: 5,
  votes: 2,
  vote_threshold: 5,
  has_voted: false,
  reset_in_progress: false,
  reset: false,
  last_reset_at: '2026-03-19T11:59:00Z',
  next_scheduled_reset: '2026-03-19T14:00:00Z',
};

describe('DemoOverlay component', () => {
  beforeEach(() => {
    mockGetDemoConfig.mockClear();
    mockGetDemoStatus.mockClear();
    mockVoteDemoReset.mockClear();
    Object.defineProperty(window, 'innerWidth', { value: 1024, writable: true, configurable: true });
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it('renders nothing when demo mode is disabled', async () => {
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: false } });
    const { container } = render(<DemoOverlay />);
    await waitFor(() => expect(mockGetDemoConfig).toHaveBeenCalled());
    expect(container.innerHTML).toBe('');
  });

  it('renders nothing when getDemoConfig fails', async () => {
    mockGetDemoConfig.mockRejectedValue(new Error('fail'));
    const { container } = render(<DemoOverlay />);
    await waitFor(() => expect(mockGetDemoConfig).toHaveBeenCalled());
    expect(container.innerHTML).toBe('');
  });

  it('renders nothing when getDemoConfig returns success=false', async () => {
    mockGetDemoConfig.mockResolvedValue({ success: false });
    const { container } = render(<DemoOverlay />);
    await waitFor(() => expect(mockGetDemoConfig).toHaveBeenCalled());
    expect(container.innerHTML).toBe('');
  });

  it('renders the overlay when demo mode is enabled and status loaded', async () => {
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: true } });
    mockGetDemoStatus.mockResolvedValue({ success: true, data: DEMO_STATUS });
    render(<DemoOverlay />);

    await waitFor(() => {
      expect(screen.getByText('DEMO')).toBeInTheDocument();
    });
  });

  it('shows timer formatted as MM:SS', async () => {
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: true } });
    mockGetDemoStatus.mockResolvedValue({ success: true, data: DEMO_STATUS });
    render(<DemoOverlay />);

    await waitFor(() => {
      // 1800s = 30:00
      expect(screen.getByText('30:00')).toBeInTheDocument();
    });
  });

  it('shows viewer count', async () => {
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: true } });
    mockGetDemoStatus.mockResolvedValue({ success: true, data: DEMO_STATUS });
    render(<DemoOverlay />);

    await waitFor(() => {
      expect(screen.getByText('5')).toBeInTheDocument();
    });
  });

  it('toggles collapse state', async () => {
    const user = userEvent.setup();
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: true } });
    mockGetDemoStatus.mockResolvedValue({ success: true, data: DEMO_STATUS });
    render(<DemoOverlay />);

    await waitFor(() => expect(screen.getByText('Vote to Reset')).toBeInTheDocument());

    // Click to collapse
    const toggleBtn = screen.getByRole('button', { expanded: true });
    await user.click(toggleBtn);

    await waitFor(() => {
      expect(screen.queryByText('Vote to Reset')).not.toBeInTheDocument();
    });

    // Click to expand again
    const collapsedBtn = screen.getByRole('button', { expanded: false });
    await user.click(collapsedBtn);

    await waitFor(() => {
      expect(screen.getByText('Vote to Reset')).toBeInTheDocument();
    });
  });

  it('handles vote button click and updates state optimistically', async () => {
    const user = userEvent.setup();
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: true } });
    mockGetDemoStatus.mockResolvedValue({ success: true, data: DEMO_STATUS });
    mockVoteDemoReset.mockResolvedValue({ success: true });
    render(<DemoOverlay />);

    await waitFor(() => expect(screen.getByText('Vote to Reset')).toBeInTheDocument());
    await user.click(screen.getByText('Vote to Reset'));

    await waitFor(() => {
      expect(mockVoteDemoReset).toHaveBeenCalled();
    });
  });

  it('disables vote when already voted', async () => {
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: true } });
    mockGetDemoStatus.mockResolvedValue({
      success: true,
      data: { ...DEMO_STATUS, has_voted: true },
    });
    render(<DemoOverlay />);

    await waitFor(() => expect(screen.getByText('DEMO')).toBeInTheDocument());
    // The vote button should be disabled
    const btns = screen.getAllByRole('button');
    const disabledBtns = btns.filter(b => b.hasAttribute('disabled'));
    expect(disabledBtns.length).toBeGreaterThan(0);
  });

  it('shows Resetting... when reset is in progress', async () => {
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: true } });
    mockGetDemoStatus.mockResolvedValue({
      success: true,
      data: { ...DEMO_STATUS, reset_in_progress: true },
    });
    render(<DemoOverlay />);

    await waitFor(() => {
      expect(screen.getByText('Resetting...')).toBeInTheDocument();
    });
  });

  it('shows last reset and next reset info', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-03-19T12:00:00Z'));
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: true } });
    mockGetDemoStatus.mockResolvedValue({ success: true, data: DEMO_STATUS });
    await act(async () => {
      render(<DemoOverlay />);
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(screen.getByText('Last reset')).toBeInTheDocument();
    expect(screen.getByText('Next reset')).toBeInTheDocument();
    expect(screen.getByText('1m ago')).toBeInTheDocument();
    expect(screen.getByText('2h 0m')).toBeInTheDocument();
  });

  it('does not show last/next reset when they are null', async () => {
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: true } });
    mockGetDemoStatus.mockResolvedValue({
      success: true,
      data: { ...DEMO_STATUS, last_reset_at: null, next_scheduled_reset: null },
    });
    render(<DemoOverlay />);

    await waitFor(() => expect(screen.getByText('DEMO')).toBeInTheDocument());
    expect(screen.queryByText('Last reset')).not.toBeInTheDocument();
    expect(screen.queryByText('Next reset')).not.toBeInTheDocument();
  });

  it('applies red color for low timer (<300s)', async () => {
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: true } });
    mockGetDemoStatus.mockResolvedValue({
      success: true,
      data: { ...DEMO_STATUS, timer_seconds: 120 },
    });
    render(<DemoOverlay />);

    await waitFor(() => {
      const timer = screen.getByText('02:00');
      expect(timer.className).toContain('text-red-700');
    });
  });

  it('handles vote_threshold of 0 without division by zero', async () => {
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: true } });
    mockGetDemoStatus.mockResolvedValue({
      success: true,
      data: { ...DEMO_STATUS, vote_threshold: 0, votes: 0 },
    });
    render(<DemoOverlay />);

    await waitFor(() => {
      expect(screen.getByRole('progressbar')).toBeInTheDocument();
    });
  });

  it('renders nothing when status fetch fails', async () => {
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: true } });
    mockGetDemoStatus.mockRejectedValue(new Error('net'));
    const { container } = render(<DemoOverlay />);
    await waitFor(() => expect(mockGetDemoConfig).toHaveBeenCalled());
    await waitFor(() => {
      expect(container.querySelector('.glass-card')).toBeNull();
    });
  });

  it('defaults to collapsed on small screens', async () => {
    Object.defineProperty(window, 'innerWidth', { value: 400, writable: true, configurable: true });
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: true } });
    mockGetDemoStatus.mockResolvedValue({ success: true, data: DEMO_STATUS });
    render(<DemoOverlay />);

    await waitFor(() => expect(screen.getByText('DEMO')).toBeInTheDocument());
    // Should be collapsed (no vote button visible)
    expect(screen.queryByText('Vote to Reset')).not.toBeInTheDocument();
  });

  it('reloads when the backend signals a reset', async () => {
    const reloadPage = vi.fn();
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: true } });
    mockGetDemoStatus.mockResolvedValue({
      success: true,
      data: { ...DEMO_STATUS, reset: true },
    });

    render(<DemoOverlay reloadPage={reloadPage} />);

    await waitFor(() => {
      expect(reloadPage).toHaveBeenCalledOnce();
    });
  });

  it('updates the local countdown every second while expanded', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-03-19T12:00:00Z'));
    mockGetDemoConfig.mockResolvedValue({ success: true, data: { demo_mode: true } });
    mockGetDemoStatus.mockResolvedValue({ success: true, data: DEMO_STATUS });

    render(<DemoOverlay />);

    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(screen.getByText('30:00')).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(screen.getByText((_, element) => element?.textContent === '29:59')).toBeInTheDocument();
  });
});
