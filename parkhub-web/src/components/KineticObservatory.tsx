/**
 * Kinetic Observatory component kit.
 *
 * Implements the .stitch/DESIGN.md vision: KPI cards with delta badges,
 * trend sparklines, live sensor feeds, and recent-activity tables. Uses
 * the theme bridge tokens (--theme-card-bg, --glass-*) so every one of
 * the 16 design themes auto-repaints these surfaces.
 *
 * Motion: framer-motion with spring (stiffness 300, damping 30).
 * Typography: tabular-nums on values, -0.02em tracking on headlines.
 * No 1px borders — ghost borders via color-mix / outline at low alpha.
 */
import { motion } from 'framer-motion';
import { ArrowUp, ArrowDown, CircleDashed } from '@phosphor-icons/react';
import type { ReactNode } from 'react';
import { AnimatedCounter } from './AnimatedCounter';

// ── Spring physics config (DESIGN.md §5) ───────────────────────────────
const spring = { type: 'spring' as const, stiffness: 300, damping: 30 };

/* ═══════════════════════════════════════════════════════════════════════
   KPI Stat Card with optional delta badge + live indicator
   ═══════════════════════════════════════════════════════════════════════ */

export interface KpiCardProps {
  /** Machine-neutral label, e.g. "Total Revenue" */
  label: string;
  /** The number to display (rendered with AnimatedCounter + tabular-nums) */
  value: number;
  /** Optional prefix (e.g. "$" or "€") */
  prefix?: string;
  /** Optional suffix (e.g. "%" or "min") */
  suffix?: string;
  /** Optional icon on the left */
  icon?: ReactNode;
  /** Optional delta badge — positive ⇒ up/green, negative ⇒ down/red */
  delta?: {
    value: number;
    suffix?: string;
    label?: string;
  };
  /** Show a pulsing "Live" dot when true */
  live?: boolean;
  /** Visual size (default 1x1) */
  size?: 'sm' | 'md' | 'lg';
  /** Testing id */
  'data-testid'?: string;
}

export function KpiCard({
  label,
  value,
  prefix,
  suffix,
  icon,
  delta,
  live,
  size = 'md',
  ...rest
}: KpiCardProps) {
  const padding = size === 'sm' ? 'p-4' : size === 'lg' ? 'p-6' : 'p-5';
  const valueSize = size === 'sm' ? 'text-2xl' : size === 'lg' ? 'text-4xl' : 'text-3xl';

  const isPositive = delta && delta.value >= 0;
  const DeltaIcon = isPositive ? ArrowUp : ArrowDown;

  return (
    <motion.div
      whileHover={{ scale: 1.02, y: -2 }}
      transition={spring}
      data-testid={rest['data-testid'] ?? `kpi-${label.toLowerCase().replace(/\s+/g, '-')}`}
      className={`card relative overflow-hidden ${padding} group`}
    >
      {/* Ambient primary-tinted glow (DESIGN.md §2 "Glass Morphism") */}
      <div className="pointer-events-none absolute inset-0 opacity-0 group-hover:opacity-100 transition-opacity duration-400">
        <div className="absolute -top-10 -right-10 w-32 h-32 rounded-full bg-primary-500/15 blur-2xl" />
      </div>

      <div className="relative flex items-start justify-between">
        <div className="flex items-center gap-2">
          {icon && (
            <div className="flex items-center justify-center w-8 h-8 rounded-lg bg-primary-500/10 text-primary-500 [&>svg]:w-4 [&>svg]:h-4">
              {icon}
            </div>
          )}
          <p className="text-xs font-medium uppercase tracking-wider text-surface-500 dark:text-surface-400">
            {label}
          </p>
        </div>

        {live ? (
          <LiveBadge />
        ) : delta ? (
          <DeltaBadge value={delta.value} suffix={delta.suffix} positive={isPositive} Icon={DeltaIcon} />
        ) : null}
      </div>

      <p
        className={`relative mt-4 ${valueSize} font-bold text-surface-900 dark:text-white`}
        style={{ fontVariantNumeric: 'tabular-nums', letterSpacing: '-0.03em', lineHeight: 1 }}
      >
        {prefix}
        <AnimatedCounter value={value} duration={800} />
        {suffix}
      </p>
    </motion.div>
  );
}

