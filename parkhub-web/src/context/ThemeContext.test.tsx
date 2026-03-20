import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Hoisted mocks ──
// vi.hoisted runs BEFORE imports are evaluated, so the module-level
// `window.matchMedia(...)` in ThemeContext.tsx sees our mock.
const { localStorageMock, matchMediaState, persistentMql } = vi.hoisted(() => {
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

  // Install mocks on globals before any module code runs
  Object.defineProperty(globalThis.window ?? globalThis, 'localStorage', {
    value: localStorageMock, writable: true, configurable: true,
  });
  Object.defineProperty(globalThis.window ?? globalThis, 'matchMedia', {
    writable: true, configurable: true,
    value: vi.fn((_query: string) => persistentMql),
  });

  return { localStorageMock, matchMediaState, persistentMql };
});

import { ThemeProvider, useTheme } from './ThemeContext';

// Helper component to consume the context
function ThemeConsumer() {
  const { theme, resolved, setTheme } = useTheme();
  return (
    <div>
      <span data-testid="theme">{theme}</span>
      <span data-testid="resolved">{resolved}</span>
      <button data-testid="set-dark" onClick={() => setTheme('dark')}>Dark</button>
      <button data-testid="set-light" onClick={() => setTheme('light')}>Light</button>
      <button data-testid="set-system" onClick={() => setTheme('system')}>System</button>
    </div>
  );
}

describe('ThemeContext', () => {
  beforeEach(() => {
    localStorageMock.clear();
    matchMediaState.dark = false;
    matchMediaState.listeners.length = 0;
    document.documentElement.classList.remove('dark');
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('useTheme throws outside ThemeProvider', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(() => render(<ThemeConsumer />)).toThrow(
      'useTheme must be used within ThemeProvider',
    );

    spy.mockRestore();
  });

  it('defaults to system theme when no localStorage value', () => {
    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    expect(screen.getByTestId('theme').textContent).toBe('system');
  });

  it('resolved theme is light when system prefers light', () => {
    matchMediaState.dark = false;

    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    expect(screen.getByTestId('resolved').textContent).toBe('light');
  });

  it('resolved theme is dark when system prefers dark', () => {
    matchMediaState.dark = true;

    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    expect(screen.getByTestId('resolved').textContent).toBe('dark');
  });

  it('setTheme updates theme state', async () => {
    const user = userEvent.setup();

    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    expect(screen.getByTestId('theme').textContent).toBe('system');

    await user.click(screen.getByTestId('set-dark'));
    expect(screen.getByTestId('theme').textContent).toBe('dark');
    expect(screen.getByTestId('resolved').textContent).toBe('dark');

    await user.click(screen.getByTestId('set-light'));
    expect(screen.getByTestId('theme').textContent).toBe('light');
    expect(screen.getByTestId('resolved').textContent).toBe('light');
  });

  it('persists theme to localStorage', async () => {
    const user = userEvent.setup();

    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    await user.click(screen.getByTestId('set-dark'));
    expect(localStorageMock.setItem).toHaveBeenCalledWith('parkhub_theme', 'dark');

    await user.click(screen.getByTestId('set-light'));
    expect(localStorageMock.setItem).toHaveBeenCalledWith('parkhub_theme', 'light');
  });

  it('reads initial theme from localStorage', () => {
    localStorageMock.setItem('parkhub_theme', 'dark');

    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    expect(screen.getByTestId('theme').textContent).toBe('dark');
    expect(screen.getByTestId('resolved').textContent).toBe('dark');
  });

  it('toggles dark class on document element', async () => {
    const user = userEvent.setup();

    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    await user.click(screen.getByTestId('set-dark'));
    expect(document.documentElement.classList.contains('dark')).toBe(true);

    await user.click(screen.getByTestId('set-light'));
    expect(document.documentElement.classList.contains('dark')).toBe(false);
  });

  it('setTheme to system uses system preference', async () => {
    const user = userEvent.setup();
    matchMediaState.dark = true;

    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    // First set to light explicitly
    await user.click(screen.getByTestId('set-light'));
    expect(screen.getByTestId('resolved').textContent).toBe('light');

    // Then switch to system — which prefers dark
    await user.click(screen.getByTestId('set-system'));
    expect(screen.getByTestId('theme').textContent).toBe('system');
    expect(screen.getByTestId('resolved').textContent).toBe('dark');
  });
});
