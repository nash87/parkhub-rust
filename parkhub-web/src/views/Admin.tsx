import { Outlet, Link, useLocation } from 'react-router-dom';
import { useState } from 'react';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  ChartBar, GearSix, Users, Megaphone, ChartLine, MapPin, Translate, PresentationChart, Gauge,
  Buildings, ClockCounterClockwise, Database, Car, Wheelchair, Wrench, CurrencyDollar, UserPlus, Lightning,
  PuzzlePiece, GraphicsCard, ShieldCheck, LockKey, MapTrifold, ArrowsClockwise, List, X, ArrowSquareOut,
} from '@phosphor-icons/react';

/**
 * Admin shell — categorised sidebar on desktop, mobile drawer on small
 * screens. Replaces the previous horizontal-scrolling tab bar which required
 * users to sideways-swipe through 24 tabs to find features like Modules or
 * Compliance. Sections are grouped by domain (Overview / Operations /
 * People & Access / Compliance & Data / Billing & Plans / Integrations),
 * matching the claude.ai/design v4 pattern used on the Settings hub.
 */

type NavItem = {
  key: string;
  label: string;
  to: string;
  icon: React.ComponentType<{ className?: string; weight?: 'regular' | 'fill' | 'bold' | 'duotone' }>;
  external?: boolean;
};

type NavSection = {
  key: string;
  label: string;
  items: NavItem[];
};

function useAdminSections(): NavSection[] {
  const { t } = useTranslation();
  return [
    {
      key: 'overview',
      label: t('admin.group.overview', 'Overview'),
      items: [
        { key: 'reports', label: t('admin.overview', 'Overview'), to: '/admin', icon: ChartBar },
        { key: 'analytics', label: t('admin.analytics', 'Analytics'), to: '/admin/analytics', icon: PresentationChart },
        { key: 'audit', label: t('admin.auditLog', 'Audit Log'), to: '/admin/audit-log', icon: ClockCounterClockwise },
      ],
    },
    {
      key: 'operations',
      label: t('admin.group.operations', 'Operations'),
      items: [
        { key: 'lots', label: t('admin.lots', 'Lots'), to: '/admin/lots', icon: MapPin },
        { key: 'zones', label: t('parkingZones.title', 'Zones'), to: '/admin/zones', icon: MapTrifold },
        { key: 'fleet', label: t('admin.fleet', 'Fleet'), to: '/admin/fleet', icon: Car },
        { key: 'chargers', label: t('admin.chargers', 'EV Chargers'), to: '/admin/chargers', icon: Lightning },
        { key: 'maintenance', label: t('admin.maintenance', 'Maintenance'), to: '/admin/maintenance', icon: Wrench },
        { key: 'accessible', label: t('admin.accessible', 'Accessible'), to: '/admin/accessible', icon: Wheelchair },
        { key: 'visitors', label: t('admin.visitors', 'Visitors'), to: '/admin/visitors', icon: UserPlus },
      ],
    },
    {
      key: 'people',
      label: t('admin.group.peopleAccess', 'People & Access'),
      items: [
        { key: 'users', label: t('admin.users', 'Users'), to: '/admin/users', icon: Users },
        { key: 'roles', label: t('rbac.title', 'Roles'), to: '/admin/roles', icon: LockKey },
        { key: 'tenants', label: t('admin.tenants', 'Tenants'), to: '/admin/tenants', icon: Buildings },
        { key: 'sso', label: t('admin.sso', 'SSO & SAML'), to: '/admin/sso', icon: ShieldCheck },
      ],
    },
    {
      key: 'compliance',
      label: t('admin.group.complianceData', 'Compliance & Data'),
      items: [
        { key: 'compliance', label: t('compliance.title', 'Compliance'), to: '/admin/compliance', icon: ShieldCheck },
        { key: 'data', label: t('admin.dataManagement', 'Data'), to: '/admin/data', icon: Database },
        { key: 'rateLimits', label: t('admin.rateLimits', 'Rate Limits'), to: '/admin/rate-limits', icon: Gauge },
        { key: 'announcements', label: t('admin.announcements', 'Announcements'), to: '/admin/announcements', icon: Megaphone },
      ],
    },
    {
      key: 'billing',
      label: t('admin.group.billingReports', 'Billing & Reports'),
      items: [
        { key: 'billing', label: t('admin.billing', 'Billing'), to: '/admin/billing', icon: CurrencyDollar },
        { key: 'reports', label: t('admin.reports', 'Reports'), to: '/admin/reports', icon: ChartLine },
        { key: 'translations', label: t('admin.translations', 'Translations'), to: '/admin/translations', icon: Translate },
      ],
    },
    {
      key: 'platform',
      label: t('admin.group.platform', 'Platform'),
      items: [
        { key: 'settings', label: t('admin.settings', 'Settings'), to: '/admin/settings', icon: GearSix },
        { key: 'modules', label: t('admin.modules.title', 'Modules & Features'), to: '/admin/modules', icon: PuzzlePiece },
        { key: 'plugins', label: t('admin.plugins', 'Plugins'), to: '/admin/plugins', icon: PuzzlePiece },
        { key: 'updates', label: t('nav.updates', 'Updates'), to: '/admin/updates', icon: ArrowsClockwise },
        { key: 'graphql', label: 'GraphQL Playground', to: '/api/v1/graphql/playground', icon: GraphicsCard, external: true },
      ],
    },
  ];
}

function isActivePath(pathname: string, to: string): boolean {
  if (to === '/admin') return pathname === '/admin';
  return pathname === to || pathname.startsWith(to + '/');
}