function LiveBadge() {
  return (
    <span
      data-testid="live-badge"
      className="inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-[10px] font-semibold uppercase tracking-wider bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
    >
      <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 pulse-dot" />
      Live
    </span>
  );
}

function DeltaBadge({
  value,
  suffix = '%',
  positive,
  Icon,
}: {
  value: number;
  suffix?: string;
  positive: boolean;
  Icon: typeof ArrowUp;
}) {
  const color = positive
    ? 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400'
    : 'bg-rose-500/10 text-rose-600 dark:text-rose-400';
  return (
    <span
      data-testid="delta-badge"
      className={`inline-flex items-center gap-0.5 px-2 py-1 rounded-full text-[10px] font-semibold ${color}`}
      style={{ fontVariantNumeric: 'tabular-nums' }}
    >
      <Icon weight="bold" className="w-2.5 h-2.5" />
      {Math.abs(value)}
      {suffix}
    </span>
  );
}

/* ═══════════════════════════════════════════════════════════════════════
   Trend Sparkline Card — miniature SVG line chart with gradient fill
   ═══════════════════════════════════════════════════════════════════════ */

export interface TrendCardProps {
  title: string;
  subtitle?: string;
  /** Array of numeric values to plot */
  points: number[];
  /** Optional period selector options */
  periods?: { key: string; label: string }[];
  /** Currently selected period */
  activePeriod?: string;
  onPeriodChange?: (key: string) => void;
  /** Optional tick labels under the chart */
  labels?: string[];
  /** Accessible chart description for screen readers */
  ariaLabel?: string;
}

export function TrendCard({
  title,
  subtitle,
  points,
  periods,
  activePeriod,
  onPeriodChange,
  labels,
  ariaLabel,
}: TrendCardProps) {
  const width = 400;
  const height = 140;
  const padX = 12;
  const padY = 8;

  const min = Math.min(...points, 0);
  const max = Math.max(...points, 1);
  const range = max - min || 1;

  const toX = (i: number) =>
    padX + (i / Math.max(points.length - 1, 1)) * (width - padX * 2);
  const toY = (v: number) =>
    height - padY - ((v - min) / range) * (height - padY * 2);

  const pathD = points
    .map((v, i) => `${i === 0 ? 'M' : 'L'}${toX(i).toFixed(1)},${toY(v).toFixed(1)}`)
    .join(' ');
  const areaD = `${pathD} L${toX(points.length - 1).toFixed(1)},${height - padY} L${toX(0).toFixed(1)},${height - padY} Z`;

  return (
    <motion.section
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={spring}
      data-testid="trend-card"
      className="card p-6"
      aria-label={ariaLabel ?? title}
    >
      <header className="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-3 mb-4">
        <div>
          <h3 className="text-lg font-semibold text-surface-900 dark:text-white" style={{ letterSpacing: '-0.02em' }}>
            {title}
          </h3>
          {subtitle && (
            <p className="text-sm text-surface-500 dark:text-surface-400 mt-0.5">{subtitle}</p>
          )}
        </div>
        {periods && periods.length > 0 && (
          <nav className="inline-flex rounded-lg bg-surface-100 dark:bg-surface-800/60 p-0.5" aria-label={title}>
            {periods.map((p) => (
              <button
                key={p.key}
                onClick={() => onPeriodChange?.(p.key)}
                type="button"
                aria-pressed={activePeriod === p.key}
                className={`px-3 py-1.5 rounded-md text-xs font-semibold transition-all ${
                  activePeriod === p.key
                    ? 'bg-white dark:bg-surface-700 text-surface-900 dark:text-white shadow-sm'
                    : 'text-surface-500 hover:text-surface-700 dark:hover:text-surface-300'
                }`}
              >
                {p.label}
              </button>
            ))}
          </nav>
        )}
      </header>

      <svg viewBox={`0 0 ${width} ${height}`} className="w-full h-36" role="img" aria-label={ariaLabel ?? title}>
        <defs>
          <linearGradient id="trend-grad" x1="0%" y1="0%" x2="0%" y2="100%">
            <stop offset="0%" stopColor="var(--color-primary-500, #14b8a6)" stopOpacity="0.35" />
            <stop offset="100%" stopColor="var(--color-primary-500, #14b8a6)" stopOpacity="0" />
          </linearGradient>
        </defs>
        <path d={areaD} fill="url(#trend-grad)" />
        <motion.path
          d={pathD}
          fill="none"
          stroke="var(--color-primary-500, #14b8a6)"
          strokeWidth={2}
          strokeLinecap="round"
          strokeLinejoin="round"
          initial={{ pathLength: 0 }}
          animate={{ pathLength: 1 }}
          transition={{ duration: 0.8, ease: 'easeOut' }}
        />
        {points.map((v, i) => (
          <circle
            key={i}
            cx={toX(i)}
            cy={toY(v)}
            r={2.5}
            fill="var(--color-primary-500, #14b8a6)"
          />
        ))}
      </svg>

      {labels && labels.length > 0 && (
        <div className="flex justify-between mt-2 px-3 text-[10px] font-medium text-surface-500 dark:text-surface-400 uppercase tracking-wider">
          {labels.map((l) => (
            <span key={l}>{l}</span>
          ))}
        </div>
      )}
    </motion.section>
  );
}

