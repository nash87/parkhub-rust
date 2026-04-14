import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { renderHook } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

import { usePageTitle } from './usePageTitle';

const wrapper = (initialPath: string) => ({ children }: { children: React.ReactNode }) => (
  <MemoryRouter initialEntries={[initialPath]}>{children}</MemoryRouter>
);

describe('usePageTitle', () => {
  beforeEach(() => {
    document.title = '';
  });

  afterEach(() => {
    document.title = '';
  });

  it('sets title for known root route', () => {
    renderHook(() => usePageTitle(), { wrapper: wrapper('/') });
    expect(document.title).toBe('nav.dashboard — ParkHub');
  });

  it('sets title for nested admin route via parent fallback', () => {
    renderHook(() => usePageTitle(), { wrapper: wrapper('/admin/users') });
    expect(document.title).toBe('admin.title — ParkHub');
  });

  it('sets title for /book route', () => {
    renderHook(() => usePageTitle(), { wrapper: wrapper('/book') });
    expect(document.title).toBe('book.title — ParkHub');
  });

  it('falls back to ParkHub for unknown route', () => {
    renderHook(() => usePageTitle(), { wrapper: wrapper('/totally-unknown-route') });
    expect(document.title).toBe('ParkHub');
  });
});
