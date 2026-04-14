import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Hoisted localStorage mock ──
const { localStorageMock } = vi.hoisted(() => {
  let store: Record<string, string> = {};
  const localStorageMock = {
    getItem: vi.fn((key: string) => store[key] ?? null),
    setItem: vi.fn((key: string, val: string) => { store[key] = val; }),
    removeItem: vi.fn((key: string) => { delete store[key]; }),
    clear: vi.fn(() => { store = {}; }),
  };

  Object.defineProperty(globalThis.window ?? globalThis, 'localStorage', {
    value: localStorageMock, writable: true, configurable: true,
  });

  return { localStorageMock };
});

import { UseCaseProvider, useUseCase, type UseCase } from './UseCaseContext';

// Helper component to consume the context
function UseCaseConsumer() {
  const { useCase, setUseCase, hasChosen } = useUseCase();
  return (
    <div>
      <span data-testid="usecase">{useCase}</span>
      <span data-testid="has-chosen">{String(hasChosen)}</span>
      <button data-testid="set-residential" onClick={() => setUseCase('residential')}>Residential</button>
      <button data-testid="set-personal" onClick={() => setUseCase('personal')}>Personal</button>
      <button data-testid="set-business" onClick={() => setUseCase('business')}>Business</button>
    </div>
  );
}

describe('UseCaseContext', () => {
  beforeEach(() => {
    localStorageMock.clear();
    delete document.documentElement.dataset.usecase;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('useUseCase throws outside UseCaseProvider', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(() => render(<UseCaseConsumer />)).toThrow(
      'useUseCase must be used within UseCaseProvider',
    );
    spy.mockRestore();
  });

  it('defaults to business when no localStorage value', () => {
    render(
      <UseCaseProvider>
        <UseCaseConsumer />
      </UseCaseProvider>,
    );
    expect(screen.getByTestId('usecase').textContent).toBe('business');
    expect(screen.getByTestId('has-chosen').textContent).toBe('false');
  });

  it('reads initial use case from localStorage', () => {
    localStorageMock.setItem('parkhub_usecase', 'residential');

    render(
      <UseCaseProvider>
        <UseCaseConsumer />
      </UseCaseProvider>,
    );

    expect(screen.getByTestId('usecase').textContent).toBe('residential');
    expect(screen.getByTestId('has-chosen').textContent).toBe('true');
  });

  it('setUseCase updates state and persists to localStorage', async () => {
    const user = userEvent.setup();

    render(
      <UseCaseProvider>
        <UseCaseConsumer />
      </UseCaseProvider>,
    );

    await user.click(screen.getByTestId('set-residential'));
    expect(screen.getByTestId('usecase').textContent).toBe('residential');
    expect(screen.getByTestId('has-chosen').textContent).toBe('true');
    expect(localStorageMock.setItem).toHaveBeenCalledWith('parkhub_usecase', 'residential');
  });

  it('sets data-usecase attribute on document element', async () => {
    const user = userEvent.setup();

    render(
      <UseCaseProvider>
        <UseCaseConsumer />
      </UseCaseProvider>,
    );

    // Initial value
    expect(document.documentElement.getAttribute('data-usecase')).toBe('business');

    await user.click(screen.getByTestId('set-personal'));
    expect(document.documentElement.getAttribute('data-usecase')).toBe('personal');
  });

  it('cycles through all use cases', async () => {
    const user = userEvent.setup();

    render(
      <UseCaseProvider>
        <UseCaseConsumer />
      </UseCaseProvider>,
    );

    const cases: UseCase[] = ['residential', 'personal', 'business'];
    for (const uc of cases) {
      await user.click(screen.getByTestId(`set-${uc}`));
      expect(screen.getByTestId('usecase').textContent).toBe(uc);
    }
  });

  it('hasChosen stays true once a selection is made', async () => {
    const user = userEvent.setup();

    render(
      <UseCaseProvider>
        <UseCaseConsumer />
      </UseCaseProvider>,
    );

    expect(screen.getByTestId('has-chosen').textContent).toBe('false');

    await user.click(screen.getByTestId('set-business'));
    expect(screen.getByTestId('has-chosen').textContent).toBe('true');

    // Selecting again keeps it true
    await user.click(screen.getByTestId('set-personal'));
    expect(screen.getByTestId('has-chosen').textContent).toBe('true');
  });
});
