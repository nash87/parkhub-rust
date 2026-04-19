/**
 * Settings primitives — small, reusable layout + control components
 * extracted from the claude.ai/design v4 handoff bundle (settings.jsx).
 *
 * Ship list:
 *  - SCard     — section card with optional title/subtitle/actions
 *  - SRow      — labelled row with optional lock-by indicator
 *  - SSeg      — segmented control (2-4 options), pill-style
 *  - SToggle   — iOS-style switch
 *  - ThemeSwatches — color-swatch theme picker
 *  - NavLayoutGrid — 2x2 grid of nav-style previews (stub targets
 *                    until nav-variants.jsx ports land in T-1842;
 *                    selection persists but only "classic" renders
 *                    today so the component degrades gracefully.)
 *
 * These are deliberately generic; Settings.tsx composes them.
 */

import type { ReactNode } from 'react';
import { Lock } from '@phosphor-icons/react';

// ─── Section Card ────────────────────────────────────────────────────────

export function SCard({
  title,
  subtitle,
  actions,
  id,
  children,
}: {
  title?: string;
  subtitle?: string;
  actions?: ReactNode;
  id?: string;
  children: ReactNode;
}) {
  return (
    <section
      id={id}
      // `density-card` consumes var(--density-card-padding) so the
      // Appearance → Density segmented control actually reflows this
      // card's inner spacing (compact/cozy/comfortable). See
      // styles/global.css density block + hooks/useDensity.
      className="card density-card mb-4 border border-surface-200 dark:border-surface-800"
    >
      {(title || actions) && (
        <header className="flex items-center justify-between mb-3">
          <div>
            {title && (
              <h2
                className="text-base font-semibold text-surface-900 dark:text-white"
                style={{ letterSpacing: '-0.01em' }}
              >
                {title}
              </h2>
            )}
            {subtitle && (
              <p className="text-[13px] text-surface-500 dark:text-surface-400 mt-0.5">
                {subtitle}
              </p>
            )}
          </div>
          {actions && <div className="flex items-center gap-2">{actions}</div>}
        </header>
      )}
      {children}
    </section>
  );
}

// ─── Labelled Row ────────────────────────────────────────────────────────

