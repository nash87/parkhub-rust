export {
  DEFAULT_SETTINGS,
  SETTINGS_VERSION,
  STORAGE_KEY,
  V5_APPEARANCE_MODES,
  V5_DENSITIES,
  V5_FONT_SCALES,
  V5_FONT_VARIANTS,
  V5_SIDEBAR_VARIANTS,
  migrate,
  readStoredSettings,
  writeStoredSettings,
} from './settings';
export type {
  UserSettings,
  V5AppearanceMode,
  V5Density,
  V5FontScale,
  V5FontVariant,
  V5SidebarVariant,
} from './settings';
export {
  V5SettingsProvider,
  useV5Feature,
  useV5Settings,
  useV5SettingsOptional,
} from './SettingsProvider';
export type { V5SettingsCtx, V5SettingsSyncState } from './SettingsProvider';
