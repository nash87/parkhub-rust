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

import { ConfigEditor, type JsonSchema } from './ConfigEditor';

const SAMPLE_SCHEMA: JsonSchema = {
  type: 'object',
  properties: {
    mode: { type: 'string', enum: ['basic', 'pro'], title: 'Mode' },
    notify_email: { type: 'string', format: 'email', title: 'Notify Email' },
    opens_at: { type: 'string', format: 'time', title: 'Opens at' },
    label: { type: 'string', maxLength: 16, title: 'Label' },
    active: { type: 'boolean', title: 'Active' },
    max_slots: { type: 'integer', minimum: 1, maximum: 10, title: 'Max slots' },
  },
  required: ['mode', 'max_slots'],
};

function renderEditor(overrides?: Partial<Parameters<typeof ConfigEditor>[0]>) {
  const onSave = vi.fn().mockResolvedValue(undefined);
  const onCancel = vi.fn();
  const defaults = {
    schema: SAMPLE_SCHEMA,
    values: {
      mode: 'basic',
      notify_email: 'ops@example.com',
      opens_at: '08:00',
      label: 'main',
      active: true,
      max_slots: 5,
    } as Record<string, unknown>,
    moduleName: 'sample',
    onSave,
    onCancel,
  };
  const utils = render(<ConfigEditor {...defaults} {...overrides} />);
  return { ...utils, onSave, onCancel };
}

describe('ConfigEditor', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders every supported field type with the right input', () => {
    renderEditor();
    expect(screen.getByTestId('cfg-field-mode').tagName).toBe('SELECT');
    expect(screen.getByTestId('cfg-field-notify_email').getAttribute('type')).toBe('email');
    expect(screen.getByTestId('cfg-field-opens_at').getAttribute('type')).toBe('time');
    expect(screen.getByTestId('cfg-field-label').getAttribute('type')).toBe('text');
    expect(screen.getByTestId('cfg-field-active').getAttribute('role')).toBe('switch');
    expect(screen.getByTestId('cfg-field-max_slots').getAttribute('type')).toBe('number');
  });

  it('marks required fields with a star and associates aria-required', () => {
    renderEditor();
    const mode = screen.getByTestId('cfg-field-mode');
    const max = screen.getByTestId('cfg-field-max_slots');
    const label = screen.getByTestId('cfg-field-label');
    expect(mode.getAttribute('aria-required')).toBe('true');
    expect(max.getAttribute('aria-required')).toBe('true');
    expect(label.getAttribute('aria-required')).toBeNull();
    // Two stars on the two required fields
    const stars = screen.getAllByText('*');
    expect(stars.length).toBe(2);
  });

  it('submits coerced values on save', async () => {
    const { onSave } = renderEditor();
    // Flip the boolean
    fireEvent.click(screen.getByTestId('cfg-field-active'));
    // Change the integer
    fireEvent.change(screen.getByTestId('cfg-field-max_slots'), { target: { value: '7' } });
    // Change the text
    fireEvent.change(screen.getByTestId('cfg-field-label'), { target: { value: 'other' } });

    fireEvent.click(screen.getByTestId('config-editor-save'));

    await waitFor(() => expect(onSave).toHaveBeenCalledTimes(1));
    const arg = onSave.mock.calls[0][0];
    expect(arg.active).toBe(false);
    expect(arg.max_slots).toBe(7);
    expect(arg.label).toBe('other');
    // Unchanged fields still round-trip
    expect(arg.mode).toBe('basic');
    expect(arg.notify_email).toBe('ops@example.com');
  });

  it('blocks submit and shows inline errors when a required field is missing', async () => {
    const { onSave } = renderEditor({
      values: {
        mode: '',
        notify_email: '',
        opens_at: '',
        label: '',
        active: false,
        max_slots: undefined,
      },
    });
    fireEvent.click(screen.getByTestId('config-editor-save'));

    // onSave must NOT have been called — client-side shape check fails
    await waitFor(() => {
      const errs = screen.getAllByRole('alert');
      expect(errs.length).toBeGreaterThan(0);
    });
    expect(onSave).not.toHaveBeenCalled();
    expect(screen.getByTestId('cfg-field-mode').getAttribute('aria-invalid')).toBe('true');
    expect(screen.getByTestId('cfg-field-max_slots').getAttribute('aria-invalid')).toBe('true');
  });

  it('surfaces server-side 422 fieldErrors inline', () => {
    renderEditor({
      fieldErrors: [{ field: 'notify_email', message: 'not a valid domain' }],
      error: 'Please fix the highlighted fields',
    });
    expect(screen.getByTestId('config-editor-error').textContent).toContain(
      'Please fix the highlighted fields',
    );
    expect(screen.getByText('not a valid domain')).toBeInTheDocument();
    expect(
      screen.getByTestId('cfg-field-notify_email').getAttribute('aria-invalid'),
    ).toBe('true');
  });

  it('calls onCancel when the cancel button is clicked', () => {
    const { onCancel } = renderEditor();
    fireEvent.click(screen.getByTestId('config-editor-cancel'));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it('respects busy state by disabling controls and showing saving copy', () => {
    renderEditor({ busy: true });
    expect(screen.getByTestId('config-editor-save').textContent).toMatch(/Saving/i);
    expect((screen.getByTestId('cfg-field-max_slots') as HTMLInputElement).disabled).toBe(true);
    expect((screen.getByTestId('cfg-field-active') as HTMLButtonElement).disabled).toBe(true);
  });

  it('enforces integer min/max inline', async () => {
    const { onSave } = renderEditor();
    fireEvent.change(screen.getByTestId('cfg-field-max_slots'), { target: { value: '42' } });
    fireEvent.click(screen.getByTestId('config-editor-save'));
    await waitFor(() => {
      expect(
        screen.getByTestId('cfg-field-max_slots').getAttribute('aria-invalid'),
      ).toBe('true');
    });
    expect(onSave).not.toHaveBeenCalled();
  });
});
