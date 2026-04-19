import { test as base, expect } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

// Playwright fixture that exposes an AxeBuilder pre-configured with WCAG 2.1 AA
// rules. Use it in any spec: `test('…', async ({ axe }) => { await axe(); })`.
//
// The fixture runs axe against the currently-loaded page, asserts zero
// critical/serious violations, and writes a JSON report to
// `test-results/axe/<test>.json` for post-run inspection.
export const test = base.extend<{
  axe: (opts?: { include?: string; exclude?: string[] }) => Promise<void>;
}>({
  axe: async ({ page }, use, testInfo) => {
    const run = async (opts?: { include?: string; exclude?: string[] }) => {
      let builder = new AxeBuilder({ page })
        .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'best-practice']);
      if (opts?.include) builder = builder.include(opts.include);
      if (opts?.exclude) for (const sel of opts.exclude) builder = builder.exclude(sel);

      const results = await builder.analyze();
      const serious = results.violations.filter(v => v.impact === 'serious' || v.impact === 'critical');

      await testInfo.attach('axe-report.json', {
        body: JSON.stringify(results, null, 2),
        contentType: 'application/json',
      });

      expect(serious, `axe-core found ${serious.length} serious/critical a11y violations:\n` +
        serious.map(v => `  - ${v.id}: ${v.help} (${v.nodes.length} node${v.nodes.length === 1 ? '' : 's'})`).join('\n')
      ).toHaveLength(0);
    };
    await use(run);
  },
});

export { expect };