export function SRow({
  title,
  description,
  lockedBy,
  compact,
  children,
}: {
  title: string;
  description?: string;
  /** If present, shown as an "Enforced by {lockedBy}" badge with a lock icon */
  lockedBy?: string;
  compact?: boolean;
  children: ReactNode;
}) {
  return (
    <div
      className={`flex items-start justify-between gap-4 ${compact ? 'py-2' : 'py-3'} border-b border-surface-100 dark:border-surface-800 last:border-b-0`}
    >
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <p className="text-sm font-medium text-surface-900 dark:text-white">
            {title}
          </p>
          {lockedBy && (
            <span
              className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded-md text-[10px] font-semibold text-amber-700 dark:text-amber-400 bg-amber-500/10"
              title={`Enforced by ${lockedBy}`}
            >
              <Lock weight="fill" className="w-2.5 h-2.5" />
              {lockedBy}
            </span>
          )}
        </div>
        {description && (
          <p className="text-[12px] text-surface-500 dark:text-surface-400 mt-0.5 leading-relaxed">
            {description}
          </p>
        )}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

// ─── Segmented Control ───────────────────────────────────────────────────

export function SSeg<T extends string>({
  options,
  value,
  onChange,
  disabled,
}: {
  options: { value: T; label: string }[];
  value: T;
  onChange: (value: T) => void;
  disabled?: boolean;
}) {
  return (
    <div
      role="radiogroup"
      className={`inline-flex gap-0.5 p-0.5 rounded-lg bg-surface-100 dark:bg-surface-800 ${disabled ? 'opacity-50 pointer-events-none' : ''}`}
    >
      {options.map((o) => {
        const active = o.value === value;
        return (
          <button
            key={o.value}
            type="button"
            role="radio"
            aria-checked={active}
            onClick={() => onChange(o.value)}
            className={`px-3 py-1.5 text-xs font-semibold rounded-md transition-colors ${
              active
                ? 'bg-white dark:bg-surface-700 text-surface-900 dark:text-white shadow-sm'
                : 'text-surface-500 dark:text-surface-400 hover:text-surface-900 dark:hover:text-white'
            }`}
          >
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

// ─── iOS-style Toggle ────────────────────────────────────────────────────

export function SToggle({
  value,
  onChange,
  disabled,
  label,
}: {
  value: boolean;
  onChange: (value: boolean) => void;
  disabled?: boolean;
  label?: string;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={value}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!value)}
      className={`relative inline-flex w-10 h-6 rounded-full transition-colors ${
        value ? 'bg-primary-500' : 'bg-surface-300 dark:bg-surface-700'
      } ${disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
    >
      <span
        className={`absolute top-0.5 w-5 h-5 rounded-full bg-white shadow-sm transition-transform ${
          value ? 'translate-x-[18px]' : 'translate-x-0.5'
        }`}
      />
    </button>
  );
}

// ─── Theme color-swatch picker ───────────────────────────────────────────

export interface ThemeSwatch {
  value: string;
  label: string;
  /** CSS color (or gradient) rendered in the swatch */
  color: string;
}

export function ThemeSwatches({
  value,
  onChange,
  options,
  disabled,
}: {
  value: string;
  onChange: (value: string) => void;
  options: ThemeSwatch[];
  disabled?: boolean;
}) {
  return (
    <div className="flex flex-wrap gap-2">
      {options.map((o) => {
        const active = o.value === value;
        return (
          <button
            key={o.value}
            type="button"
            role="radio"
            aria-checked={active}
            aria-label={o.label}
            title={o.label}
            disabled={disabled}
            onClick={() => onChange(o.value)}
            className={`w-9 h-9 rounded-full transition-transform ${
              active
                ? 'ring-2 ring-primary-500 ring-offset-2 dark:ring-offset-surface-900 scale-110'
                : 'hover:scale-105'
            } ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
            style={{ background: o.color }}
          />
        );
      })}
    </div>
  );
}

// ─── Nav layout picker — all four variants now live (see components/nav/) ───

export type NavLayout = 'classic' | 'rail' | 'top' | 'dock' | 'focus';

export const NAV_LAYOUTS: { value: NavLayout; label: string; description: string }[] = [
  { value: 'classic', label: 'Classic sidebar', description: 'Left rail with labels' },
  { value: 'rail', label: 'Icon rail', description: 'Icons only, side-popping tooltip' },
  { value: 'top', label: 'Top tabs', description: 'Horizontal navigation + overflow' },
  { value: 'dock', label: 'Floating dock', description: 'macOS-style bottom dock' },
  { value: 'focus', label: 'Focus', description: 'Dark rail with live pass + floor heatmap' },
];

export function NavLayoutGrid({
  value,
  onChange,
  disabled,
}: {
  value: NavLayout;
  onChange: (value: NavLayout) => void;
  disabled?: boolean;
}) {
  return (
    <div
      role="radiogroup"
      className={`grid grid-cols-2 gap-3 ${disabled ? 'opacity-50 pointer-events-none' : ''}`}
    >
      {NAV_LAYOUTS.map((l) => {
        const active = l.value === value;
        return (
          <button
            key={l.value}
            type="button"
            role="radio"
            aria-checked={active}
            onClick={() => onChange(l.value)}
            className={`relative flex flex-col items-start gap-1 p-4 rounded-xl border-2 text-left transition-colors cursor-pointer ${
              active
                ? 'border-primary-500 bg-primary-500/5'
                : 'border-surface-200 dark:border-surface-800 hover:border-surface-300 dark:hover:border-surface-700'
            }`}
          >
            <NavLayoutPreview layout={l.value} active={active} />
            <span className="text-sm font-semibold text-surface-900 dark:text-white mt-2">
              {l.label}
            </span>
            <span className="text-[11px] text-surface-500 dark:text-surface-400 leading-snug">
              {l.description}
            </span>
          </button>
        );
      })}
    </div>
  );
}

/**
 * Tiny schematic thumbnail so users can see at a glance what each
 * layout looks like before committing. Pure CSS — no screenshots to keep
 * in sync, and the preview tracks the active accent colour.
 */
function NavLayoutPreview({ layout, active }: { layout: NavLayout; active: boolean }) {
  const primary = active
    ? 'bg-primary-500'
    : 'bg-surface-300 dark:bg-surface-600';
  const secondary = 'bg-surface-200 dark:bg-surface-700';
  const frame = 'rounded bg-surface-100 dark:bg-surface-800 border border-surface-200 dark:border-surface-700';

  switch (layout) {
    case 'classic':
      return (
        <div className={`relative flex w-full h-20 p-1 gap-1 ${frame}`}>
          <div className="w-1/4 flex flex-col gap-0.5 p-0.5">
            <span className={`block h-1 w-full rounded-sm ${primary}`} />
            <span className={`block h-1 w-3/4 rounded-sm ${secondary}`} />
            <span className={`block h-1 w-3/4 rounded-sm ${secondary}`} />
            <span className={`block h-1 w-3/4 rounded-sm ${secondary}`} />
          </div>
          <div className="flex-1 flex flex-col gap-0.5 p-0.5">
            <span className={`block h-2 w-3/4 rounded-sm ${secondary}`} />
            <span className={`block h-4 w-full rounded-sm ${secondary}`} />
          </div>
        </div>
      );
    case 'rail':
      return (
        <div className={`relative flex w-full h-20 p-1 gap-1 ${frame}`}>
          <div className="w-2 flex flex-col items-center gap-0.5 py-0.5">
            <span className={`block w-1.5 h-1.5 rounded-sm ${primary}`} />
            <span className={`block w-1.5 h-1.5 rounded-sm ${secondary}`} />
            <span className={`block w-1.5 h-1.5 rounded-sm ${secondary}`} />
          </div>
          <div className="flex-1 flex flex-col gap-0.5 p-0.5">
            <span className={`block h-2 w-3/4 rounded-sm ${secondary}`} />
            <span className={`block h-4 w-full rounded-sm ${secondary}`} />
          </div>
        </div>
      );
    case 'top':
      return (
        <div className={`relative flex flex-col w-full h-20 p-1 gap-1 ${frame}`}>
          <div className="flex items-center gap-0.5 h-2">
            <span className={`block h-1 w-4 rounded-sm ${primary}`} />
            <span className={`block h-1 w-3 rounded-sm ${secondary}`} />
            <span className={`block h-1 w-3 rounded-sm ${secondary}`} />
            <span className={`block h-1 w-3 rounded-sm ${secondary}`} />
          </div>
          <div className="flex-1 flex flex-col gap-0.5 p-0.5">
            <span className={`block h-2 w-3/4 rounded-sm ${secondary}`} />
            <span className={`block h-4 w-full rounded-sm ${secondary}`} />
          </div>
        </div>
      );
    case 'focus':
      // Dark opinionated rail with a "live pass" card hint + 3 occupancy bars.
      return (
        <div
          className={`relative flex w-full h-20 p-1 gap-1 rounded ${active ? 'border-primary-500 border' : 'border border-surface-300 dark:border-surface-700'}`}
          style={{ background: 'oklch(0.17 0.02 260)' }}
        >
          <div className="w-2/5 flex flex-col gap-0.5 p-0.5">
            {/* simulated live-pass card */}
            <span
              className="block h-3 w-full rounded-sm"
              style={{ background: active ? 'color-mix(in oklch, var(--color-primary-500) 40%, oklch(0.22 0.02 260))' : 'oklch(0.22 0.02 260)' }}
            />
            {/* occupancy bars */}
            <span className="block h-0.5 w-3/4 rounded-full" style={{ background: 'oklch(0.58 0.16 25)' }} />
            <span className="block h-0.5 w-2/3 rounded-full" style={{ background: 'oklch(0.70 0.14 75)' }} />
            <span className="block h-0.5 w-1/2 rounded-full" style={{ background: 'oklch(0.58 0.14 150)' }} />
          </div>
          <div className="flex-1 flex flex-col gap-0.5 p-0.5">
            <span className="block h-2 w-3/4 rounded-sm" style={{ background: 'oklch(0.32 0.02 260)' }} />
            <span className="block h-4 w-full rounded-sm" style={{ background: 'oklch(0.28 0.02 260)' }} />
          </div>
        </div>
      );
    case 'dock':
    default:
      return (
        <div className={`relative flex flex-col w-full h-20 p-1 ${frame}`}>
          <div className="flex-1 flex flex-col gap-0.5 p-0.5">
            <span className={`block h-2 w-3/4 rounded-sm ${secondary}`} />
            <span className={`block h-4 w-full rounded-sm ${secondary}`} />
          </div>
          <div className="flex items-center justify-center gap-0.5 pb-0.5">
            <span className={`block w-2 h-2 rounded-full ${primary}`} />
            <span className={`block w-2 h-2 rounded-full ${secondary}`} />
            <span className={`block w-2 h-2 rounded-full ${secondary}`} />
            <span className={`block w-2 h-2 rounded-full ${secondary}`} />
          </div>
        </div>
      );
  }
}
