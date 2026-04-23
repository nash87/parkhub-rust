import { beforeEach, describe, expect, it } from 'vitest';
import { render, screen, act } from '@testing-library/react';
import { V5ThemeProvider, useV5Theme, V5_MODES, type V5Mode } from './ThemeProvider';

function Probe() {
  const { mode, setMode, isVoid, isDark } = useV5Theme();
  return (
    <div>
      <span data-testid="mode">{mode}</span>
      <span data-testid="isVoid">{String(isVoid)}</span>
      <span data-testid="isDark">{String(isDark)}</span>
      {V5_MODES.map((m) => (
        <button key={m} onClick={() => setMode(m)} data-testid={`set-${m}`}>
          {m}
        </button>
      ))}
    </div>
  );
}

describe('V5ThemeProvider', () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.removeAttribute('data-ph-mode');
  });

  it('defaults to marble_light when neither storage nor prefers-dark set', () => {
    render(
      <V5ThemeProvider>
        <Probe />
      </V5ThemeProvider>
    );
    expect(screen.getByTestId('mode').textContent).toBe('marble_light');
    expect(document.documentElement.getAttribute('data-ph-mode')).toBe('marble_light');
  });

  it('reads mode from localStorage on mount', () => {
    window.localStorage.setItem('ph-v5-mode', 'void');
    render(
      <V5ThemeProvider>
        <Probe />
      </V5ThemeProvider>
    );
    expect(screen.getByTestId('mode').textContent).toBe('void');
    expect(screen.getByTestId('isVoid').textContent).toBe('true');
    expect(screen.getByTestId('isDark').textContent).toBe('true');
  });

  it('ignores an unknown mode string in storage', () => {
    window.localStorage.setItem('ph-v5-mode', 'neon-explosion' as V5Mode);
    render(
      <V5ThemeProvider>
        <Probe />
      </V5ThemeProvider>
    );
    expect(['marble_light', 'marble_dark']).toContain(screen.getByTestId('mode').textContent);
  });

  it('setMode updates context, <html data-ph-mode>, and localStorage', () => {
    render(
      <V5ThemeProvider>
        <Probe />
      </V5ThemeProvider>
    );
    act(() => {
      screen.getByTestId('set-void').click();
    });
    expect(screen.getByTestId('mode').textContent).toBe('void');
    expect(document.documentElement.getAttribute('data-ph-mode')).toBe('void');
    expect(window.localStorage.getItem('ph-v5-mode')).toBe('void');
  });

  it('isDark is true for marble_dark and void but false for marble_light', () => {
    render(
      <V5ThemeProvider>
        <Probe />
      </V5ThemeProvider>
    );
    act(() => screen.getByTestId('set-marble_dark').click());
    expect(screen.getByTestId('isDark').textContent).toBe('true');
    expect(screen.getByTestId('isVoid').textContent).toBe('false');
    act(() => screen.getByTestId('set-marble_light').click());
    expect(screen.getByTestId('isDark').textContent).toBe('false');
  });
});
