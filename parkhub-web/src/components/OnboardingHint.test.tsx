import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockIsEnabled = vi.fn();

vi.mock('../context/FeaturesContext', () => ({
  useFeatures: () => ({ isEnabled: mockIsEnabled }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, variants, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  Lightbulb: (props: any) => <span data-testid="icon-lightbulb" {...props} />,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key === 'common.dismiss' ? 'Dismiss' : key,
  }),
}));

import { OnboardingHint, resetAllHints } from './OnboardingHint';

describe('OnboardingHint', () => {
  let localStorageStore: Record<string, string> = {};

  beforeEach(() => {
    vi.useFakeTimers();
    localStorageStore = {};
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation((key: string) => localStorageStore[key] ?? null);
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation((key: string, val: string) => { localStorageStore[key] = val; });
    vi.spyOn(Storage.prototype, 'removeItem').mockImplementation((key: string) => { delete localStorageStore[key]; });
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it('returns null when onboarding_hints is disabled', () => {
    mockIsEnabled.mockReturnValue(false);
    const { container } = render(
      <OnboardingHint id="test-hint" message="Hello" />
    );
    expect(container.innerHTML).toBe('');
  });

  it('shows hint after 800ms delay when enabled and not dismissed', async () => {
    mockIsEnabled.mockReturnValue(true);
    render(<OnboardingHint id="test-hint" message="Try this feature!" />);

    // Not visible yet
    expect(screen.queryByText('Try this feature!')).not.toBeInTheDocument();

    // Advance past delay
    await act(async () => {
      vi.advanceTimersByTime(900);
    });

    expect(screen.getByText('Try this feature!')).toBeInTheDocument();
  });

  it('does not show hint if already dismissed', async () => {
    mockIsEnabled.mockReturnValue(true);
    localStorageStore['parkhub_hint_test-hint'] = '1';

    render(<OnboardingHint id="test-hint" message="Hello" />);

    await act(async () => {
      vi.advanceTimersByTime(1000);
    });

    expect(screen.queryByText('Hello')).not.toBeInTheDocument();
  });

  it('dismisses hint on X button click and persists', async () => {
    mockIsEnabled.mockReturnValue(true);

    render(<OnboardingHint id="dismiss-me" message="Dismiss me" />);

    await act(async () => {
      vi.advanceTimersByTime(900);
    });

    expect(screen.getByText('Dismiss me')).toBeInTheDocument();

    const dismissBtn = screen.getByLabelText('Dismiss');
    await act(async () => {
      dismissBtn.click();
    });

    expect(localStorageStore['parkhub_hint_dismiss-me']).toBe('1');
  });

  it('renders with position top', async () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(
      <OnboardingHint id="top-hint" message="Top hint" position="top" />
    );

    await act(async () => {
      vi.advanceTimersByTime(900);
    });

    expect(screen.getByText('Top hint')).toBeInTheDocument();
    // Check that position classes are applied
    expect(container.innerHTML).toContain('bottom-full');
  });

  it('renders with position bottom by default', async () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(
      <OnboardingHint id="bottom-hint" message="Bottom hint" />
    );

    await act(async () => {
      vi.advanceTimersByTime(900);
    });

    expect(container.innerHTML).toContain('top-full');
  });

  it('accepts custom icon', async () => {
    mockIsEnabled.mockReturnValue(true);
    const CustomIcon = (props: any) => <span data-testid="custom-icon" {...props} />;

    render(
      <OnboardingHint id="custom-icon-hint" message="Custom" icon={CustomIcon} />
    );

    await act(async () => {
      vi.advanceTimersByTime(900);
    });

    expect(screen.getByTestId('custom-icon')).toBeInTheDocument();
  });

  it('applies additional className', async () => {
    mockIsEnabled.mockReturnValue(true);
    const { container } = render(
      <OnboardingHint id="class-hint" message="Classed" className="extra-cls" />
    );

    await act(async () => {
      vi.advanceTimersByTime(900);
    });

    expect(container.innerHTML).toContain('extra-cls');
  });
});

describe('resetAllHints', () => {
  it('removes all parkhub_hint_ keys from localStorage', () => {
    const store: Record<string, string> = {
      'parkhub_hint_a': '1',
      'parkhub_hint_b': '1',
      'other_key': 'keep',
    };
    const removeSpy = vi.spyOn(Storage.prototype, 'removeItem').mockImplementation((key: string) => { delete store[key]; });
    vi.spyOn(Object, 'keys').mockReturnValueOnce(Object.keys(store));
    // We need to mock localStorage properly
    Object.defineProperty(window, 'localStorage', {
      value: {
        getItem: (key: string) => store[key] ?? null,
        setItem: (key: string, val: string) => { store[key] = val; },
        removeItem: (key: string) => { delete store[key]; },
        clear: () => {},
        key: (i: number) => Object.keys(store)[i],
        get length() { return Object.keys(store).length; },
      },
      writable: true,
      configurable: true,
    });

    resetAllHints();

    expect(store['other_key']).toBe('keep');
    expect(store['parkhub_hint_a']).toBeUndefined();
    expect(store['parkhub_hint_b']).toBeUndefined();
  });
});
