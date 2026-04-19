// App version exposed to the UI. Sourced from the Vite env variable
// VITE_APP_VERSION (wired in astro.config.mjs from package.json) with a
// placeholder fallback so dev builds without the env don't break.
//
// Previously this lived hardcoded in Layout.tsx ("v4.9.0") and drifted —
// v4.13.0 and later shipped with a stale footer. One-line source so it
// stays accurate forever.
export const APP_VERSION: string =
  (typeof import.meta !== 'undefined' && (import.meta as { env?: { VITE_APP_VERSION?: string } }).env?.VITE_APP_VERSION)
  || 'dev';