/* ═══════════════════════════════════════════════════════════════════════
   Live Sensor Feed — list of sensors with status indicators
   ═══════════════════════════════════════════════════════════════════════ */

export type SensorStatus = 'active' | 'maintenance' | 'offline';

export interface SensorEntry {
  name: string;
  status: SensorStatus;
}

export interface SensorFeedCardProps {
  title: string;
  subtitle?: string;
  sensors: SensorEntry[];
}

const STATUS_STYLES: Record<SensorStatus, { label: string; dot: string; text: string }> = {
  active: {
    label: 'Active',
    dot: 'bg-emerald-500',
    text: 'text-emerald-600 dark:text-emerald-400',
  },
  maintenance: {
    label: 'Maintenance',
    dot: 'bg-amber-500',
    text: 'text-amber-600 dark:text-amber-400',
  },
  offline: {
    label: 'Offline',
    dot: 'bg-rose-500',
    text: 'text-rose-600 dark:text-rose-400',
  },
};

export function SensorFeedCard({ title, subtitle, sensors }: SensorFeedCardProps) {
  return (
    <motion.section
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={spring}
      data-testid="sensor-feed-card"
      className="card p-6"
    >
      <header className="mb-4">
        <h3 className="text-lg font-semibold text-surface-900 dark:text-white" style={{ letterSpacing: '-0.02em' }}>
          {title}
        </h3>
        {subtitle && (
          <p className="text-sm text-surface-500 dark:text-surface-400 mt-0.5">{subtitle}</p>
        )}
      </header>

      <ul className="space-y-2">
        {sensors.map((s) => {
          const styles = STATUS_STYLES[s.status];
          return (
            <li
              key={s.name}
              data-testid={`sensor-${s.name.toLowerCase().replace(/\s+/g, '-')}`}
              className="flex items-center justify-between rounded-lg px-3 py-2 bg-surface-50/60 dark:bg-surface-800/40"
            >
              <span className="flex items-center gap-2 text-sm text-surface-700 dark:text-surface-300">
                <span className={`w-1.5 h-1.5 rounded-full ${styles.dot} ${s.status === 'active' ? 'pulse-dot' : ''}`} />
                {s.name}
              </span>
              <span className={`text-xs font-semibold uppercase tracking-wider ${styles.text}`}>
                {styles.label}
              </span>
            </li>
          );
        })}
      </ul>
    </motion.section>
  );
}

/* ═══════════════════════════════════════════════════════════════════════
   Recent Activity Table
   ═══════════════════════════════════════════════════════════════════════ */

export type ActivityStatus = 'in_progress' | 'confirmed' | 'pending' | 'completed' | 'cancelled';

export interface ActivityRow {
  id: string;
  vehicle: string;
  owner?: string;
  slot: string;
  checkInTime?: string;
  duration?: string;
  status: ActivityStatus;
}

