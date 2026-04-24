import { describe, expect, it } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';

/**
 * Static guards over `App.tsx`.
 *
 *  - All 26 navigation entries ship as real v5 screens; PlaceholderV5 is
 *    retired (#383) so its import + file must not come back.
 *  - URL-based deep-linking + View Transitions + keyboard-shortcut hook
 *    must stay wired into the shell (Tier-1 UX #386).
 */
describe('design-v5/App', () => {
  const appSrc = fs.readFileSync(
    path.resolve(__dirname, './App.tsx'),
    'utf8',
  );

  it('does not import PlaceholderV5', () => {
    expect(appSrc).not.toMatch(/PlaceholderV5/);
  });

  it('has no Placeholder screen file on disk', () => {
    const placeholderPath = path.resolve(__dirname, './screens/Placeholder.tsx');
    expect(fs.existsSync(placeholderPath)).toBe(false);
  });

  it('imports startViewTransition for cross-screen fades', () => {
    expect(appSrc).toMatch(/from '\.\/viewTransitions'/);
    expect(appSrc).toMatch(/startViewTransition\(/);
  });

  it('wires deep-link hooks so /v5/<id> round-trips', () => {
    expect(appSrc).toMatch(/from '\.\/useDeepLink'/);
    expect(appSrc).toMatch(/useSyncScreenToUrl/);
    expect(appSrc).toMatch(/readScreenFromUrl/);
  });

  it('wires the shared keyboard-shortcut hook', () => {
    expect(appSrc).toMatch(/from '\.\/useKeyboardShortcuts'/);
    expect(appSrc).toMatch(/useKeyboardShortcuts\(/);
  });
});
