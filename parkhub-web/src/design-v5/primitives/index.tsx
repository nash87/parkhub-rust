import { type CSSProperties, type ReactNode } from 'react';
import { icons, type IconKey } from '../icons';

/* ═════════════════════════════════════════════════════════════
   v5 Primitives — thin, tokenized, composable
   All surfaces read from CSS custom properties (--v5-*) so a
   single parent attribute flip repaints the entire subtree.
   ═════════════════════════════════════════════════════════════ */

/** Thin-stroke SVG icon. `d` is either a single path or an array of paths. */
export function V5Icon({
  d,
  size = 15,
  color = 'currentColor',
  strokeWidth = 1.6,
}: {
  d: string | readonly string[];
  size?: number;
  color?: string;
  strokeWidth?: number;
}) {
  const paths = Array.isArray(d) ? d : [d as string];
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {paths.map((p, i) => (
        <path key={i} d={p} />
      ))}
    </svg>
  );
}

/** Convenience — render by registry key so consumers don't import `icons` directly. */
export function V5NamedIcon({ name, ...rest }: { name: IconKey } & Omit<Parameters<typeof V5Icon>[0], 'd'>) {
  return <V5Icon d={icons[name]} {...rest} />;
}

export type BadgeVariant =
  | 'primary' | 'success' | 'warning' | 'error' | 'info' | 'gray' | 'ev' | 'purple';

const BADGE_COLORS: Record<BadgeVariant, { bg: string; fg: string }> = {
  primary: { bg: 'oklch(0.57 0.14 175 / 0.12)', fg: 'oklch(0.57 0.14 175)' },
  success: { bg: 'oklch(0.65 0.17 160 / 0.12)', fg: 'oklch(0.65 0.17 160)' },
  warning: { bg: 'oklch(0.74 0.16 75 / 0.12)', fg: 'oklch(0.74 0.16 75)' },
  error: { bg: 'oklch(0.58 0.22 25 / 0.12)', fg: 'oklch(0.58 0.22 25)' },
  info: { bg: 'oklch(0.58 0.18 260 / 0.12)', fg: 'oklch(0.58 0.18 260)' },
  gray: { bg: 'oklch(0.5 0 0 / 0.15)', fg: 'oklch(0.7 0 0)' },
  ev: { bg: 'oklch(0.62 0.18 260 / 0.12)', fg: 'oklch(0.62 0.18 260)' },
  purple: { bg: 'oklch(0.62 0.18 295 / 0.12)', fg: 'oklch(0.62 0.18 295)' },
};

export function Badge({
  variant = 'primary',
  dot = false,
  children,
}: {
  variant?: BadgeVariant;
  dot?: boolean;
  children: ReactNode;
}) {
  const { bg, fg } = BADGE_COLORS[variant];
  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 4,
        padding: '2px 8px',
        borderRadius: 999,
        fontSize: 10,
        fontWeight: 500,
        background: bg,
        color: fg,
        flexShrink: 0,
      }}
    >
      {dot && (
        <span
          style={{
            width: 5,
            height: 5,
            borderRadius: '50%',
            background: fg,
            display: 'inline-block',
          }}
          aria-hidden="true"
        />
      )}
      {children}
    </span>
  );
}

/**
 * Surface card. Uses the `.v5-lift` utility for subtle hover elevation —
 * disabled automatically under prefers-reduced-motion via the utility's rule.
 */
export function Card({
  children,
  style,
  className = '',
  lift = true,
  as: Tag = 'div',
  onClick,
}: {
  children: ReactNode;
  style?: CSSProperties;
  className?: string;
  lift?: boolean;
  as?: 'div' | 'button' | 'a';
  onClick?: () => void;
}) {
  return (
    <Tag
      onClick={onClick}
      className={`${lift ? 'v5-lift' : ''} ${className}`.trim()}
      style={{
        background: 'var(--v5-sur)',
        border: '1px solid var(--v5-bor)',
        borderRadius: 14,
        boxShadow: 'var(--v5-shadow-card)',
        ...style,
      }}
    >
      {children}
    </Tag>
  );
}

/** Small DM Mono caps label — goes above section content. */
export function SectionLabel({ children }: { children: ReactNode }) {
  return (
    <div
      className="v5-mono"
      style={{
        fontSize: 9,
        letterSpacing: 1.4,
        color: 'var(--v5-mut)',
        textTransform: 'uppercase',
        marginBottom: 8,
      }}
    >
      {children}
    </div>
  );
}

