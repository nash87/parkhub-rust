import { describe, it, expect } from 'vitest';
import { isActivePath } from './navActive';

describe('isActivePath', () => {
  it('matches exact paths', () => {
    expect(isActivePath('/bookings', '/bookings')).toBe(true);
  });

  it('does not match sibling paths that share a prefix', () => {
    // Regression for Codex P2 #349: `/book` should not be active when
    // the user is on `/bookings`.
    expect(isActivePath('/bookings', '/book')).toBe(false);
  });

  it('matches deeper segments under the same root', () => {
    expect(isActivePath('/bookings/123', '/bookings')).toBe(true);
    expect(isActivePath('/admin/users', '/admin')).toBe(true);
  });

  it('matches paths with query strings', () => {
    expect(isActivePath('/bookings?filter=active', '/bookings')).toBe(true);
  });

  it('only matches root for root paths', () => {
    expect(isActivePath('/', '/')).toBe(true);
    expect(isActivePath('/anywhere', '/')).toBe(false);
  });
});
