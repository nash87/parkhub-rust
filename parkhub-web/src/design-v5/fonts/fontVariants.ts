/**
 * v5 font variants — five families, lazy-loaded only when the user
 * actually picks them.
 *
 *   - inter      (default, already loaded by ./fonts.ts)
 *   - dmmono     (already loaded; primarily mono usage)
 *   - system     (no download — use OS UI font stack)
 *   - plex       (IBM Plex Sans, lazy-loaded when chosen)
 *   - atkinson   (Atkinson Hyperlegible — a11y-optimised, lazy-loaded)
 *
 * Lazy-loading uses `import()` with `?url` so Astro/Vite splits each
 * variant into its own chunk. We track loaded variants so flipping back
 * doesn't repeat the download. Failures degrade silently — the CSS
 * variable already falls back to the system stack.
 */
import type { V5FontVariant } from '../settings/settings';

interface FontDescriptor {
  /** CSS font-family value applied via --v5-font-family. */
  fontFamily: string;
  /** Lazy loader — undefined means "no download needed". */
  load?: () => Promise<unknown>;
}

const SYSTEM_STACK = "system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif";

export const FONT_VARIANTS: Record<V5FontVariant, FontDescriptor> = {
  inter: {
    fontFamily: `'Inter Variable', 'Inter', ${SYSTEM_STACK}`,
    // Bundled by fonts.ts on initial load — no lazy needed.
  },
  dmmono: {
    fontFamily: `'DM Mono', ui-monospace, 'SFMono-Regular', Menlo, Consolas, monospace`,
    // Bundled by fonts.ts on initial load.
  },
  system: {
    fontFamily: SYSTEM_STACK,
    // Native — no download.
  },
  plex: {
    fontFamily: `'IBM Plex Sans', ${SYSTEM_STACK}`,
    load: () => import('@fontsource/ibm-plex-sans/400.css').catch(() => undefined),
  },
  atkinson: {
    fontFamily: `'Atkinson Hyperlegible', ${SYSTEM_STACK}`,
    load: () =>
      import('@fontsource/atkinson-hyperlegible/400.css').catch(() => undefined),
  },
};

const loaded = new Set<V5FontVariant>(['inter', 'dmmono', 'system']);

/**
 * Apply a font variant — lazy-load if needed, then set the CSS variable.
 * Resolves once the font CSS is loaded (or immediately for already-loaded
 * variants). Errors are swallowed so a network failure can't crash the
 * settings UI.
 */
export async function applyFontVariant(variant: V5FontVariant): Promise<void> {
  const descriptor = FONT_VARIANTS[variant];
  if (!descriptor) return;

  if (!loaded.has(variant) && descriptor.load) {
    try {
      await descriptor.load();
      loaded.add(variant);
    } catch {
      /* fall through — CSS variable still applies, browser falls back to system stack */
    }
  }

  if (typeof document !== 'undefined') {
    document.documentElement.style.setProperty('--v5-font-family', descriptor.fontFamily);
  }
}

/** Test-only — reset the loaded set so each test starts cold. */
export function __resetFontLoaderForTests() {
  loaded.clear();
  loaded.add('inter');
  loaded.add('dmmono');
  loaded.add('system');
}

export const V5_FONT_LABELS: Record<V5FontVariant, string> = {
  inter: 'Inter (Standard)',
  dmmono: 'DM Mono',
  system: 'System',
  plex: 'IBM Plex',
  atkinson: 'Atkinson Hyperlegible',
};
