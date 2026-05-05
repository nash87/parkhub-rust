import type { ReactNode } from 'react';

export type V11MeterTone =
  | 'primary'
  | 'accent'
  | 'info'
  | 'success'
  | 'warn'
  | 'danger';

export interface V11MeterProps {
  label: string;
  value: string | number;
  icon: ReactNode;
  tone?: V11MeterTone;
  bar?: number;
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
