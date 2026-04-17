import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string, opts?: Record<string, unknown>) => {
      const base = fallback ?? key;
      if (opts && typeof base === 'string') {
        return Object.entries(opts).reduce(
          (acc, [k, v]) => acc.replace(new RegExp(`{{\\s*${k}\\s*}}`, 'g'), String(v)),
          base,
        );
      }
      return base;
    },
    i18n: { exists: () => false },
  }),
}));

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
vi.mock('react-hot-toast', () => ({
  default: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

const getModuleConfigMock = vi.fn();
const patchModuleConfigMock = vi.fn();
vi.mock('../api/client', () => ({
  api: {
    getModuleConfig: (...args: unknown[]) => getModuleConfigMock(...args),
    patchModuleConfig: (...args: unknown[]) => patchModuleConfigMock(...args),
  },
}));

import { ConfigEditorModal } from './ConfigEditorModal';

const SAMPLE = {
  schema: {
    type: 'object' as const,
    properties: {
      mode: { type: 'string' as const, enum: ['basic', 'pro'] },
      active: { type: 'boolean' as const },
    },
    required: ['mode'],
  },
  values: { mode: 'basic', active: true },
};

describe('ConfigEditorModal', () => {
  beforeEach(() => {
    getModuleConfigMock.mockReset();
    patchModuleConfigMock.mockReset();
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
  });

  it('renders nothing when closed', () => {
    const onClose = vi.fn();
    render(<ConfigEditorModal moduleName="sample" isOpen={false} onClose={onClose} />);
    expect(screen.queryByTestId('config-modal')).toBeNull();
    expect(getModuleConfigMock).not.toHaveBeenCalled();
  });

  it('fetches and hydrates the form when opened', async () => {
    getModuleConfigMock.mockResolvedValue({ success: true, data: SAMPLE });
    const onClose = vi.fn();
    render(<ConfigEditorModal moduleName="sample" isOpen={true} onClose={onClose} />);

    await waitFor(() => {
      expect(screen.getByTestId('cfg-field-mode')).toBeInTheDocument();
    });
    expect(getModuleConfigMock).toHaveBeenCalledWith('sample');
    expect((screen.getByTestId('cfg-field-mode') as HTMLSelectElement).value).toBe('basic');
  });

  it('shows a load-error state when the fetch fails', async () => {
    getModuleConfigMock.mockResolvedValue({
      success: false,
      data: null,
      error: { code: 'NOT_FOUND', message: 'No such module' },
    });
    render(<ConfigEditorModal moduleName="sample" isOpen={true} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByTestId('config-modal-load-error')).toBeInTheDocument();
    });
  });

  it('saves values, closes, and shows a success toast on 200', async () => {
    getModuleConfigMock.mockResolvedValue({ success: true, data: SAMPLE });
    patchModuleConfigMock.mockResolvedValue({
      success: true,
      data: { ...SAMPLE, values: { mode: 'pro', active: true } },
    });
    const onClose = vi.fn();
    render(<ConfigEditorModal moduleName="sample" isOpen={true} onClose={onClose} />);

    await waitFor(() => expect(screen.getByTestId('cfg-field-mode')).toBeInTheDocument());

    fireEvent.change(screen.getByTestId('cfg-field-mode'), { target: { value: 'pro' } });
    fireEvent.click(screen.getByTestId('config-editor-save'));

    await waitFor(() => expect(patchModuleConfigMock).toHaveBeenCalledTimes(1));
    expect(patchModuleConfigMock.mock.calls[0][0]).toBe('sample');
    expect(patchModuleConfigMock.mock.calls[0][1]).toEqual({ mode: 'pro', active: true });

    await waitFor(() => expect(toastSuccessMock).toHaveBeenCalled());
    expect(onClose).toHaveBeenCalled();
  });

  it('surfaces 422 details as inline field errors without closing', async () => {
    getModuleConfigMock.mockResolvedValue({ success: true, data: SAMPLE });
    patchModuleConfigMock.mockResolvedValue({
      success: false,
      data: null,
      error: {
        code: 'CONFIG_VALIDATION_FAILED',
        message: 'validation failed',
        details: [{ field: 'mode', message: 'unknown mode' }],
      },
    });
    const onClose = vi.fn();
    render(<ConfigEditorModal moduleName="sample" isOpen={true} onClose={onClose} />);

    await waitFor(() => expect(screen.getByTestId('cfg-field-mode')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('config-editor-save'));

    await waitFor(() => {
      expect(screen.getByText('unknown mode')).toBeInTheDocument();
    });
    expect(onClose).not.toHaveBeenCalled();
    expect(toastSuccessMock).not.toHaveBeenCalled();
    expect(toastErrorMock).not.toHaveBeenCalled();
  });

  it('shows an error toast and keeps the modal open on 5xx/403', async () => {
    getModuleConfigMock.mockResolvedValue({ success: true, data: SAMPLE });
    patchModuleConfigMock.mockResolvedValue({
      success: false,
      data: null,
      error: { code: 'HTTP_500', message: 'boom' },
    });
    const onClose = vi.fn();
    render(<ConfigEditorModal moduleName="sample" isOpen={true} onClose={onClose} />);

    await waitFor(() => expect(screen.getByTestId('cfg-field-mode')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('config-editor-save'));

    await waitFor(() => expect(toastErrorMock).toHaveBeenCalled());
    expect(onClose).not.toHaveBeenCalled();
  });

  it('closes on Escape', async () => {
    getModuleConfigMock.mockResolvedValue({ success: true, data: SAMPLE });
    const onClose = vi.fn();
    render(<ConfigEditorModal moduleName="sample" isOpen={true} onClose={onClose} />);
    await waitFor(() => expect(screen.getByTestId('cfg-field-mode')).toBeInTheDocument());
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(onClose).toHaveBeenCalled();
  });
});
