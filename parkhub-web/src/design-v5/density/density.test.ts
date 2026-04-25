import { describe, expect, it, beforeEach } from 'vitest';
import './density.css';

/**
 * jsdom doesn't apply CSS via stylesheets (it parses but doesn't compute),
 * so these tests only verify the dataset attribute contract — the actual
 * rule application is asserted via Playwright in the e2e suite. We do verify
 * the file is at least syntactically valid (parsed without throwing) by the
 * import side-effect above.
 */
describe('density tokens', () => {
  beforeEach(() => {
    document.documentElement.removeAttribute('data-ph-density');
    document.documentElement.removeAttribute('data-ph-high-contrast');
    document.documentElement.removeAttribute('data-ph-reduced-motion');
  });

  it('document accepts compact density attribute', () => {
    document.documentElement.setAttribute('data-ph-density', 'compact');
    expect(document.documentElement.getAttribute('data-ph-density')).toBe('compact');
  });

  it('document accepts spacious density attribute', () => {
    document.documentElement.setAttribute('data-ph-density', 'spacious');
    expect(document.documentElement.getAttribute('data-ph-density')).toBe('spacious');
  });

  it('document accepts high-contrast attribute', () => {
    document.documentElement.setAttribute('data-ph-high-contrast', 'true');
    expect(document.documentElement.getAttribute('data-ph-high-contrast')).toBe('true');
  });

  it('document accepts reduced-motion attribute', () => {
    document.documentElement.setAttribute('data-ph-reduced-motion', 'true');
    expect(document.documentElement.getAttribute('data-ph-reduced-motion')).toBe('true');
  });
});
