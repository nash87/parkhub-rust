import { defineConfig } from 'vitest/config';
import { playwright } from '@vitest/browser-playwright';
import react from '@astrojs/react';

/**
 * Vitest 4 browser-mode config — runs React component tests in a real browser
 * (Playwright/Chromium provider) instead of jsdom. Recommended over jsdom for
 * any code that touches DOM APIs, layout, animations, or React 19 concurrent
 * features that depend on real reactive primitives.
 *
 * Run: `npx vitest run --config vitest.config.browser.ts`
 *
 * NOTE: this is a SEPARATE config from vitest.config.ts so unit tests stay fast
 * (no browser spin-up). Browser tests live in src/**\/*.browser.spec.{ts,tsx}.
 *
 * Component-binding shim is `vitest-browser-react` (NOT -svelte) — picks up the
 * React 19 transformer from `@astrojs/react` so JSX in *.browser.spec.tsx works
 * the same way as the regular `npm test` suite.
 */
export default defineConfig({
	plugins: [react()],
	test: {
		include: ['src/**/*.browser.spec.{ts,tsx}'],
		exclude: ['e2e/**', 'node_modules/**', 'dist/**', '.astro/**'],
		passWithNoTests: true,
		globals: true,
		setupFiles: ['./src/test/msw/vitest-setup.ts'],
		browser: {
			enabled: true,
			provider: playwright(),
			headless: true,
			instances: [
				{
					browser: 'chromium',
				},
			],
		},
	},
});
