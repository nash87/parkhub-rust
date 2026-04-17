/**
 * useModuleToggle — reusable hook that calls
 * `PATCH /api/v1/admin/modules/{name}` with `{ runtime_enabled }` and
 * exposes the in-flight + error state so any caller can render their own UI.
 *
 * The backend returns the updated ModuleInfo on 200; on 400/409/500 it
 * returns the ApiResponse envelope with an error, and the hook surfaces
 * the error code/message so the caller can revert optimistic UI + toast.
 *
 * Shipped for T-1720 v2 frontend.
 */
import { useCallback, useState } from 'react';
import { api, type ModuleInfo } from '../api/client';

export interface ModuleToggleResult {
  ok: boolean;
  module?: ModuleInfo;
  errorCode?: string;
  errorMessage?: string;
}

export interface UseModuleToggle {
  /** True while a PATCH is in-flight for this module. */
  inFlight: boolean;
  /** Last error code (e.g. HTTP_409, NETWORK). Null when no error. */
  error: string | null;
  /** Fire the toggle. Resolves with the server result; never throws. */
  toggle: (runtimeEnabled: boolean) => Promise<ModuleToggleResult>;
  /** Clear the last error — useful when the caller has shown the toast. */
  clearError: () => void;
}

export function useModuleToggle(name: string): UseModuleToggle {
  const [inFlight, setInFlight] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const toggle = useCallback(
    async (runtimeEnabled: boolean): Promise<ModuleToggleResult> => {
      setInFlight(true);
      setError(null);
      try {
        const res = await api.patchModule(name, runtimeEnabled);
        if (res.success && res.data) {
          return { ok: true, module: res.data };
        }
        const code = res.error?.code ?? 'UNKNOWN';
        const message = res.error?.message ?? 'Unknown error';
        setError(code);
        return { ok: false, errorCode: code, errorMessage: message };
      } finally {
        setInFlight(false);
      }
    },
    [name],
  );

  const clearError = useCallback(() => setError(null), []);

  return { inFlight, error, toggle, clearError };
}
