import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import {
  DEFAULT_SETTINGS,
  STORAGE_KEY,
  migrate,
  readStoredSettings,
  writeStoredSettings,
  type UserSettings,
} from './settings';

/**
 * Surface contract for the v5 settings layer.
 *
 *   - `settings`     — current resolved settings (migrate-coerced, never null)
 *   - `updateSection(section, patch)` — partial update to a top-level group
 *   - `updateSetting(section, key, value)` — single-key update (most common)
 *   - `resetSettings()` — back to factory defaults
 *   - `hydrateRemote(remote)` — apply a payload pulled from /api/v1/me/settings
 *
 * The provider owns persistence: every change writes localStorage + flips
 * the dataset attribute on `<html>` so density / font / contrast tokens
 * cascade. Backend sync is **debounced** so toggling 5 things in a row
 * results in one PUT, not five.
 */

export interface V5SettingsCtx {
  settings: UserSettings;
  updateSection: <K extends keyof UserSettings>(section: K, patch: Partial<UserSettings[K]>) => void;
  updateSetting: <K extends keyof UserSettings, P extends keyof UserSettings[K]>(
    section: K,
    key: P,
    value: UserSettings[K][P],
  ) => void;
  resetSettings: () => void;
  hydrateRemote: (remote: unknown) => void;
  /** Last known sync state — UI can show "saved", "saving…", "error". */
  syncState: V5SettingsSyncState;
}

export type V5SettingsSyncState = 'idle' | 'saving' | 'saved' | 'error';

const Ctx = createContext<V5SettingsCtx | null>(null);

interface V5SettingsProviderProps {
  children: ReactNode;
  /** Optional sync hook — called debounced after settings change. */
  syncToServer?: (settings: UserSettings) => Promise<unknown>;
  /** Debounce window for backend sync, ms. Defaults to 600. */
  syncDebounceMs?: number;
  /** Optional override for initial settings (testing). */
  initialOverride?: UserSettings;
}

function applyDocumentAttributes(settings: UserSettings) {
  if (typeof document === 'undefined') return;
  const root = document.documentElement;
  root.setAttribute('data-ph-mode', settings.appearance.mode);
  root.setAttribute('data-ph-density', settings.appearance.density);
  root.setAttribute('data-ph-font', settings.appearance.font);
  root.setAttribute(
    'data-ph-reduced-motion',
    settings.appearance.reducedMotion ? 'true' : 'false',
  );
  root.setAttribute(
    'data-ph-high-contrast',
    settings.appearance.highContrast ? 'true' : 'false',
  );
  root.style.setProperty('--v5-font-scale', String(settings.appearance.fontScale));
}

export function V5SettingsProvider({
  children,
  syncToServer,
  syncDebounceMs = 600,
  initialOverride,
}: V5SettingsProviderProps) {
  const [settings, setSettings] = useState<UserSettings>(
    () => initialOverride ?? readStoredSettings(),
  );
  const [syncState, setSyncState] = useState<V5SettingsSyncState>('idle');
  const syncTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const syncSeq = useRef(0); // stale-callback guard
  // Race guard: don't push the initial (default/local) state to the server
  // before either remote hydration or an explicit user action. Otherwise a
  // fresh browser would overwrite previously saved server settings with
  // defaults on first paint. Flips true once user-action setters or
  // `hydrateRemote` run.
  const syncArmed = useRef(false);

  // Apply document attributes immediately on every change so the UI
  // reflects the new tokens without an extra render pass.
  useEffect(() => {
    applyDocumentAttributes(settings);
    writeStoredSettings(settings);
  }, [settings]);

  // Debounced backend sync (disabled when no syncToServer provided).
  useEffect(() => {
    if (!syncToServer) return;
    // Skip sync until we have either hydrated from the server or seen a
    // user-driven change — see `syncArmed` above.
    if (!syncArmed.current) return;
    if (syncTimer.current) clearTimeout(syncTimer.current);
    syncTimer.current = setTimeout(() => {
      const seq = ++syncSeq.current;
      setSyncState('saving');
      syncToServer(settings)
        .then(() => {
          if (seq === syncSeq.current) setSyncState('saved');
        })
        .catch(() => {
          if (seq === syncSeq.current) setSyncState('error');
        });
    }, syncDebounceMs);
    return () => {
      if (syncTimer.current) clearTimeout(syncTimer.current);
    };
  }, [settings, syncToServer, syncDebounceMs]);

  // Cross-tab sync: another tab writes localStorage → we pick it up.
  useEffect(() => {
    if (typeof window === 'undefined') return;
    const handler = (e: StorageEvent) => {
      if (e.key !== STORAGE_KEY || e.newValue == null) return;
      try {
        setSettings(migrate(JSON.parse(e.newValue)));
      } catch {
        /* noop */
      }
    };
    window.addEventListener('storage', handler);
    return () => window.removeEventListener('storage', handler);
  }, []);

  const updateSection = useCallback(
    <K extends keyof UserSettings>(section: K, patch: Partial<UserSettings[K]>) => {
      syncArmed.current = true;
      setSettings((prev) => ({
        ...prev,
        [section]: { ...prev[section], ...patch },
      }));
    },
    [],
  );

  const updateSetting = useCallback(
    <K extends keyof UserSettings, P extends keyof UserSettings[K]>(
      section: K,
      key: P,
      value: UserSettings[K][P],
    ) => {
      syncArmed.current = true;
      setSettings((prev) => ({
        ...prev,
        [section]: { ...prev[section], [key]: value },
      }));
    },
    [],
  );

  const resetSettings = useCallback(() => {
    syncArmed.current = true;
    setSettings({ ...DEFAULT_SETTINGS });
  }, []);

  const hydrateRemote = useCallback((remote: unknown) => {
    if (remote == null) return;
    // Server is canonical; arm the sync loop so subsequent local changes
    // get persisted. The hydrate itself does not need to round-trip back.
    syncArmed.current = true;
    setSettings((prev) => {
      const incoming = migrate(remote);
      // Don't overwrite locally-changed appearance with stale server data
      // if the user just changed something this session — but for the
      // initial hydrate (when state still matches DEFAULT or stored), the
      // server is canonical. We accept the server value unconditionally
      // because the debounce cycle keeps server in sync.
      void prev;
      return incoming;
    });
  }, []);

  const value = useMemo<V5SettingsCtx>(
    () => ({ settings, updateSection, updateSetting, resetSettings, hydrateRemote, syncState }),
    [settings, updateSection, updateSetting, resetSettings, hydrateRemote, syncState],
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}

export function useV5Settings(): V5SettingsCtx {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error('useV5Settings must be used within <V5SettingsProvider>');
  return ctx;
}

/**
 * Non-throwing variant — returns null when used outside the provider.
 * Use sparingly; this is meant for opt-in widgets (e.g. variant-switching
 * sidebar) that need to render even when no provider exists.
 */
export function useV5SettingsOptional(): V5SettingsCtx | null {
  return useContext(Ctx);
}

/**
 * Convenience: subscribe to a single feature flag. Returns `false` when
 * the provider is unavailable so the caller can be rendered outside the
 * v5 shell without throwing.
 */
export function useV5Feature(key: keyof UserSettings['features']): boolean {
  const ctx = useContext(Ctx);
  if (!ctx) return DEFAULT_SETTINGS.features[key];
  return ctx.settings.features[key];
}
