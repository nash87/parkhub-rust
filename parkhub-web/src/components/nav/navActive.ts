/**
 * Segment-aware active-path check shared by every nav variant
 * (Classic, Rail, TopTabs, FloatingDock).
 *
 * `pathname.startsWith(to)` misclassifies sibling paths that share a
 * prefix — e.g. `/book` vs `/bookings` would both be marked active when
 * the user visits `/bookings`. This helper only accepts the match when
 * the next character is a segment boundary ('/') or a query ('?'), or
 * when the whole path equals `to`.
 *
 * Exported as a standalone module so it can be unit-tested and reused
 * without pulling in any React dependency.
 */
export function isActivePath(pathname: string, to: string): boolean {
  if (pathname === to) return true;
  if (to === '/') return false;
  if (pathname.startsWith(to + '/')) return true;
  if (pathname.startsWith(to + '?')) return true;
  return false;
}
