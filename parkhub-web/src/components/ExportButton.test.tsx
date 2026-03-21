import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { ExportButton, type ExportType } from './ExportButton';

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (_: string, f: string) => f }) }));
beforeEach(() => { vi.restoreAllMocks(); localStorage.clear(); });

describe('ExportButton', () => {
  it('renders toggle', () => { render(<ExportButton />); expect(screen.getByTestId('export-toggle')).toBeDefined(); });
  it('dropdown hidden by default', () => { render(<ExportButton />); expect(screen.queryByTestId('export-dropdown')).toBeNull(); });
  it('opens on click', () => { render(<ExportButton />); fireEvent.click(screen.getByTestId('export-toggle')); expect(screen.getByTestId('export-dropdown')).toBeDefined(); });
  it('closes on second click', () => {
    render(<ExportButton />); const b = screen.getByTestId('export-toggle');
    fireEvent.click(b); expect(screen.getByTestId('export-dropdown')).toBeDefined();
    fireEvent.click(b); expect(screen.queryByTestId('export-dropdown')).toBeNull();
  });
  it('shows all export options', () => {
    render(<ExportButton />); fireEvent.click(screen.getByTestId('export-toggle'));
    expect(screen.getByTestId('export-bookings')).toBeDefined();
    expect(screen.getByTestId('export-users')).toBeDefined();
    expect(screen.getByTestId('export-revenue')).toBeDefined();
  });
  it('shows date inputs', () => {
    render(<ExportButton />); fireEvent.click(screen.getByTestId('export-toggle'));
    expect(screen.getByTestId('export-from')).toBeDefined();
    expect(screen.getByTestId('export-to')).toBeDefined();
  });
  it('date defaults', () => {
    render(<ExportButton />); fireEvent.click(screen.getByTestId('export-toggle'));
    const to = screen.getByTestId('export-to') as HTMLInputElement;
    expect(to.value).toBe(new Date().toISOString().slice(0, 10));
  });
  it('date change', () => {
    render(<ExportButton />); fireEvent.click(screen.getByTestId('export-toggle'));
    const f = screen.getByTestId('export-from') as HTMLInputElement;
    fireEvent.change(f, { target: { value: '2026-01-15' } }); expect(f.value).toBe('2026-01-15');
  });
});

describe('ExportButton download', () => {
  it('fetches with auth', async () => {
    localStorage.setItem('parkhub_token', 'tok');
    const fm = vi.fn().mockResolvedValue({ ok: true, blob: () => Promise.resolve(new Blob([''])) });
    vi.stubGlobal('fetch', fm);
    vi.stubGlobal('URL', { ...URL, createObjectURL: vi.fn().mockReturnValue('b:x'), revokeObjectURL: vi.fn() });
    render(<ExportButton />); fireEvent.click(screen.getByTestId('export-toggle')); fireEvent.click(screen.getByTestId('export-bookings'));
    await waitFor(() => expect(fm).toHaveBeenCalledTimes(1));
    expect(fm.mock.calls[0][0]).toContain('/api/v1/admin/export/bookings');
    expect(fm.mock.calls[0][1].headers.Authorization).toBe('Bearer tok');
  });
  it('correct URL per type', async () => {
    const fm = vi.fn().mockResolvedValue({ ok: true, blob: () => Promise.resolve(new Blob([''])) });
    vi.stubGlobal('fetch', fm); vi.stubGlobal('URL', { ...URL, createObjectURL: vi.fn().mockReturnValue('b:x'), revokeObjectURL: vi.fn() });
    for (const t of ['bookings', 'users', 'revenue']) {
      fm.mockClear(); const { unmount } = render(<ExportButton />);
      fireEvent.click(screen.getByTestId('export-toggle')); fireEvent.click(screen.getByTestId(`export-${t}`));
      await waitFor(() => expect(fm).toHaveBeenCalledTimes(1));
      expect(fm.mock.calls[0][0]).toContain(`/api/v1/admin/export/${t}`); unmount();
    }
  });
  it('handles error', async () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false, status: 403, text: () => Promise.resolve('Forbidden') }));
    render(<ExportButton />); fireEvent.click(screen.getByTestId('export-toggle')); fireEvent.click(screen.getByTestId('export-users'));
    await waitFor(() => expect(spy).toHaveBeenCalled()); spy.mockRestore();
  });
  it('no token', async () => {
    const fm = vi.fn().mockResolvedValue({ ok: true, blob: () => Promise.resolve(new Blob([''])) });
    vi.stubGlobal('fetch', fm); vi.stubGlobal('URL', { ...URL, createObjectURL: vi.fn().mockReturnValue('b:x'), revokeObjectURL: vi.fn() });
    render(<ExportButton />); fireEvent.click(screen.getByTestId('export-toggle')); fireEvent.click(screen.getByTestId('export-revenue'));
    await waitFor(() => expect(fm).toHaveBeenCalledTimes(1)); expect(fm.mock.calls[0][1].headers).toEqual({});
  });
  it('custom baseUrl', async () => {
    const fm = vi.fn().mockResolvedValue({ ok: true, blob: () => Promise.resolve(new Blob([''])) });
    vi.stubGlobal('fetch', fm); vi.stubGlobal('URL', { ...URL, createObjectURL: vi.fn().mockReturnValue('b:x'), revokeObjectURL: vi.fn() });
    render(<ExportButton baseUrl="https://api.test.com" />); fireEvent.click(screen.getByTestId('export-toggle')); fireEvent.click(screen.getByTestId('export-bookings'));
    await waitFor(() => expect(fm).toHaveBeenCalledTimes(1)); expect(fm.mock.calls[0][0]).toMatch(/^https:\/\/api\.test\.com/);
  });
});
