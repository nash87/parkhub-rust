import type { IconKey } from './icons';

/**
 * v5 navigation registry — 26 screens across 3 sections.
 * Single source of truth: consumed by both sidebars (marble + void) and
 * the Command Palette. Section order reflects information hierarchy
 * (main tasks → fleet ops → admin).
 */

export type NavSection = 'main' | 'fleet' | 'admin';

export interface NavItem {
  id: string;
  label: string;
  /** Section label — mapped to localized heading in the sidebar. */
  section: NavSection;
  /** Icon key referencing the v5 icon registry. */
  icon: IconKey;
  /** Editorial numbering for the void sidebar (01…26). */
  n: string;
}

export const NAV: readonly NavItem[] = [
  { id: 'dashboard', icon: 'home', label: 'Dashboard', section: 'main', n: '01' },
  { id: 'buchungen', icon: 'list', label: 'Buchungen', section: 'main', n: '02' },
  { id: 'buchen', icon: 'plus', label: 'Platz buchen', section: 'main', n: '03' },
  { id: 'fahrzeuge', icon: 'car', label: 'Fahrzeuge', section: 'main', n: '04' },
  { id: 'kalender', icon: 'cal', label: 'Kalender', section: 'main', n: '05' },
  { id: 'karte', icon: 'map', label: 'Karte', section: 'main', n: '06' },
  { id: 'credits', icon: 'credit', label: 'Credits', section: 'main', n: '07' },
  { id: 'team', icon: 'users', label: 'Team', section: 'fleet', n: '08' },
  { id: 'rangliste', icon: 'rank', label: 'Rangliste', section: 'fleet', n: '09' },
  { id: 'ev', icon: 'bolt', label: 'EV-Laden', section: 'fleet', n: '10' },
  { id: 'tausch', icon: 'swap', label: 'Tausch', section: 'fleet', n: '11' },
  { id: 'einchecken', icon: 'check', label: 'Einchecken', section: 'fleet', n: '12' },
  { id: 'vorhersagen', icon: 'predict', label: 'Vorhersagen', section: 'fleet', n: '13' },
  { id: 'gaestepass', icon: 'guest', label: 'Gäste-Pass', section: 'fleet', n: '14' },
  { id: 'analytics', icon: 'analytics', label: 'Analytics', section: 'admin', n: '15' },
  { id: 'nutzer', icon: 'users', label: 'Nutzer', section: 'admin', n: '16' },
  { id: 'billing', icon: 'billing', label: 'Abrechnung', section: 'admin', n: '17' },
  { id: 'lobby', icon: 'monitor', label: 'Lobby-Display', section: 'admin', n: '18' },
  { id: 'benachrichtigungen', icon: 'bell', label: 'Benachrichtigungen', section: 'admin', n: '19' },
  { id: 'einstellungen', icon: 'gear', label: 'Einstellungen', section: 'admin', n: '20' },
  { id: 'standorte', icon: 'map', label: 'Standorte', section: 'admin', n: '21' },
  { id: 'integrations', icon: 'key', label: 'Integrationen', section: 'admin', n: '22' },
  { id: 'apikeys', icon: 'key', label: 'API-Schlüssel', section: 'admin', n: '23' },
  { id: 'audit', icon: 'shield', label: 'Audit-Log', section: 'admin', n: '24' },
  { id: 'policies', icon: 'shield', label: 'Richtlinien', section: 'admin', n: '25' },
  { id: 'profil', icon: 'users', label: 'Mein Profil', section: 'main', n: '26' },
] as const;

export const SECTION_HEADINGS: Record<NavSection, string> = {
  main: 'Grundlagen',
  fleet: 'Flotte',
  admin: 'Admin',
};

export type ScreenId = typeof NAV[number]['id'];

export const byId = new Map<string, NavItem>(NAV.map((n) => [n.id, n]));

export function breadcrumbFor(id: ScreenId): string {
  const n = byId.get(id);
  if (!n) return '';
  return `${SECTION_HEADINGS[n.section]} / ${n.label}`;
}
