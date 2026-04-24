import { describe, expect, it } from 'vitest';
import fs from 'node:fs';
import path from 'node:path';

/**
 * All 26 navigation entries ship as real v5 screens — the PlaceholderV5
 * fallback component and its import are dead code. This guard keeps it
 * that way so we don't silently reintroduce a placeholder path.
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
});
