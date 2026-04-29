import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import pkg from '../../package.json' with { type: 'json' };

const repoRoot = resolve(process.cwd(), '..');

describe('app version drift', () => {
  it('parkhub-web/package.json matches root package.json', () => {
    const rootPkg = JSON.parse(readFileSync(`${repoRoot}/package.json`, 'utf8'));
    expect(pkg.version).toBe(rootPkg.version);
  });

  it('parkhub-web/package.json matches workspace Cargo.toml', () => {
    const cargo = readFileSync(`${repoRoot}/Cargo.toml`, 'utf8');
    const m = cargo.match(/^version\s*=\s*"([^"]+)"/m);
    expect(m?.[1]).toBe(pkg.version);
  });
});
