/**
 * ConfigEditorModal — fetches a module's JSON Schema config + current
 * values from `/api/v1/admin/modules/{name}/config`, renders them via
 * ConfigEditor, and writes back on save (T-1720 v3).
 *
 * Error contract (matches both parkhub-rust and parkhub-php backends):
 *   - 200 OK                      → close modal, toast success
 *   - 422 CONFIG_VALIDATION_FAILED → surface per-field errors inline, keep open
 *   - 403 / 404 / 5xx / network   → toast error, keep modal open
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';
import { api } from '../api/client';
import {
  ConfigEditor,
  type FieldError,
  type JsonSchema,
} from './ConfigEditor';

export interface ConfigEditorModalProps {
  moduleName: string;
  isOpen: boolean;
  onClose: () => void;
}

interface LoadedConfig {
  schema: JsonSchema;
  values: Record<string, unknown>;
}

export function ConfigEditorModal({ moduleName, isOpen, onClose }: ConfigEditorModalProps) {
  const { t } = useTranslation();
  const [loaded, setLoaded] = useState<LoadedConfig | null>(null);
  const [loadErr, setLoadErr] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [fieldErrors, setFieldErrors] = useState<FieldError[] | undefined>(undefined);
  const [formError, setFormError] = useState<string | undefined>(undefined);
  const dialogRef = useRef<HTMLDivElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);
  const previouslyFocused = useRef<HTMLElement | null>(null);

  // Fetch config whenever the modal opens for a (potentially different) module.
  useEffect(() => {
    if (!isOpen) return;
    let active = true;
    setLoaded(null);
    setLoadErr(null);
    setFieldErrors(undefined);
    setFormError(undefined);

    (async () => {
      const res = await api.getModuleConfig(moduleName);
      if (!active) return;
      if (res.success && res.data) {
        setLoaded({ schema: res.data.schema, values: res.data.values });
      } else {
        setLoadErr(res.error?.message ?? 'Failed to load config');
      }
    })();

    return () => {
      active = false;
    };
  }, [isOpen, moduleName]);

  // Focus management: remember the previously-focused element on open, restore
  // on close. Close the modal on Escape. Trap Tab inside the dialog while open.
  useEffect(() => {
    if (!isOpen) return;
    previouslyFocused.current = (document.activeElement as HTMLElement) ?? null;
    // Focus the close button as an initial anchor; ConfigEditor itself moves
    // focus to the first field once schema has loaded.
    closeButtonRef.current?.focus();

    function onKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onClose();
        return;
      }
      if (e.key === 'Tab' && dialogRef.current) {
        const focusables = dialogRef.current.querySelectorAll<HTMLElement>(
          'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
        );
        if (focusables.length === 0) return;
        const first = focusables[0];
        const last = focusables[focusables.length - 1];
        const active = document.activeElement as HTMLElement | null;
        if (e.shiftKey && active === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && active === last) {
          e.preventDefault();
          first.focus();
        }
      }
    }
    document.addEventListener('keydown', onKeyDown);
    return () => {
      document.removeEventListener('keydown', onKeyDown);
      previouslyFocused.current?.focus?.();
    };
  }, [isOpen, onClose]);

  const onSave = useCallback(
    async (next: Record<string, unknown>) => {
      setBusy(true);
      setFieldErrors(undefined);
      setFormError(undefined);
      try {
        const res = await api.patchModuleConfig(moduleName, next);
        if (res.success && res.data) {
          toast.success(
            t('admin.modules.config.successToast', '{{name}} config saved', {
              name: moduleName,
            }),
          );
          setLoaded({ schema: res.data.schema, values: res.data.values });
          onClose();
          return;
        }
        const code = res.error?.code;
        if (code === 'CONFIG_VALIDATION_FAILED' || code === 'HTTP_422') {
          const details = (res.error?.details as FieldError[] | undefined) ?? [];
          setFieldErrors(details);
          setFormError(
            t(
              'admin.modules.config.validationFailed',
              'Please fix the highlighted fields',
            ),
          );
          return;
        }
        toast.error(
          t('admin.modules.config.errorToast', 'Could not save {{name}} config', {
            name: moduleName,
          }),
        );
      } finally {
        setBusy(false);
      }
    },
    [moduleName, t, onClose],
  );

  if (!isOpen) return null;

  return (
    <>
      <div
        className="fixed inset-0 z-[60] bg-black/40"
        onClick={onClose}
        aria-hidden="true"
        data-testid="config-modal-overlay"
      />
      <div
        className="fixed inset-0 z-[61] flex items-center justify-center p-4"
        role="dialog"
        aria-modal="true"
        aria-labelledby="config-modal-title"
        data-testid="config-modal"
      >
        <div
          ref={dialogRef}
          className="w-full max-w-lg rounded-xl border border-surface-200/50 dark:border-surface-700/50 bg-white dark:bg-surface-900 p-6 shadow-xl"
          onClick={(e) => e.stopPropagation()}
        >
          <div className="flex items-start justify-between gap-4 mb-4">
            <h2 id="config-modal-title" className="text-lg font-semibold">
              {t('admin.modules.config.modalTitle', 'Configure {{name}}', {
                name: moduleName,
              })}
            </h2>
            <button
              ref={closeButtonRef}
              type="button"
              onClick={onClose}
              aria-label={t('admin.modules.config.cancel', 'Cancel')}
              className="rounded p-1 text-surface-500 hover:bg-surface-100 dark:hover:bg-surface-800"
              data-testid="config-modal-close"
            >
              ×
            </button>
          </div>

          {loaded === null && loadErr === null && (
            <p className="text-sm text-surface-500" data-testid="config-modal-loading">
              {t('loading', 'Loading…')}
            </p>
          )}

          {loadErr && (
            <div role="alert" className="text-sm text-red-500" data-testid="config-modal-load-error">
              {loadErr}
            </div>
          )}

          {loaded && Object.keys(loaded.schema.properties ?? {}).length === 0 && (
            <p className="text-sm text-surface-500" data-testid="config-modal-no-schema">
              {t('admin.modules.config.noSchema', 'No configuration schema')}
            </p>
          )}

          {loaded && Object.keys(loaded.schema.properties ?? {}).length > 0 && (
            <ConfigEditor
              schema={loaded.schema}
              values={loaded.values}
              moduleName={moduleName}
              onSave={onSave}
              onCancel={onClose}
              busy={busy}
              error={formError}
              fieldErrors={fieldErrors}
            />
          )}
        </div>
      </div>
    </>
  );
}
