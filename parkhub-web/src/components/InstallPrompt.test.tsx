import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  DownloadSimple: (props: any) => <span data-testid="icon-download" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => opts?.defaultValue || key,
  }),
}));

import { InstallPrompt } from './InstallPrompt';

describe('InstallPrompt', () => {
  let localStorageStore: Record<string, string> = {};
  const originalMatchMedia = window.matchMedia;
  let swMessageListeners: ((e: MessageEvent) => void)[] = [];

  beforeEach(() => {
    localStorageStore = {};
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation((key: string) => localStorageStore[key] ?? null);
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation((key: string, val: string) => { localStorageStore[key] = val; });

    // Mock matchMedia for standalone check
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    // Mock navigator.serviceWorker
    swMessageListeners = [];
    Object.defineProperty(navigator, 'serviceWorker', {
      value: {
        addEventListener: vi.fn((_, handler) => swMessageListeners.push(handler)),
        removeEventListener: vi.fn(),
        controller: { postMessage: vi.fn() },
      },
      writable: true,
      configurable: true,
    });
  });

  afterEach(() => {
    window.matchMedia = originalMatchMedia;
    vi.restoreAllMocks();
  });

  it('renders without crash (no visible install banner by default)', () => {
    const { container } = render(<InstallPrompt />);
    // No visible banner initially (no beforeinstallprompt event fired)
    expect(container).toBeTruthy();
  });

  it('shows install banner when beforeinstallprompt fires', async () => {
    render(<InstallPrompt />);

    // Fire the beforeinstallprompt event
    const event = new Event('beforeinstallprompt');
    Object.defineProperty(event, 'preventDefault', { value: vi.fn() });
    window.dispatchEvent(event);

    await waitFor(() => {
      expect(screen.getByText('Install ParkHub')).toBeInTheDocument();
    });
  });

  it('does not show banner if dismissed recently', () => {
    localStorageStore['parkhub_install_dismissed'] = Date.now().toString();
    render(<InstallPrompt />);

    const event = new Event('beforeinstallprompt');
    window.dispatchEvent(event);

    expect(screen.queryByText('Install ParkHub')).not.toBeInTheDocument();
  });

  it('does not show banner if in standalone mode', () => {
    window.matchMedia = vi.fn().mockImplementation((query: string) => ({
      matches: query === '(display-mode: standalone)',
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }));

    render(<InstallPrompt />);

    const event = new Event('beforeinstallprompt');
    window.dispatchEvent(event);

    expect(screen.queryByText('Install ParkHub')).not.toBeInTheDocument();
  });

  it('dismiss button hides the banner and stores dismiss time', async () => {
    const user = userEvent.setup();
    render(<InstallPrompt />);

    const event = new Event('beforeinstallprompt') as any;
    event.preventDefault = vi.fn();
    window.dispatchEvent(event);

    await waitFor(() => {
      expect(screen.getByText('Install ParkHub')).toBeInTheDocument();
    });

    const notNowBtn = screen.getByText('Not now');
    await user.click(notNowBtn);

    expect(screen.queryByRole('complementary')).not.toBeInTheDocument();
    expect(localStorageStore['parkhub_install_dismissed']).toBeTruthy();
  });

  it('install button calls prompt() on deferred event', async () => {
    const user = userEvent.setup();
    render(<InstallPrompt />);

    const mockPrompt = vi.fn().mockResolvedValue(undefined);
    const event = new Event('beforeinstallprompt') as any;
    event.preventDefault = vi.fn();
    event.prompt = mockPrompt;
    event.userChoice = Promise.resolve({ outcome: 'accepted' as const });
    window.dispatchEvent(event);

    await waitFor(() => {
      expect(screen.getByText('Install')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Install'));

    await waitFor(() => {
      expect(mockPrompt).toHaveBeenCalled();
    });
  });

  it('shows sync status toast when MUTATION_QUEUED message received', async () => {
    render(<InstallPrompt />);

    act(() => {
      swMessageListeners.forEach(handler => {
        handler({ data: { type: 'MUTATION_QUEUED', queueLength: 3 } } as any);
      });
    });

    await waitFor(() => {
      expect(screen.getByRole('status')).toBeInTheDocument();
    });
  });

  it('clears sync toast on SYNC_RESULT with synced > 0', async () => {
    vi.useFakeTimers();
    render(<InstallPrompt />);

    act(() => {
      swMessageListeners.forEach(handler => {
        handler({ data: { type: 'MUTATION_QUEUED', queueLength: 2 } } as any);
      });
    });

    act(() => {
      swMessageListeners.forEach(handler => {
        handler({ data: { type: 'SYNC_RESULT', synced: 2 } } as any);
      });
    });

    // Sync message shown briefly
    expect(screen.getByRole('status')).toBeInTheDocument();

    // After 3000ms timeout, toast should clear
    await act(async () => {
      vi.advanceTimersByTime(3100);
    });

    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    vi.useRealTimers();
  });

  it('clears sync toast immediately on SYNC_RESULT with synced 0', async () => {
    render(<InstallPrompt />);

    act(() => {
      swMessageListeners.forEach(handler => {
        handler({ data: { type: 'MUTATION_QUEUED', queueLength: 1 } } as any);
      });
    });

    await waitFor(() => {
      expect(screen.getByRole('status')).toBeInTheDocument();
    });

    act(() => {
      swMessageListeners.forEach(handler => {
        handler({ data: { type: 'SYNC_RESULT', synced: 0 } } as any);
      });
    });

    await waitFor(() => {
      expect(screen.queryByRole('status')).not.toBeInTheDocument();
    });
  });

  it('sends REPLAY_SYNC_QUEUE when going online', () => {
    render(<InstallPrompt />);
    const postMessageSpy = (navigator.serviceWorker as any).controller.postMessage;

    window.dispatchEvent(new Event('online'));
    expect(postMessageSpy).toHaveBeenCalledWith({ type: 'REPLAY_SYNC_QUEUE' });
  });
});
