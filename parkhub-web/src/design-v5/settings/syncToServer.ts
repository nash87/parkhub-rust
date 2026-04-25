import { api } from '../../api/client';
import type { UserSettings } from './settings';

/**
 * Push the user's v5 settings blob to the server. Throws when the API
 * response is unsuccessful so the SettingsProvider's `syncState` reflects
 * the failure (rather than masking 401/404/413/etc as `saved`).
 *
 * Network errors propagate as rejections from `api.updateSettings`
 * directly. The thrown `Error` for `success: false` carries the server's
 * machine-readable code/message when available so callers can surface a
 * useful toast.
 */
export async function syncSettingsToServer(settings: UserSettings): Promise<unknown> {
  const res = await api.updateSettings(settings as unknown as Record<string, unknown>);
  if (!res.success) {
    const code = res.error?.code ?? 'UNKNOWN';
    const message = res.error?.message ?? 'settings update failed';
    throw new Error(`${code}: ${message}`);
  }
  return res;
}
