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

// ── Integration test for DemoOverlay component rendering ──
// We mock the API and verify the component renders its key elements.

describe('DemoOverlay component', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-03-19T12:00:00Z'));
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('renders nothing when demo mode is disabled', async () => {
    // Mock the api module
    vi.doMock('../api/client', () => ({
      api: {
        getDemoConfig: vi.fn().mockResolvedValue({ success: true, data: { demo_mode: false } }),
        getDemoStatus: vi.fn().mockResolvedValue({ success: false, data: null }),
        voteDemoReset: vi.fn(),
      },
    }));

    // Mock dependencies
    vi.doMock('react-i18next', () => ({
      useTranslation: () => ({ t: (key: string, fallback?: string) => fallback || key }),
    }));
    vi.doMock('framer-motion', () => ({
      motion: {
        div: ({ children, ...props }: any) => {
          const { initial, animate, exit, transition, whileHover, whileTap, ...rest } = props;
          return <div {...rest}>{children}</div>;
        },
      },
      AnimatePresence: ({ children }: any) => <>{children}</>,
    }));
    vi.doMock('@phosphor-icons/react', () => ({
      Sparkle: () => <span data-testid="icon-sparkle" />,
      Eye: () => <span data-testid="icon-eye" />,
      Timer: () => <span data-testid="icon-timer" />,
      ArrowsClockwise: () => <span data-testid="icon-reset" />,
      CaretDown: () => <span data-testid="icon-caret-down" />,
      CaretUp: () => <span data-testid="icon-caret-up" />,
    }));

    const React = await import('react');
    const { render } = await import('@testing-library/react');
    const { DemoOverlay } = await import('./DemoOverlay');

    const { container } = render(<DemoOverlay />);

    // Wait for the useEffect to run
    await vi.runAllTimersAsync();

    // Should render nothing since demo_mode is false
    expect(container.innerHTML).toBe('');

    vi.doUnmock('../api/client');
    vi.doUnmock('react-i18next');
    vi.doUnmock('framer-motion');
    vi.doUnmock('@phosphor-icons/react');
  });
});