export function Divider() {
  return <div style={{ height: 1, background: 'var(--v5-bor)', margin: '10px 0' }} />;
}

export function LiveDot({ color = 'var(--v5-ok)' }: { color?: string }) {
  return (
    <span
      aria-hidden="true"
      style={{
        width: 7,
        height: 7,
        borderRadius: '50%',
        background: color,
        display: 'inline-block',
        animation: 'ph-v5-pulse 2s infinite',
        flexShrink: 0,
      }}
    />
  );
}

export function Toggle({
  checked,
  onChange,
  ariaLabel,
}: {
  checked: boolean;
  onChange?: (next: boolean) => void;
  ariaLabel?: string;
}) {
  const disabled = !onChange;
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel}
      disabled={disabled}
      onClick={() => onChange?.(!checked)}
      style={{
        width: 38,
        height: 21,
        borderRadius: 11,
        background: checked ? 'var(--v5-acc)' : 'var(--v5-bor)',
        cursor: disabled ? 'default' : 'pointer',
        position: 'relative',
        transition: 'background 0.2s',
        flexShrink: 0,
        border: 0,
        padding: 0,
      }}
    >
      <span
        style={{
          position: 'absolute',
          top: 2.5,
          left: checked ? 19 : 2.5,
          width: 16,
          height: 16,
          borderRadius: '50%',
          background: '#fff',
          transition: 'left 0.2s',
          boxShadow: '0 1px 4px rgba(0, 0, 0, 0.25)',
        }}
      />
    </button>
  );
}

export function Row({
  label,
  sub,
  children,
  last = false,
}: {
  label: string;
  sub?: string;
  children?: ReactNode;
  last?: boolean;
}) {
  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        padding: '11px 18px',
        borderBottom: last ? 'none' : '1px solid var(--v5-bor)',
        gap: 12,
      }}
    >
      <div style={{ flex: 1 }}>
        <div style={{ fontSize: 12, color: 'var(--v5-txt)', fontWeight: 500 }}>{label}</div>
        {sub && <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 1 }}>{sub}</div>}
      </div>
      {children}
    </div>
  );
}

/**
 * KPI card — `value` may be a ReactNode (for <NumberFlow>-style widgets)
 * or a primitive. `accent` switches to the tinted surface styling used
 * for the "credits" hero in the dashboard.
 */
export function StatCard({
  label,
  value,
  sub,
  accent = false,
  icon,
  delay = 0,
}: {
  label: string;
  value: ReactNode;
  sub?: string;
  accent?: boolean;
  icon?: IconKey;
  delay?: number;
}) {
  return (
    <div
      className="v5-lift v5-ani"
      style={{
        background: accent ? 'var(--v5-acc-muted)' : 'var(--v5-sur)',
        border: `1px solid ${accent ? 'color-mix(in oklch, var(--v5-acc) 50%, transparent)' : 'var(--v5-bor)'}`,
        borderRadius: 12,
        padding: '12px 16px',
        animationDelay: `${delay}s`,
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
        <div
          className="v5-mono"
          style={{
            fontSize: 9,
            letterSpacing: 1.3,
            color: accent ? 'var(--v5-acc)' : 'var(--v5-mut)',
            textTransform: 'uppercase',
          }}
        >
          {label}
        </div>
        {icon && (
          <div
            style={{
              width: 26,
              height: 26,
              borderRadius: 8,
              background: accent ? 'var(--v5-acc-muted)' : 'var(--v5-sur2)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
            }}
          >
            <V5Icon
              d={icons[icon]}
              size={12}
              color={accent ? 'var(--v5-acc)' : 'var(--v5-mut)'}
            />
          </div>
        )}
      </div>
      <div
        className="v5-mono"
        style={{
          fontSize: 26,
          fontWeight: 700,
          color: accent ? 'var(--v5-acc)' : 'var(--v5-txt)',
          letterSpacing: '-0.5px',
          margin: '4px 0',
          // Use Inter by default for the number for balanced weight; the
          // v5-mono class provides tabular-nums which applies either way.
          fontFamily: 'inherit',
        }}
      >
        {value}
      </div>
      {sub && <div style={{ fontSize: 10, color: 'var(--v5-mut)' }}>{sub}</div>}
    </div>
  );
}
