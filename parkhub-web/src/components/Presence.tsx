/**
 * Presence indicator — stacked avatar circles showing who else is viewing
 * the current surface. Ported from the claude.ai/design v3 handoff bundle
 * (qol.jsx Presence).
 *
 * The design's hardcoded list is flipped to props so callers can hand in
 * real presence data (e.g. from a future WebSocket subscription on the
 * user's org), fall back to a sample for demos, and vary the overflow
 * count. Status dot (green) is rendered inside each avatar to signal
 * "currently active".
 *
 * Deliberately not auto-wired into Layout: the API contract for presence
 * data doesn't exist yet in parkhub-server / Laravel backends, so callers
 * opt in explicitly when they have a source to plug in.
 */

interface PresenceUser {
  /** Display name, e.g. "Lena K." */
  name: string;
  /** Hex or CSS color for the avatar gradient */
  color: string;
  /** Optional subtitle rendered into the native tooltip, e.g. "L2-08" */
  subtitle?: string;
}

interface PresenceProps {
  users: PresenceUser[];
  /** Show "+N" chip after the avatars when additional viewers exist */
  moreCount?: number;
  /** Max avatars to render inline before the overflow chip kicks in */
  maxShown?: number;
}

function initials(name: string): string {
  return name
    .split(/\s+/)
    .map((s) => s[0])
    .filter(Boolean)
    .join('')
    .slice(0, 2)
    .toUpperCase();
}

export function Presence({ users, moreCount, maxShown = 4 }: PresenceProps) {
  const shown = users.slice(0, maxShown);
  // Overflow count defaults to any truncation from users[] beyond maxShown,
  // but an explicit moreCount prop always wins (e.g. when you know the real
  // remote headcount but only have a sample locally).
  const overflow = moreCount ?? (users.length > maxShown ? users.length - maxShown : 0);

  return (
    <div className="flex items-center ml-1" role="group" aria-label="Currently viewing">
      {shown.map((p, i) => (
        <div
          key={p.name}
          title={p.subtitle ? `${p.name} · ${p.subtitle}` : p.name}
          className="relative flex items-center justify-center w-[26px] h-[26px] rounded-full text-white text-[10px] font-bold border-2 border-surface-50 dark:border-surface-900 cursor-default"
          style={{
            background: `linear-gradient(135deg, ${p.color}, color-mix(in oklch, ${p.color} 70%, black))`,
            marginLeft: i === 0 ? 0 : -8,
            zIndex: shown.length - i,
          }}
        >
          {initials(p.name)}
          <span
            aria-hidden="true"
            className="absolute -bottom-px -right-px w-2 h-2 rounded-full bg-emerald-500 border-2 border-surface-50 dark:border-surface-900"
          />
        </div>
      ))}
      {overflow > 0 && (
        <div
          className="flex items-center justify-center w-[26px] h-[26px] rounded-full text-[10px] font-bold text-surface-500 dark:text-surface-400 bg-surface-100 dark:bg-surface-800 border-2 border-surface-50 dark:border-surface-900"
          style={{ marginLeft: -8 }}
        >
          +{overflow}
        </div>
      )}
    </div>
  );
}
