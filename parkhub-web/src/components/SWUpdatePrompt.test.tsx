import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  ArrowsClockwise: (props: any) => <span data-testid="icon-reload" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => fallback || key,
  }),
}));

import { SWUpdatePrompt } from './SWUpdatePrompt';

describe('SWUpdatePrompt', () => {
  let swReady: (reg: any) => void;
  let controllerChangeListeners: (() => void)[] = [];
  const originalLocation = window.location;

  beforeEach(() => {
    controllerChangeListeners = [];

    const swMock = {
      ready: new Promise<any>((resolve) => { swReady = resolve; }),
      addEventListener: vi.fn((event: string, handler: any) => {
        if (event === 'controllerchange') controllerChangeListeners.push(handler);
      }),
      removeEventListener: vi.fn(),
      controller: {},
    };

    Object.defineProperty(navigator, 'serviceWorker', {
      value: swMock,
      writable: true,
      configurable: true,
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders nothing when no service worker is waiting', () => {
    const { container } = render(<SWUpdatePrompt />);
    // No update banner
    expect(screen.queryByText('A new version is available')).not.toBeInTheDocument();
  });

  it('shows update banner when a worker is already waiting', async () => {
    render(<SWUpdatePrompt />);

    const mockWorker = { postMessage: vi.fn() };
    act(() => {
      swReady({ waiting: mockWorker, addEventListener: vi.fn() });
    });

    await waitFor(() => {
      expect(screen.getByText('A new version is available')).toBeInTheDocument();
    });
  });

  it('reload button sends SKIP_WAITING to worker', async () => {
    const user = userEvent.setup();
    render(<SWUpdatePrompt />);

    const mockWorker = { postMessage: vi.fn() };
    act(() => {
      swReady({ waiting: mockWorker, addEventListener: vi.fn() });
    });

    await waitFor(() => {
      expect(screen.getByText('Reload')).toBeInTheDocument();
    });

    await user.click(screen.getByText('Reload'));

    expect(mockWorker.postMessage).toHaveBeenCalledWith({ type: 'SKIP_WAITING' });
  });

  it('dismiss button hides the banner', async () => {
    const user = userEvent.setup();
    render(<SWUpdatePrompt />);

    const mockWorker = { postMessage: vi.fn() };
    act(() => {
      swReady({ waiting: mockWorker, addEventListener: vi.fn() });
    });

    await waitFor(() => {
      expect(screen.getByText('A new version is available')).toBeInTheDocument();
    });

    await user.click(screen.getByLabelText('Dismiss'));

    expect(screen.queryByText('A new version is available')).not.toBeInTheDocument();
  });

  it('does nothing when navigator.serviceWorker is missing', () => {
    delete (navigator as any).serviceWorker;
    const { container } = render(<SWUpdatePrompt />);
    expect(container).toBeTruthy();
  });

  it('reloads page when controllerchange fires', async () => {
    const originalLocation = window.location;
    Object.defineProperty(window, 'location', {
      configurable: true,
      value: { ...originalLocation, reload: vi.fn() },
    });

    render(<SWUpdatePrompt />);
    expect(controllerChangeListeners.length).toBeGreaterThan(0);
    act(() => { controllerChangeListeners[0](); });
    act(() => { controllerChangeListeners[0](); });
    expect(window.location.reload).toHaveBeenCalledTimes(1);

    Object.defineProperty(window, 'location', { configurable: true, value: originalLocation });
  });

  it('shows banner when updatefound fires and new worker is installed', async () => {
    let updateFoundHandler: () => void = () => {};
    let stateChangeHandler: (this: any) => void = () => {};

    const newWorker = {
      state: 'installing',
      addEventListener: vi.fn((event: string, handler: any) => {
        if (event === 'statechange') stateChangeHandler = handler;
      }),
      postMessage: vi.fn(),
    };

    const mockReg = {
      waiting: null,
      installing: newWorker,
      addEventListener: vi.fn((event: string, handler: any) => {
        if (event === 'updatefound') updateFoundHandler = handler;
      }),
    };

    render(<SWUpdatePrompt />);

    await act(async () => {
      swReady(mockReg);
    });

    await act(async () => {
      updateFoundHandler();
    });

    // Simulate state change to installed
    newWorker.state = 'installed';
    await act(async () => {
      stateChangeHandler.call(newWorker);
    });

    await waitFor(() => {
      expect(screen.getByText('A new version is available')).toBeInTheDocument();
    });
  });
});
