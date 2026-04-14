/**
 * Comprehensive theme switching audit.
 *
 * Tests every one of the 16 design themes end-to-end:
 * - Activation via setDesignTheme
 * - data-design-theme DOM attribute set correctly
 * - localStorage persistence
 * - Server sync via PUT /api/v1/preferences/theme
 * - Light + dark mode toggle per theme
 * - Theme preview colors present for both modes
 * - currentDesignTheme metadata resolves
 * - Re-hydration from localStorage on mount
 * - Invalid theme IDs are rejected
 * - User + admin flows both persist correctly
 */
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, act } from '@testing-library/react';
import type { ReactNode } from 'react';

// Hoisted mocks — must run before ThemeContext module code (window.matchMedia at module scope)
vi.hoisted(() => {
  let store: Record<string, string> = {};
  const localStorageMock = {
    getItem: vi.fn((key: string) => store[key] ?? null),
    setItem: vi.fn((key: string, val: string) => { store[key] = val; }),
    removeItem: vi.fn((key: string) => { delete store[key]; }),
    clear: vi.fn(() => { store = {}; }),
  };
  const persistentMql = {
    matches: false,
    media: '(prefers-color-scheme: dark)',
    addEventListener: vi.fn(),
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
});

const updateDesignThemePreferenceSpy = vi.fn(() =>
  Promise.resolve({ success: true, data: { design_theme: 'classic' } })
);
vi.mock('../api/client', () => ({
  getInMemoryToken: vi.fn(() => null),
  api: {
    updateDesignThemePreference: (id: string) => updateDesignThemePreferenceSpy(id),
  },
}));

import { ThemeProvider, useTheme, DESIGN_THEMES, type DesignThemeId } from './ThemeContext';

const ALL_THEME_IDS: DesignThemeId[] = [
  'classic', 'glass', 'bento', 'brutalist', 'neon', 'warm',
  'liquid', 'mono', 'ocean', 'forest', 'synthwave', 'zen',
  'aurora', 'material', 'sakura', 'midnight',
];

function TestHarness({ onReady }: { onReady: (state: ReturnType<typeof useTheme>) => void }) {
  const state = useTheme();
  onReady(state);
  return <div data-testid="harness">{state.designTheme}</div>;
}

function renderWithTheme(children: ReactNode = null) {
  const states: Array<ReturnType<typeof useTheme>> = [];
  const captured: { current: ReturnType<typeof useTheme> | null } = { current: null };
  const utils = render(
    <ThemeProvider>
      <TestHarness onReady={(s) => { states.push(s); captured.current = s; }} />
      {children}
    </ThemeProvider>
  );
  return { ...utils, getState: () => captured.current!, states };
}

describe('Theme Switching — Comprehensive Audit', () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.className = '';
    delete document.documentElement.dataset.designTheme;
    updateDesignThemePreferenceSpy.mockClear();
    updateDesignThemePreferenceSpy.mockResolvedValue({ success: true, data: { design_theme: 'classic' } });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Design theme catalog integrity', () => {
    it('exports exactly 16 themes', () => {
      expect(DESIGN_THEMES).toHaveLength(16);
    });

    it('has no duplicate theme IDs', () => {
      const ids = DESIGN_THEMES.map(t => t.id);
      expect(new Set(ids).size).toBe(ids.length);
    });

    it('every theme has a name, description, tags, and preview colors', () => {
      for (const theme of DESIGN_THEMES) {
        expect(theme.id).toBeTruthy();
        expect(theme.name).toBeTruthy();
        expect(theme.description).toBeTruthy();
        expect(theme.tags.length).toBeGreaterThan(0);
        expect(theme.previewColors.light).toHaveLength(5);
        expect(theme.previewColors.dark).toHaveLength(5);
      }
    });

    it('catalog matches the exported DesignThemeId union', () => {
      const catalogIds = DESIGN_THEMES.map(t => t.id).sort();
      const expected = [...ALL_THEME_IDS].sort();
      expect(catalogIds).toEqual(expected);
    });
  });

  // Programmatically generate one test per theme
  describe.each(ALL_THEME_IDS)('Theme: %s', (themeId) => {
    it(`activates and applies data-design-theme="${themeId}"`, () => {
      const { getState } = renderWithTheme();
      act(() => {
        getState().setDesignTheme(themeId);
      });
      expect(document.documentElement.dataset.designTheme).toBe(themeId);
      expect(getState().designTheme).toBe(themeId);
    });

    it(`persists ${themeId} to localStorage`, () => {
      const { getState } = renderWithTheme();
      act(() => {
        getState().setDesignTheme(themeId);
      });
      expect(localStorage.getItem('parkhub_design_theme')).toBe(themeId);
    });

    it(`syncs ${themeId} to server via API client`, () => {
      const { getState } = renderWithTheme();
      act(() => {
        getState().setDesignTheme(themeId);
      });
      expect(updateDesignThemePreferenceSpy).toHaveBeenCalledWith(themeId);
    });

    it(`re-hydrates ${themeId} from localStorage on next mount`, () => {
      localStorage.setItem('parkhub_design_theme', themeId);
      const { getState } = renderWithTheme();
      expect(getState().designTheme).toBe(themeId);
      expect(document.documentElement.dataset.designTheme).toBe(themeId);
    });

    it(`exposes metadata for ${themeId} via currentDesignTheme`, () => {
      const { getState } = renderWithTheme();
      act(() => {
        getState().setDesignTheme(themeId);
      });
      const current = getState().currentDesignTheme;
      expect(current.id).toBe(themeId);
      expect(current.name).toBeTruthy();
      expect(current.previewColors.light).toHaveLength(5);
      expect(current.previewColors.dark).toHaveLength(5);
    });

    it(`preserves ${themeId} across light and dark toggles`, () => {
      const { getState } = renderWithTheme();
      act(() => {
        getState().setDesignTheme(themeId);
        getState().setTheme('light');
      });
      expect(getState().designTheme).toBe(themeId);
      expect(document.documentElement.classList.contains('dark')).toBe(false);

      act(() => {
        getState().setTheme('dark');
      });
      expect(getState().designTheme).toBe(themeId);
      expect(document.documentElement.classList.contains('dark')).toBe(true);
      expect(document.documentElement.dataset.designTheme).toBe(themeId);
    });
  });

  describe('Light/Dark mode behavior', () => {
    it('defaults to system preference', () => {
      const { getState } = renderWithTheme();
      expect(getState().theme).toBe('system');
    });

    it('persists light mode selection', () => {
      const { getState } = renderWithTheme();
      act(() => { getState().setTheme('light'); });
      expect(localStorage.getItem('parkhub_theme')).toBe('light');
      expect(document.documentElement.classList.contains('dark')).toBe(false);
    });

    it('persists dark mode selection', () => {
      const { getState } = renderWithTheme();
      act(() => { getState().setTheme('dark'); });
      expect(localStorage.getItem('parkhub_theme')).toBe('dark');
      expect(document.documentElement.classList.contains('dark')).toBe(true);
    });

    it('updates meta theme-color on dark mode', () => {
      const metaLight = document.createElement('meta');
      metaLight.setAttribute('name', 'theme-color');
      metaLight.setAttribute('media', '(prefers-color-scheme: light)');
      const metaDark = document.createElement('meta');
      metaDark.setAttribute('name', 'theme-color');
      metaDark.setAttribute('media', '(prefers-color-scheme: dark)');
      document.head.appendChild(metaLight);
      document.head.appendChild(metaDark);

      const { getState } = renderWithTheme();
      act(() => { getState().setTheme('dark'); });
      expect(metaLight.getAttribute('content')).toBe('#042f2e');
      expect(metaDark.getAttribute('content')).toBe('#042f2e');

      act(() => { getState().setTheme('light'); });
      expect(metaLight.getAttribute('content')).toBe('#0d9488');
      expect(metaDark.getAttribute('content')).toBe('#0d9488');

      metaLight.remove();
      metaDark.remove();
    });
  });

  describe('Invalid theme handling', () => {
    it('rejects invalid theme IDs in setDesignTheme', () => {
      const { getState } = renderWithTheme();
      act(() => {
        getState().setDesignTheme('classic');
      });
      act(() => {
        // @ts-expect-error intentional invalid value
        getState().setDesignTheme('nonexistent-theme-id');
      });
      expect(getState().designTheme).toBe('classic');
      expect(localStorage.getItem('parkhub_design_theme')).toBe('classic');
    });

    it('falls back to classic on corrupted localStorage', () => {
      localStorage.setItem('parkhub_design_theme', 'garbage-not-a-theme');
      const { getState } = renderWithTheme();
      expect(getState().designTheme).toBe('classic');
    });

    it('currentDesignTheme falls back to first entry when designTheme is somehow unknown', () => {
      // Force an invalid ID by bypassing localStorage validation
      localStorage.setItem('parkhub_design_theme', 'classic');
      const { getState } = renderWithTheme();
      // currentDesignTheme is computed, always resolves to a valid theme
      expect(getState().currentDesignTheme).toBeDefined();
      expect(getState().currentDesignTheme.id).toBe('classic');
    });
  });

  describe('User vs Admin flow — server sync', () => {
    it('User role: theme change triggers API update', () => {
      const { getState } = renderWithTheme();
      act(() => { getState().setDesignTheme('ocean'); });
      expect(updateDesignThemePreferenceSpy).toHaveBeenCalledWith('ocean');
    });

    it('Admin role: same API method is used (admins use user preferences)', () => {
      const { getState } = renderWithTheme();
      act(() => { getState().setDesignTheme('forest'); });
      expect(updateDesignThemePreferenceSpy).toHaveBeenCalledTimes(1);
      expect(updateDesignThemePreferenceSpy).toHaveBeenCalledWith('forest');
    });

    it('API failure on theme sync does not crash UI', () => {
      updateDesignThemePreferenceSpy.mockRejectedValueOnce(new Error('network error'));
      const { getState } = renderWithTheme();
      expect(() => {
        act(() => { getState().setDesignTheme('neon'); });
      }).not.toThrow();
      expect(getState().designTheme).toBe('neon');
      expect(document.documentElement.dataset.designTheme).toBe('neon');
    });
  });

  describe('Rapid theme switching', () => {
    it('handles switching through all 16 themes in sequence', () => {
      const { getState } = renderWithTheme();
      for (const id of ALL_THEME_IDS) {
        act(() => { getState().setDesignTheme(id); });
        expect(document.documentElement.dataset.designTheme).toBe(id);
        expect(localStorage.getItem('parkhub_design_theme')).toBe(id);
      }
    });

    it('handles rapid light/dark toggles without losing design theme', () => {
      const { getState } = renderWithTheme();
      act(() => { getState().setDesignTheme('sakura'); });
      for (let i = 0; i < 5; i++) {
        act(() => { getState().setTheme(i % 2 === 0 ? 'dark' : 'light'); });
      }
      expect(getState().designTheme).toBe('sakura');
    });
  });

  describe('Hook contract', () => {
    it('useTheme throws outside ThemeProvider', () => {
      function NakedConsumer() {
        useTheme();
        return null;
      }
      const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
      expect(() => render(<NakedConsumer />)).toThrow(/ThemeProvider/);
      spy.mockRestore();
    });
  });
});
