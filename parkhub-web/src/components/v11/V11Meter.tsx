/**
 * V11Meter — SOTA-2026 stat-card primitive.
 *
 * Single source of truth for the .v11-meter chrome (PR #490). Replaces the
 * 3 near-identical local StatCard re-implementations in AdminReports,
 * AdminBilling, AdminAccessible (and any future copies).
 *
 * Tone modifiers map to the global CSS:
 *   primary, accent, info, success, warn, danger
 *
 * Example:
 *   <V11Meter
 *     icon={<Users weight="bold" className="w-3.5 h-3.5" />}
 *     label="Total users"
 *     value={201}
 *     tone="primary"
 *     bar={201 / 1000}    // optional: 0..1 fill
 *   />
 */

import type { ReactNode } from 'react';

export type V11MeterTone =
  | 'primary'
  | 'accent'
  | 'info'
  | 'success'
  | 'warn'
  | 'danger';

export interface V11MeterProps {
  /** UPPERCASE eyebrow label rendered next to the icon. */
  label: string;
  /** Hero-size value (number → rendered as-is; string → use for $ / units). */
  value: string | number;
  /** Eyebrow icon — pass a Phosphor `<Foo weight="bold" className="w-3.5 h-3.5" />`. */
  icon: ReactNode;
  /** Color tone — drives the .v11-meter--{tone} CSS modifier. */
  tone?: V11MeterTone;
  /** Optional 0..1 fill for the bottom progress bar. Omit to hide the bar. */
  bar?: number;
  /** Optional `data-testid` for E2E hooks. */
  testId?: string;
}

export function V11Meter({
  label,
  value,
  icon,
  tone = 'primary',
  bar,
  testId,
}: V11MeterProps) {
  const showBar = typeof bar === 'number';
  const fillPct = showBar ? Math.min(Math.max(bar, 0), 1) * 100 : 0;
  return (
    <div className={`v11-meter v11-meter--${tone}`} data-testid={testId}>
      <div className="v11-meter-eyebrow">
        {icon}
        {label}
      </div>
      <div className="v11-meter-value">{value}</div>
      {showBar && (
        <div className="v11-meter-bar" aria-hidden="true">
          <i style={{ width: `${fillPct}%` }}></i>
        </div>
      )}
    </div>
  );
}
