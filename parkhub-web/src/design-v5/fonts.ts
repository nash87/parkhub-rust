/**
 * Self-hosted v5 fonts — Inter (display/UI) + DM Mono (numbers/IDs/timestamps).
 * Self-hosted via @fontsource so the strict CSP (`font-src 'self' data:`)
 * doesn't have to be widened. Import this module exactly once from the
 * v5 app root to trigger font bundling without side-effects elsewhere.
 *
 * Perf: Inter via @fontsource-variable/inter ships a single variable-font
 * woff2 covering all weights (300–800) in ~60KB — ~5× smaller than loading
 * 6 separate weight files. Keeps LCP under the 3.5s Lighthouse budget.
 * DM Mono has no variable build upstream, so we ship only the weights
 * actually used: 400 for copy, 500 for emphasis.
 */
import '@fontsource-variable/inter';
import '@fontsource/dm-mono/400.css';
import '@fontsource/dm-mono/500.css';