export interface RecentActivityCardProps {
  title: string;
  rows: ActivityRow[];
  viewAllHref?: string;
  onViewAll?: () => void;
  emptyText?: string;
  columnLabels?: {
    vehicle: string;
    slot: string;
    checkIn: string;
    duration: string;
    status: string;
  };
}

const ACTIVITY_STATUS_STYLES: Record<ActivityStatus, string> = {
  in_progress: 'bg-primary-500/10 text-primary-600 dark:text-primary-400',
  confirmed: 'bg-emerald-500/10 text-emerald-600 dark:text-emerald-400',
  pending: 'bg-amber-500/10 text-amber-600 dark:text-amber-400',
  completed: 'bg-surface-500/10 text-surface-500 dark:text-surface-400',
  cancelled: 'bg-rose-500/10 text-rose-600 dark:text-rose-400',
};

export function RecentActivityCard({
  title,
  rows,
  viewAllHref,
  onViewAll,
  emptyText = 'No recent activity',
  columnLabels = {
    vehicle: 'Vehicle / Owner',
    slot: 'Slot No.',
    checkIn: 'Check-In Time',
    duration: 'Duration',
    status: 'Status',
  },
}: RecentActivityCardProps) {
  return (
    <motion.section
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={spring}
      data-testid="recent-activity-card"
      className="card p-6"
    >
      <header className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-semibold text-surface-900 dark:text-white" style={{ letterSpacing: '-0.02em' }}>
          {title}
        </h3>
        {(viewAllHref || onViewAll) && (
          <a
            href={viewAllHref ?? '#'}
            onClick={(e) => {
              if (onViewAll) {
                e.preventDefault();
                onViewAll();
              }
            }}
            className="text-sm text-primary-600 hover:text-primary-500 dark:text-primary-400 font-medium"
          >
            View All
          </a>
        )}
      </header>

      {rows.length === 0 ? (
        <div className="py-12 text-center">
          <CircleDashed weight="bold" className="w-8 h-8 text-surface-300 dark:text-surface-600 mx-auto mb-3" />
          <p className="text-sm text-surface-500 dark:text-surface-400">{emptyText}</p>
        </div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full text-sm" data-testid="recent-activity-table">
            <thead>
              <tr className="text-left text-[11px] font-semibold uppercase tracking-wider text-surface-500 dark:text-surface-400 border-b border-surface-200/50 dark:border-surface-800/50">
                <th className="pb-2 pr-4">{columnLabels.vehicle}</th>
                <th className="pb-2 pr-4">{columnLabels.slot}</th>
                <th className="pb-2 pr-4 hidden sm:table-cell">{columnLabels.checkIn}</th>
                <th className="pb-2 pr-4 hidden md:table-cell">{columnLabels.duration}</th>
                <th className="pb-2">{columnLabels.status}</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr
                  key={r.id}
                  data-testid={`activity-row-${r.id}`}
                  className="border-b border-surface-100/50 dark:border-surface-800/30 last:border-0"
                >
                  <td className="py-3 pr-4">
                    <div className="font-medium text-surface-900 dark:text-white">{r.vehicle}</div>
                    {r.owner && (
                      <div className="text-xs text-surface-500 dark:text-surface-400 mt-0.5">{r.owner}</div>
                    )}
                  </td>
                  <td className="py-3 pr-4 text-surface-700 dark:text-surface-300" style={{ fontVariantNumeric: 'tabular-nums' }}>
                    {r.slot}
                  </td>
                  <td className="py-3 pr-4 text-surface-700 dark:text-surface-300 hidden sm:table-cell" style={{ fontVariantNumeric: 'tabular-nums' }}>
                    {r.checkInTime ?? '—'}
                  </td>
                  <td className="py-3 pr-4 text-surface-700 dark:text-surface-300 hidden md:table-cell" style={{ fontVariantNumeric: 'tabular-nums' }}>
                    {r.duration ?? '—'}
                  </td>
                  <td className="py-3">
                    <span
                      className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-semibold uppercase tracking-wider ${ACTIVITY_STATUS_STYLES[r.status]}`}
                    >
                      {r.status.replace('_', ' ')}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </motion.section>
  );
}
