/**
 * ConfigEditor — generic JSON Schema form renderer (T-1720 v3).
 *
 * Renders a form for any module whose backend ships a `config_schema`
 * object. Purposefully small scope: handles exactly the field shapes
 * the 5 v3 modules expose today, no more:
 *
 *   - string with `enum`              → <select>
 *   - string with `format: "email"`   → <input type="email">
 *   - string with `format: "time"`    → <input type="time">
 *   - string (with optional maxLength)→ <input type="text">
 *   - boolean                          → accessible switch
 *   - integer with min/max             → <input type="number">
 *
 * Client-side validation is a shape check only — the server is the
 * source of truth and returns `422 { error: 'CONFIG_VALIDATION_FAILED',
 * details: [{ field, message }] }` which is surfaced inline via the
 * `fieldErrors` prop.
 *
 * Zero runtime deps — no react-jsonschema-form — because our schema
 * shape is tightly bounded. Expand the renderer when a new schema
 * shape actually ships, not before.
 */

import {
  type FormEvent,
  type ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';

export type JsonSchemaProperty =
  | {
      type: 'string';
      enum?: string[];
      format?: 'email' | 'time' | string;
      maxLength?: number;
      title?: string;
      description?: string;
      default?: string;
    }
  | {
      type: 'boolean';
      title?: string;
      description?: string;
      default?: boolean;
    }
  | {
      type: 'integer';
      minimum?: number;
      maximum?: number;
      title?: string;
      description?: string;
      default?: number;
    };

export interface JsonSchema {
  type: 'object';
  properties: Record<string, JsonSchemaProperty>;
  required?: string[];
  title?: string;
  description?: string;
}

/** One server-side validation failure for a single field. */
export interface FieldError {
  field: string;
  message: string;
}

export interface ConfigEditorProps {
  /** JSON Schema returned by the backend (shape: {type:'object', properties}). */
  schema: JsonSchema;
  /** Current values keyed by property name. */
  values: Record<string, unknown>;
  /** Module name — used to derive i18n field keys. */
  moduleName: string;
  /** Save handler — resolves when PATCH returns 2xx; rejects on error. */
  onSave: (next: Record<string, unknown>) => Promise<void>;
  /** Optional cancel — when omitted, no cancel button is rendered. */
  onCancel?: () => void;
  /** Render a busy state while `onSave` is in-flight. */
  busy?: boolean;
  /** Form-level error (e.g. toast copy for 5xx/403). */
  error?: string;
  /** Per-field server-side validation errors from the last 422 response. */
  fieldErrors?: FieldError[];
}

function humanize(key: string): string {
  return key
    .replace(/_/g, ' ')
    .replace(/\b\w/g, (c) => c.toUpperCase())
    .trim();
}

function coerceValue(prop: JsonSchemaProperty, raw: unknown): unknown {
  if (prop.type === 'integer') {
    if (raw === '' || raw == null) return undefined;
    const n = Number(raw);
    return Number.isFinite(n) ? Math.trunc(n) : raw;
  }
  if (prop.type === 'boolean') {
    return Boolean(raw);
  }
  return raw;
}

function validateShape(
  schema: JsonSchema,
  values: Record<string, unknown>,
): FieldError[] {
  const out: FieldError[] = [];
  const required = schema.required ?? [];
  for (const [name, prop] of Object.entries(schema.properties)) {
    const v = values[name];
    const missing =
      v === undefined ||
      v === null ||
      (typeof v === 'string' && v === '');
    if (missing) {
      if (required.includes(name)) {
        out.push({ field: name, message: 'required' });
      }
      continue;
    }
    if (prop.type === 'integer') {
      if (typeof v !== 'number' || !Number.isFinite(v)) {
        out.push({ field: name, message: 'must be a number' });
        continue;
      }
      if (prop.minimum !== undefined && v < prop.minimum) {
        out.push({ field: name, message: `>= ${prop.minimum}` });
      }
      if (prop.maximum !== undefined && v > prop.maximum) {
        out.push({ field: name, message: `<= ${prop.maximum}` });
      }
    } else if (prop.type === 'string') {
      if (typeof v !== 'string') {
        out.push({ field: name, message: 'must be text' });
        continue;
      }
      if (prop.enum && !prop.enum.includes(v)) {
        out.push({ field: name, message: 'invalid option' });
      }
      if (prop.maxLength !== undefined && v.length > prop.maxLength) {
        out.push({ field: name, message: `<= ${prop.maxLength} chars` });
      }
    } else if (prop.type === 'boolean') {
      if (typeof v !== 'boolean') {
        out.push({ field: name, message: 'must be true/false' });
      }
    }
  }
  return out;
}

/**
 * Generic JSON Schema config editor form. Pure component — no module
 * awareness leaks in beyond the optional `moduleName` used for i18n
 * key derivation. Safe for any module shipping the supported shapes.
 */
export function ConfigEditor({
  schema,
  values,
  moduleName,
  onSave,
  onCancel,
  busy = false,
  error,
  fieldErrors,
}: ConfigEditorProps) {
  const { t, i18n } = useTranslation();
  const [local, setLocal] = useState<Record<string, unknown>>(() => ({ ...values }));
  const [localErrors, setLocalErrors] = useState<FieldError[]>([]);
  const firstFieldRef = useRef<HTMLElement | null>(null);

  // Reset local state when the incoming values change (modal reopened for a
  // different module, or server echoed back new values after a successful save).
  useEffect(() => {
    setLocal({ ...values });
    setLocalErrors([]);
  }, [values]);

  // Focus the first field on mount for a11y.
  useEffect(() => {
    firstFieldRef.current?.focus();
  }, []);

  const mergedErrors: Record<string, string> = useMemo(() => {
    const m: Record<string, string> = {};
    for (const e of localErrors) m[e.field] = e.message;
    for (const e of fieldErrors ?? []) m[e.field] = e.message; // server wins over client
    return m;
  }, [localErrors, fieldErrors]);

  const setField = useCallback((name: string, prop: JsonSchemaProperty, raw: unknown) => {
    setLocal((prev) => ({ ...prev, [name]: coerceValue(prop, raw) }));
    setLocalErrors((prev) => prev.filter((e) => e.field !== name));
  }, []);

  const onSubmit = async (e: FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    const errs = validateShape(schema, local);
    if (errs.length > 0) {
      setLocalErrors(errs);
      return;
    }
    await onSave(local);
  };

  function labelFor(name: string, prop: JsonSchemaProperty): string {
    const key = `admin.modules.config.${moduleName}.${name}`;
    // Some test harnesses mock useTranslation() without the i18n instance;
    // feature-detect before calling exists() so we don't explode.
    if (i18n && typeof i18n.exists === 'function' && i18n.exists(key)) return t(key);
    return prop.title ?? humanize(name);
  }

  const required = schema.required ?? [];

  return (
    <form onSubmit={onSubmit} noValidate className="space-y-4">
      {error && (
        <div
          role="alert"
          className="rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-sm text-red-600 dark:text-red-400"
          data-testid="config-editor-error"
        >
          {error}
        </div>
      )}

      {Object.entries(schema.properties).map(([name, prop], idx) => {
        const isRequired = required.includes(name);
        const fieldId = `cfg-${moduleName}-${name}`;
        const descId = `${fieldId}-desc`;
        const errId = `${fieldId}-err`;
        const errMsg = mergedErrors[name];
        const label = labelFor(name, prop);
        const description = prop.description;
        const value = local[name];

        const commonAriaProps: {
          id: string;
          'aria-invalid': boolean;
          'aria-describedby'?: string;
          'aria-required'?: boolean;
        } = {
          id: fieldId,
          'aria-invalid': !!errMsg,
        };
        const describedBy = [description ? descId : null, errMsg ? errId : null]
          .filter(Boolean)
          .join(' ');
        if (describedBy) commonAriaProps['aria-describedby'] = describedBy;
        if (isRequired) commonAriaProps['aria-required'] = true;

        const inputRefProp = idx === 0 ? { ref: firstFieldRef as React.RefObject<HTMLElement> } : {};

        let control: ReactNode;

        if (prop.type === 'boolean') {
          const checked = !!value;
          control = (
            <button
              type="button"
              role="switch"
              aria-checked={checked}
              {...commonAriaProps}
              {...inputRefProp}
              disabled={busy}
              onClick={() => setField(name, prop, !checked)}
              data-testid={`cfg-field-${name}`}
              className={`relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 dark:focus:ring-offset-surface-900 ${
                checked ? 'bg-primary-600' : 'bg-surface-300 dark:bg-surface-600'
              } ${busy ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
            >
              <span
                className={`inline-block h-3.5 w-3.5 transform rounded-full bg-white shadow-sm transition-transform ${
                  checked ? 'translate-x-5' : 'translate-x-0.5'
                }`}
              />
            </button>
          );
        } else if (prop.type === 'string' && prop.enum) {
          control = (
            <select
              {...commonAriaProps}
              {...inputRefProp}
              disabled={busy}
              value={(value as string | undefined) ?? ''}
              onChange={(e) => setField(name, prop, e.target.value)}
              data-testid={`cfg-field-${name}`}
              className="w-full rounded-lg border border-surface-200 dark:border-surface-700 bg-transparent px-3 py-2 text-sm outline-none focus:border-primary-400 disabled:opacity-50"
            >
              {!isRequired && <option value="">—</option>}
              {prop.enum.map((o) => (
                <option key={o} value={o}>
                  {o}
                </option>
              ))}
            </select>
          );
        } else if (prop.type === 'string' && prop.format === 'email') {
          control = (
            <input
              type="email"
              {...commonAriaProps}
              {...inputRefProp}
              disabled={busy}
              value={(value as string | undefined) ?? ''}
              onChange={(e) => setField(name, prop, e.target.value)}
              maxLength={prop.maxLength}
              data-testid={`cfg-field-${name}`}
              className="w-full rounded-lg border border-surface-200 dark:border-surface-700 bg-transparent px-3 py-2 text-sm outline-none focus:border-primary-400 disabled:opacity-50"
            />
          );
        } else if (prop.type === 'string' && prop.format === 'time') {
          control = (
            <input
              type="time"
              {...commonAriaProps}
              {...inputRefProp}
              disabled={busy}
              value={(value as string | undefined) ?? ''}
              onChange={(e) => setField(name, prop, e.target.value)}
              data-testid={`cfg-field-${name}`}
              className="w-full rounded-lg border border-surface-200 dark:border-surface-700 bg-transparent px-3 py-2 text-sm outline-none focus:border-primary-400 disabled:opacity-50"
            />
          );
        } else if (prop.type === 'string') {
          control = (
            <input
              type="text"
              {...commonAriaProps}
              {...inputRefProp}
              disabled={busy}
              value={(value as string | undefined) ?? ''}
              onChange={(e) => setField(name, prop, e.target.value)}
              maxLength={prop.maxLength}
              data-testid={`cfg-field-${name}`}
              className="w-full rounded-lg border border-surface-200 dark:border-surface-700 bg-transparent px-3 py-2 text-sm outline-none focus:border-primary-400 disabled:opacity-50"
            />
          );
        } else if (prop.type === 'integer') {
          control = (
            <input
              type="number"
              step={1}
              {...commonAriaProps}
              {...inputRefProp}
              disabled={busy}
              value={value === undefined || value === null ? '' : String(value)}
              onChange={(e) => setField(name, prop, e.target.value)}
              min={prop.minimum}
              max={prop.maximum}
              data-testid={`cfg-field-${name}`}
              className="w-full rounded-lg border border-surface-200 dark:border-surface-700 bg-transparent px-3 py-2 text-sm outline-none focus:border-primary-400 disabled:opacity-50"
            />
          );
        } else {
          control = null;
        }

        return (
          <div key={name} className="space-y-1">
            <label htmlFor={fieldId} className="flex items-center gap-1 text-sm font-medium">
              <span>{label}</span>
              {isRequired && (
                <span
                  aria-label={t('admin.modules.config.required', 'Required')}
                  title={t('admin.modules.config.required', 'Required')}
                  className="text-red-500"
                >
                  *
                </span>
              )}
            </label>
            {control}
            {description && (
              <p id={descId} className="text-xs text-surface-500 dark:text-surface-400">
                {description}
              </p>
            )}
            {errMsg && (
              <p id={errId} role="alert" className="text-xs text-red-600 dark:text-red-400">
                {errMsg}
              </p>
            )}
          </div>
        );
      })}

      <div className="flex items-center justify-end gap-3 pt-2">
        {onCancel && (
          <button
            type="button"
            onClick={onCancel}
            disabled={busy}
            className="rounded-lg border border-surface-200 dark:border-surface-700 bg-transparent px-4 py-2 text-sm hover:bg-surface-100 dark:hover:bg-surface-800 disabled:opacity-50"
            data-testid="config-editor-cancel"
          >
            {t('admin.modules.config.cancel', 'Cancel')}
          </button>
        )}
        <button
          type="submit"
          disabled={busy}
          className="rounded-lg bg-primary-600 px-4 py-2 text-sm font-medium text-white hover:bg-primary-700 disabled:opacity-50"
          data-testid="config-editor-save"
        >
          {busy
            ? t('admin.modules.config.saving', 'Saving…')
            : t('admin.modules.config.save', 'Save')}
        </button>
      </div>
    </form>
  );
}
