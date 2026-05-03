/**
 * HeroEyebrow — SOTA-2026 hero eyebrow primitive.
 *
 * Single source of truth for the .admin-hero-eyebrow chrome (PR #489).
 * Renders the pulsing dot + icon + UPPERCASE label group used at the top
 * of every v11 hero across all 45 admin + user-facing pages.
 *
 * Example:
 *   <HeroEyebrow icon={ShieldCheck} label={t('rbac.eyebrow', 'ACCESS CONTROL')} />
 */

import type { Icon as PhosphorIcon } from '@phosphor-icons/react';

export interface HeroEyebrowProps {
  /** Phosphor icon component (e.g. `ShieldCheck`, `MapPin`, `UserPlus`). */
  icon: PhosphorIcon;
  /** UPPERCASE eyebrow text — typically t('section.eyebrow', 'FALLBACK'). */
  label: string;
}

export function HeroEyebrow({ icon: Icon, label }: HeroEyebrowProps) {
  return (
    <div className="admin-hero-eyebrow">
      <span className="admin-hero-dot" aria-hidden="true"></span>
      <Icon weight="bold" className="w-3.5 h-3.5" />
      {label}
    </div>
  );
}