function AdminSidebar({ sections, onNavigate }: { sections: NavSection[]; onNavigate?: () => void }) {
  const location = useLocation();
  return (
    <nav aria-label="Admin navigation" className="flex flex-col gap-6">
      {sections.map(section => (
        <div key={section.key}>
          <div className="px-3 mb-1.5 text-[11px] font-semibold uppercase tracking-wider text-surface-500 dark:text-surface-400">
            {section.label}
          </div>
          <ul className="flex flex-col gap-0.5">
            {section.items.map(item => {
              const active = !item.external && isActivePath(location.pathname, item.to);
              const Icon = item.icon;
              const linkClass = `group flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                active
                  ? 'bg-primary-50 text-primary-700 dark:bg-primary-900/30 dark:text-primary-300'
                  : 'text-surface-700 dark:text-surface-300 hover:bg-surface-100 dark:hover:bg-surface-800 hover:text-surface-900 dark:hover:text-white'
              }`;
              const content = (
                <>
                  <Icon weight={active ? 'fill' : 'regular'} className="w-4 h-4 shrink-0" />
                  <span className="truncate flex-1">{item.label}</span>
                  {item.external && <ArrowSquareOut weight="regular" className="w-3 h-3 text-surface-400" />}
                  {active && (
                    <motion.span
                      layoutId="admin-active-dot"
                      className="w-1.5 h-1.5 rounded-full bg-primary-500 shrink-0"
                      transition={{ type: 'spring', stiffness: 500, damping: 30 }}
                    />
                  )}
                </>
              );
              if (item.external) {
                return (
                  <li key={item.key}>
                    <a
                      href={item.to}
                      target="_blank"
                      rel="noopener noreferrer"
                      className={linkClass}
                      onClick={onNavigate}
                    >
                      {content}
                    </a>
                  </li>
                );
              }
              return (
                <li key={item.key}>
                  <Link to={item.to} className={linkClass} aria-current={active ? 'page' : undefined} onClick={onNavigate}>
                    {content}
                  </Link>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
    </nav>
  );
}

export function AdminPage() {
  const { t } = useTranslation();
  const location = useLocation();
  const sections = useAdminSections();
  const [drawerOpen, setDrawerOpen] = useState(false);

  // Find current item label for mobile header
  const currentLabel = (() => {
    for (const section of sections) {
      for (const item of section.items) {
        if (!item.external && isActivePath(location.pathname, item.to)) return item.label;
      }
    }
    return t('admin.title');
  })();

  return (
    <div className="mx-auto max-w-7xl">
      {/* Mobile header with drawer trigger */}
      <div className="lg:hidden sticky top-0 z-30 -mx-4 px-4 py-3 flex items-center gap-3 bg-surface-50/90 dark:bg-surface-950/90 backdrop-blur border-b border-surface-200 dark:border-surface-700">
        <button
          type="button"
          onClick={() => setDrawerOpen(true)}
          aria-label={t('admin.openNav', 'Open admin navigation')}
          className="p-2 -ml-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800"
        >
          <List weight="bold" className="w-5 h-5" />
        </button>
        <div className="flex-1 min-w-0">
          <div className="text-[11px] uppercase tracking-wider text-surface-500 dark:text-surface-400">{t('admin.title')}</div>
          <div className="text-sm font-semibold text-surface-900 dark:text-white truncate">{currentLabel}</div>
        </div>
      </div>

      <div className="flex gap-6">
        {/* Desktop sidebar */}
        <aside className="hidden lg:block w-64 shrink-0 pt-2">
          <div className="mb-5">
            <h1 className="text-xl font-bold text-surface-900 dark:text-white">{t('admin.title')}</h1>
            <p className="text-xs text-surface-500 dark:text-surface-400 mt-0.5">{t('admin.subtitle')}</p>
          </div>
          <div className="sticky top-4">
            <AdminSidebar sections={sections} />
          </div>
        </aside>

        {/* Mobile drawer */}
        {drawerOpen && (
          <motion.div
            className="lg:hidden fixed inset-0 z-50"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
          >
            <button
              type="button"
              aria-label={t('admin.closeNav', 'Close admin navigation')}
              onClick={() => setDrawerOpen(false)}
              className="absolute inset-0 bg-black/40 backdrop-blur-sm"
            />
            <motion.aside
              initial={{ x: '-100%' }}
              animate={{ x: 0 }}
              exit={{ x: '-100%' }}
              transition={{ type: 'spring', stiffness: 400, damping: 36 }}
              className="absolute left-0 top-0 bottom-0 w-72 max-w-[82%] bg-white dark:bg-surface-900 shadow-2xl overflow-y-auto p-4"
            >
              <div className="flex items-center justify-between mb-4">
                <div>
                  <h2 className="font-bold text-surface-900 dark:text-white">{t('admin.title')}</h2>
                  <p className="text-[11px] text-surface-500 dark:text-surface-400">{t('admin.subtitle')}</p>
                </div>
                <button
                  type="button"
                  onClick={() => setDrawerOpen(false)}
                  aria-label={t('admin.closeNav', 'Close admin navigation')}
                  className="p-1.5 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800"
                >
                  <X weight="bold" className="w-4 h-4" />
                </button>
              </div>
              <AdminSidebar sections={sections} onNavigate={() => setDrawerOpen(false)} />
            </motion.aside>
          </motion.div>
        )}

        {/* Content */}
        <main className="min-w-0 flex-1 pt-2 pb-20 overflow-x-hidden">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
