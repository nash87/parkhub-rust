import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Hoisted mocks ──
const { localStorageMock, matchMediaState } = vi.hoisted(() => {
  let store: Record<string, string> = {};
  const localStorageMock = {
    getItem: vi.fn((key: string) => store[key] ?? null),
    setItem: vi.fn((key: string, val: string) => { store[key] = val; }),
    removeItem: vi.fn((key: string) => { delete store[key]; }),
    clear: vi.fn(() => { store = {}; }),
  };

  const matchMediaState = { dark: false, listeners: [] as Array<() => void> };

  const persistentMql = {
    get matches() { return matchMediaState.dark; },
    media: '(prefers-color-scheme: dark)',
    addEventListener: vi.fn((_event: string, handler: () => void) => {
      matchMediaState.listeners.push(handler);
    }),
    removeEventListener: vi.fn(),
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    dispatchEvent: vi.fn(),
  };

  Object.defineProperty(globalThis.window ?? globalThis, 'localStorage', {
    value: localStorageMock, writable: true, configurable: true,
  });
  Object.defineProperty(globalThis.window ?? globalThis, 'matchMedia', {
    writable: true, configurable: true,
    value: vi.fn((_query: string) => persistentMql),
  });

  return { localStorageMock, matchMediaState };
});

// Mock i18next
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string | Record<string, string>) => {
      if (typeof fallback === 'string') return fallback;
      return key;
    },
  }),
}));

// Mock framer-motion to avoid animation complexity in tests
vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, ...props }: any, ref: any) => <div ref={ref} {...props}>{children}</div>),
    button: React.forwardRef(({ children, ...props }: any, ref: any) => <button ref={ref} {...props}>{children}</button>),
    aside: React.forwardRef(({ children, ...props }: any, ref: any) => <aside ref={ref} {...props}>{children}</aside>),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

import { ThemeProvider } from '../context/ThemeContext';
import { ThemeSwitcher, ThemeSwitcherFab } from './ThemeSwitcher';

describe('ThemeSwitcher', () => {
  beforeEach(() => {
    localStorageMock.clear();
    matchMediaState.dark = false;
    vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({ ok: true, json: () => Promise.resolve({}) })));
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders all 12 theme cards when open', () => {
    render(
      <ThemeProvider>
        <ThemeSwitcher open={true} onClose={() => {}} />
      </ThemeProvider>,
    );

    // Each theme card has an aria-label with theme name
    expect(screen.getByRole('dialog')).toBeTruthy();
    const buttons = screen.getAllByRole('button', { pressed: undefined });
    // 12 theme cards + close button = 13 buttons
    const themeButtons = buttons.filter(b => b.getAttribute('aria-pressed') !== null);
    expect(themeButtons).toHaveLength(12);
  });

  it('does not render when closed', () => {
    const { container } = render(
      <ThemeProvider>
        <ThemeSwitcher open={false} onClose={() => {}} />
      </ThemeProvider>,
    );

    expect(container.querySelector('[role="dialog"]')).toBeNull();
  });

  it('marks the active theme with aria-pressed=true', () => {
    render(
      <ThemeProvider>
        <ThemeSwitcher open={true} onClose={() => {}} />
      </ThemeProvider>,
    );

    const activeButtons = screen.getAllByRole('button').filter(
      b => b.getAttribute('aria-pressed') === 'true'
    );
    expect(activeButtons).toHaveLength(1);
  });

  it('calls onClose when close button is clicked', async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();

    render(
      <ThemeProvider>
        <ThemeSwitcher open={true} onClose={onClose} />
      </ThemeProvider>,
    );

    const closeBtn = screen.getByLabelText('Close');
    await user.click(closeBtn);
    expect(onClose).toHaveBeenCalled();
  });

  it('changes theme when a card is clicked', async () => {
    const user = userEvent.setup();

    render(
      <ThemeProvider>
        <ThemeSwitcher open={true} onClose={() => {}} />
      </ThemeProvider>,
    );

    // Find the Neon theme card — it's a button with aria-pressed that contains "Neon" text
    const themeButtons = screen.getAllByRole('button').filter(
      b => b.getAttribute('aria-pressed') !== null
    );
    // Neon is the 5th theme (index 4)
    const neonCard = themeButtons[4];
    await user.click(neonCard);

    expect(localStorageMock.setItem).toHaveBeenCalledWith('parkhub_design_theme', 'neon');
  });
});

describe('ThemeSwitcherFab', () => {
  beforeEach(() => {
    localStorageMock.clear();
    matchMediaState.dark = false;
  });

  it('renders the FAB button', () => {
    render(
      <ThemeProvider>
        <ThemeSwitcherFab onClick={() => {}} />
      </ThemeProvider>,
    );

    expect(screen.getByLabelText('Change theme')).toBeTruthy();
  });

  it('calls onClick when clicked', async () => {
    const onClick = vi.fn();
    const user = userEvent.setup();

    render(
      <ThemeProvider>
        <ThemeSwitcherFab onClick={onClick} />
      </ThemeProvider>,
    );

    await user.click(screen.getByLabelText('Change theme'));
    expect(onClick).toHaveBeenCalled();
  });
});
