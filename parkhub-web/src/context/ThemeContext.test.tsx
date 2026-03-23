import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Hoisted mocks ──
// vi.hoisted runs BEFORE imports are evaluated, so the module-level
// `window.matchMedia(...)` in ThemeContext.tsx sees our mock.
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

import { ThemeProvider, useTheme, DESIGN_THEMES, type DesignThemeId } from './ThemeContext';

// Helper component to consume the context
function ThemeConsumer() {
  const { theme, resolved, setTheme, designTheme, setDesignTheme, designThemes, currentDesignTheme } = useTheme();
  return (
    <div>
      <span data-testid="theme">{theme}</span>
      <span data-testid="resolved">{resolved}</span>
      <span data-testid="design-theme">{designTheme}</span>
      <span data-testid="design-theme-name">{currentDesignTheme.name}</span>
      <span data-testid="design-themes-count">{designThemes.length}</span>
      <button data-testid="set-dark" onClick={() => setTheme('dark')}>Dark</button>
      <button data-testid="set-light" onClick={() => setTheme('light')}>Light</button>
      <button data-testid="set-system" onClick={() => setTheme('system')}>System</button>
      <button data-testid="set-glass" onClick={() => setDesignTheme('glass')}>Glass</button>
      <button data-testid="set-neon" onClick={() => setDesignTheme('neon')}>Neon</button>
      <button data-testid="set-brutalist" onClick={() => setDesignTheme('brutalist')}>Brutalist</button>
      <button data-testid="set-warm" onClick={() => setDesignTheme('warm')}>Warm</button>
      <button data-testid="set-bento" onClick={() => setDesignTheme('bento')}>Bento</button>
      <button data-testid="set-classic" onClick={() => setDesignTheme('classic')}>Classic</button>
      <button data-testid="set-liquid" onClick={() => setDesignTheme('liquid')}>Liquid</button>
      <button data-testid="set-mono" onClick={() => setDesignTheme('mono')}>Mono</button>
      <button data-testid="set-ocean" onClick={() => setDesignTheme('ocean')}>Ocean</button>
      <button data-testid="set-forest" onClick={() => setDesignTheme('forest')}>Forest</button>
      <button data-testid="set-synthwave" onClick={() => setDesignTheme('synthwave')}>Synthwave</button>
      <button data-testid="set-zen" onClick={() => setDesignTheme('zen')}>Zen</button>
    </div>
  );
}

describe('ThemeContext', () => {
  beforeEach(() => {
    localStorageMock.clear();
    matchMediaState.dark = false;
    matchMediaState.listeners.length = 0;
    document.documentElement.classList.remove('dark');
    delete document.documentElement.dataset.designTheme;
    // Mock fetch for design theme sync
    vi.stubGlobal('fetch', vi.fn(() => Promise.resolve({ ok: true, json: () => Promise.resolve({}) })));
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

  // ── Design Theme Tests ──

  it('defaults design theme to classic', () => {
    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    expect(screen.getByTestId('design-theme').textContent).toBe('classic');
    expect(screen.getByTestId('design-theme-name').textContent).toBe('Classic');
  });

  it('exposes all 12 design themes', () => {
    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    expect(screen.getByTestId('design-themes-count').textContent).toBe('12');
  });

  it('setDesignTheme changes the active design theme', async () => {
    const user = userEvent.setup();

    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    await user.click(screen.getByTestId('set-glass'));
    expect(screen.getByTestId('design-theme').textContent).toBe('glass');

    await user.click(screen.getByTestId('set-neon'));
    expect(screen.getByTestId('design-theme').textContent).toBe('neon');

    await user.click(screen.getByTestId('set-brutalist'));
    expect(screen.getByTestId('design-theme').textContent).toBe('brutalist');

    await user.click(screen.getByTestId('set-warm'));
    expect(screen.getByTestId('design-theme').textContent).toBe('warm');

    await user.click(screen.getByTestId('set-bento'));
    expect(screen.getByTestId('design-theme').textContent).toBe('bento');

    await user.click(screen.getByTestId('set-classic'));
    expect(screen.getByTestId('design-theme').textContent).toBe('classic');
  });

  it('persists design theme to localStorage', async () => {
    const user = userEvent.setup();

    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    await user.click(screen.getByTestId('set-glass'));
    expect(localStorageMock.setItem).toHaveBeenCalledWith('parkhub_design_theme', 'glass');
  });

  it('reads initial design theme from localStorage', () => {
    localStorageMock.setItem('parkhub_design_theme', 'neon');

    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    expect(screen.getByTestId('design-theme').textContent).toBe('neon');
  });

  it('sets data-design-theme attribute on document element', async () => {
    const user = userEvent.setup();

    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    // Initial
    expect(document.documentElement.dataset.designTheme).toBe('classic');

    await user.click(screen.getByTestId('set-brutalist'));
    expect(document.documentElement.dataset.designTheme).toBe('brutalist');
  });

  it('falls back to classic for invalid localStorage design theme', () => {
    localStorageMock.setItem('parkhub_design_theme', 'invalid_theme');

    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    expect(screen.getByTestId('design-theme').textContent).toBe('classic');
  });

  it('each DESIGN_THEMES entry has required fields', () => {
    for (const theme of DESIGN_THEMES) {
      expect(theme.id).toBeTruthy();
      expect(theme.name).toBeTruthy();
      expect(theme.description).toBeTruthy();
      expect(theme.previewColors.light).toHaveLength(5);
      expect(theme.previewColors.dark).toHaveLength(5);
      expect(theme.tags.length).toBeGreaterThan(0);
    }
  });

  it('DESIGN_THEMES contains all 12 themes', () => {
    const ids = DESIGN_THEMES.map(t => t.id);
    expect(ids).toContain('classic');
    expect(ids).toContain('glass');
    expect(ids).toContain('bento');
    expect(ids).toContain('brutalist');
    expect(ids).toContain('neon');
    expect(ids).toContain('warm');
    expect(ids).toContain('liquid');
    expect(ids).toContain('mono');
    expect(ids).toContain('ocean');
    expect(ids).toContain('forest');
    expect(ids).toContain('synthwave');
    expect(ids).toContain('zen');
    expect(ids).toHaveLength(12);
  });

  it('can switch to all 12 design themes', async () => {
    const user = userEvent.setup();

    render(
      <ThemeProvider>
        <ThemeConsumer />
      </ThemeProvider>,
    );

    const allThemes: DesignThemeId[] = [
      'classic', 'glass', 'bento', 'brutalist', 'neon', 'warm',
      'liquid', 'mono', 'ocean', 'forest', 'synthwave', 'zen',
    ];

    for (const themeId of allThemes) {
      await user.click(screen.getByTestId(`set-${themeId}`));
      expect(screen.getByTestId('design-theme').textContent).toBe(themeId);
      expect(document.documentElement.dataset.designTheme).toBe(themeId);
    }
  });

  it('each new theme has valid preview colors', () => {
    const newThemes: DesignThemeId[] = ['liquid', 'mono', 'ocean', 'forest', 'synthwave', 'zen'];
    for (const id of newThemes) {
      const theme = DESIGN_THEMES.find(t => t.id === id);
      expect(theme).toBeDefined();
      expect(theme!.previewColors.light).toHaveLength(5);
      expect(theme!.previewColors.dark).toHaveLength(5);
      expect(theme!.tags.length).toBeGreaterThan(0);
      expect(theme!.description.length).toBeGreaterThan(10);
    }
  });
});
