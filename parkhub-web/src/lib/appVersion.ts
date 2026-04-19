// App version exposed to the UI. Sourced from the Vite env variable
// VITE_APP_VERSION (wired in astro.config.mjs from the workspace Cargo.toml)
// with a package.json fallback so tests + plain Vite builds — which don't
// see the astro define — still resolve a real number instead of 'dev'.
//
// Previously this lived hardcoded in Layout.tsx ("v4.9.0") and drifted —
// v4.13.0 and later shipped with a stale footer. One-line source so it
// stays accurate forever.
// @ts-ignore — Vite resolves JSON imports at build time
import { version as pkgVersion } from '../../package.json';

export const APP_VERSION: string =
  (typeof import.meta !== 'undefined' && (import.meta as { env?: { VITE_APP_VERSION?: string } }).env?.VITE_APP_VERSION)
  || pkgVersion
  || 'dev';
