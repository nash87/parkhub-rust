import type { Icon as PhosphorIcon } from '@phosphor-icons/react';

export interface HeroEyebrowProps {
  icon?: PhosphorIcon;
  label: string;
}

export function HeroEyebrow({ icon: Icon, label }: HeroEyebrowProps) {
  return (
    <div className="admin-hero-eyebrow">
      <span className="admin-hero-dot" aria-hidden="true"></span>
      {Icon && <Icon weight="bold" className="w-3.5 h-3.5" />}
      {label}
    </div>
  );
}
