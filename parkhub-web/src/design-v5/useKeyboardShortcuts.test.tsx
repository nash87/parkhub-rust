import { describe, it, expect, vi, afterEach } from 'vitest';
import { render, cleanup } from '@testing-library/react';
import { useKeyboardShortcuts } from './useKeyboardShortcuts';

function fireKey(key: string, target?: EventTarget, init?: KeyboardEventInit): boolean {
  const ev = new KeyboardEvent('keydown', { key, bubbles: true, ...init });
  if (target) {
    target.dispatchEvent(ev);
  } else {
    document.dispatchEvent(ev);
  }
  return !ev.defaultPrevented;
}

function Harness({ shortcuts, enabled = true }: { shortcuts: Record<string, (e: KeyboardEvent) => void>; enabled?: boolean }) {
  useKeyboardShortcuts(shortcuts, enabled);
  return <input data-testid="form-input" />;
}

describe('useKeyboardShortcuts', () => {
  afterEach(() => cleanup());

  it('fires the handler for a registered key on document', () => {
    const nHandler = vi.fn();
    render(<Harness shortcuts={{ n: nHandler }} />);
    fireKey('n');
    expect(nHandler).toHaveBeenCalledTimes(1);
  });

  it('is case-insensitive for single-letter shortcuts', () => {
    const nHandler = vi.fn();
    render(<Harness shortcuts={{ n: nHandler }} />);
    fireKey('N');
    expect(nHandler).toHaveBeenCalledTimes(1);
  });

  it('does not fire when typing in an input element', () => {
    const nHandler = vi.fn();
    const { getByTestId } = render(<Harness shortcuts={{ n: nHandler }} />);
    const input = getByTestId('form-input');
    input.focus();
    fireKey('n', input);
    expect(nHandler).not.toHaveBeenCalled();
  });

  it('ignores modifier combos (Ctrl+K stays with the app)', () => {
    const slashHandler = vi.fn();
    render(<Harness shortcuts={{ '/': slashHandler }} />);
    fireKey('/', undefined, { ctrlKey: true });
    expect(slashHandler).not.toHaveBeenCalled();
  });

  it('fires Escape shortcut from any non-editable target', () => {
    const escHandler = vi.fn();
    render(<Harness shortcuts={{ Escape: escHandler }} />);
    fireKey('Escape');
    expect(escHandler).toHaveBeenCalledTimes(1);
  });

  it('is disabled when enabled=false', () => {
    const nHandler = vi.fn();
    render(<Harness shortcuts={{ n: nHandler }} enabled={false} />);
    fireKey('n');
    expect(nHandler).not.toHaveBeenCalled();
  });
});
