import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, cleanup, act } from '@testing-library/react';
import { readScreenFromUrl, writeScreenToUrl, useSyncScreenToUrl } from './useDeepLink';
import type { ScreenId } from './nav';

function setPath(path: string): void {
  window.history.replaceState({}, '', path);
}

describe('readScreenFromUrl', () => {
  beforeEach(() => setPath('/v5'));
  afterEach(() => setPath('/'));

  it('returns fallback when the URL has no screen segment', () => {
    expect(readScreenFromUrl('dashboard' as ScreenId)).toBe('dashboard');
  });

  it('parses a valid screen id from the pathname', () => {
    setPath('/v5/buchungen');
    expect(readScreenFromUrl('dashboard' as ScreenId)).toBe('buchungen');
  });

  it('ignores unknown ids and falls back', () => {
    setPath('/v5/not-a-real-screen');
    expect(readScreenFromUrl('dashboard' as ScreenId)).toBe('dashboard');
  });

  it('tolerates a trailing slash', () => {
    setPath('/v5/fahrzeuge/');
    expect(readScreenFromUrl('dashboard' as ScreenId)).toBe('fahrzeuge');
  });
});

describe('writeScreenToUrl', () => {
  afterEach(() => setPath('/'));

  it('pushes a new /v5/<id> entry when the screen changes', () => {
    setPath('/v5/dashboard');
    writeScreenToUrl('buchen' as ScreenId);
    expect(window.location.pathname).toBe('/v5/buchen');
  });

  it('is a no-op when the URL already matches the screen', () => {
    setPath('/v5/buchen');
    const pushSpy = vi.spyOn(window.history, 'pushState');
    writeScreenToUrl('buchen' as ScreenId);
    expect(pushSpy).not.toHaveBeenCalled();
    pushSpy.mockRestore();
  });
});

function Harness({ screen, onPopState }: { screen: ScreenId; onPopState: (s: ScreenId) => void }) {
  useSyncScreenToUrl(screen, onPopState);
  return null;
}

describe('useSyncScreenToUrl', () => {
  afterEach(() => {
    setPath('/');
    cleanup();
  });

  it('writes the initial screen to the URL on mount', () => {
    setPath('/v5');
    render(<Harness screen={'buchen' as ScreenId} onPopState={vi.fn()} />);
    expect(window.location.pathname).toBe('/v5/buchen');
  });

  it('notifies on popstate with the new screen', () => {
    setPath('/v5/buchen');
    const onPopState = vi.fn();
    render(<Harness screen={'buchen' as ScreenId} onPopState={onPopState} />);
    setPath('/v5/fahrzeuge');
    act(() => {
      window.dispatchEvent(new PopStateEvent('popstate'));
    });
    expect(onPopState).toHaveBeenCalledWith('fahrzeuge');
  });
});
