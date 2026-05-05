import { Outlet, Link, useLocation } from 'react-router-dom';
import { useState, type ComponentType } from 'react';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  ChartBarIcon, GearSixIcon, UsersIcon, MegaphoneIcon, ChartLineIcon, MapPinIcon, TranslateIcon, PresentationChartIcon, GaugeIcon,
  BuildingsIcon, ClockCounterClockwiseIcon, DatabaseIcon, CarIcon, WheelchairIcon, WrenchIcon, CurrencyDollarIcon, UserPlusIcon, LightningIcon,
  PuzzlePieceIcon, GraphicsCardIcon, ShieldCheckIcon, LockKeyIcon, MapTrifoldIcon, ArrowsClockwiseIcon, ListIcon, XIcon, ArrowSquareOutIcon,
} from '@phosphor-icons/react';

type AdminIcon = ComponentType<{ className?: string; weight?: 'regular' | 'fill' | 'bold' | 'duotone' }>;

type NavItem = {
  key: string;
  label: string;
  to: string;
  icon: AdminIcon;
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
        { key: 'overview', label: t('admin.overview', 'Overview'), to: '/admin', icon: ChartBarIcon },
        { key: 'analytics', label: t('admin.analytics', 'Analytics'), to: '/admin/analytics', icon: PresentationChartIcon },
        { key: 'audit', label: t('admin.auditLog', 'Audit Log'), to: '/admin/audit-log', icon: ClockCounterClockwiseIcon },
      ],
    },
    {
      key: 'operations',
      label: t('admin.group.operations', 'Operations'),
      items: [
        { key: 'lots', label: t('admin.lots', 'Lots'), to: '/admin/lots', icon: MapPinIcon },
        { key: 'zones', label: t('parkingZones.title', 'Zones'), to: '/admin/zones', icon: MapTrifoldIcon },
        { key: 'fleet', label: t('admin.fleet', 'Fleet'), to: '/admin/fleet', icon: CarIcon },
        { key: 'chargers', label: t('admin.chargers', 'EV Chargers'), to: '/admin/chargers', icon: LightningIcon },
        { key: 'maintenance', label: t('admin.maintenance', 'Maintenance'), to: '/admin/maintenance', icon: WrenchIcon },
        { key: 'accessible', label: t('admin.accessible', 'Accessible'), to: '/admin/accessible', icon: WheelchairIcon },
        { key: 'visitors', label: t('admin.visitors', 'Visitors'), to: '/admin/visitors', icon: UserPlusIcon },
      ],
    },
    {
      key: 'people',
      label: t('admin.group.peopleAccess', 'People & Access'),
      items: [
        { key: 'users', label: t('admin.users', 'Users'), to: '/admin/users', icon: UsersIcon },
        { key: 'roles', label: t('rbac.title', 'Roles'), to: '/admin/roles', icon: LockKeyIcon },
        { key: 'tenants', label: t('admin.tenants', 'Tenants'), to: '/admin/tenants', icon: BuildingsIcon },
        { key: 'sso', label: t('admin.sso', 'SSO & SAML'), to: '/admin/sso', icon: ShieldCheckIcon },
      ],
    },
    {
      key: 'compliance',
      label: t('admin.group.complianceData', 'Compliance & Data'),
      items: [
        { key: 'compliance', label: t('compliance.title', 'Compliance'), to: '/admin/compliance', icon: ShieldCheckIcon },
        { key: 'data', label: t('admin.dataManagement', 'Data'), to: '/admin/data', icon: DatabaseIcon },
        { key: 'rateLimits', label: t('admin.rateLimits', 'Rate Limits'), to: '/admin/rate-limits', icon: GaugeIcon },
        { key: 'announcements', label: t('admin.announcements', 'Announcements'), to: '/admin/announcements', icon: MegaphoneIcon },
      ],
    },
    {
      key: 'billing',
      label: t('admin.group.billingReports', 'Billing & Reports'),
      items: [
        { key: 'billing', label: t('admin.billing', 'Billing'), to: '/admin/billing', icon: CurrencyDollarIcon },
        { key: 'reports', label: t('admin.reports', 'Reports'), to: '/admin/reports', icon: ChartLineIcon },
        { key: 'translations', label: t('admin.translations', 'Translations'), to: '/admin/translations', icon: TranslateIcon },
      ],
    },
    {
      key: 'platform',
      label: t('admin.group.platform', 'Platform'),
      items: [
        { key: 'settings', label: t('admin.settings', 'Settings'), to: '/admin/settings', icon: GearSixIcon },
        { key: 'modules', label: t('admin.modules.title', 'Modules & Features'), to: '/admin/modules', icon: PuzzlePieceIcon },
        { key: 'plugins', label: t('admin.plugins', 'Plugins'), to: '/admin/plugins', icon: PuzzlePieceIcon },
        { key: 'updates', label: t('nav.updates', 'Updates'), to: '/admin/updates', icon: ArrowsClockwiseIcon },
        { key: 'graphql', label: 'GraphQL Playground', to: '/api/v1/graphql/playground', icon: GraphicsCardIcon, external: true },
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
    <nav aria-label="Admin navigation" className="flex flex-col gap-5">
      {sections.map(section => (
        <div key={section.key}>
          <div className="mb-1.5 px-2 text-[11px] font-semibold uppercase tracking-wide text-surface-500 dark:text-surface-400">
            {section.label}
          </div>
          <ul className="flex flex-col gap-0.5">
            {section.items.map(item => {
              const active = !item.external && isActivePath(location.pathname, item.to);
              const Icon = item.icon;
              const linkClass = [
                'group flex min-h-9 items-center gap-2 rounded-md px-2.5 py-2 text-sm font-medium transition-colors',
                active
                  ? 'bg-primary-600 text-white shadow-sm'
                  : 'text-surface-600 hover:bg-surface-100 hover:text-surface-950 dark:text-surface-300 dark:hover:bg-surface-800 dark:hover:text-white',
              ].join(' ');
              const content = (
                <>
                  <Icon weight={active ? 'fill' : 'regular'} className="h-4 w-4 shrink-0" />
                  <span className="min-w-0 flex-1 truncate">{item.label}</span>
                  {item.external && <ArrowSquareOutIcon weight="regular" className="h-3.5 w-3.5 shrink-0 text-surface-400" />}
                </>
              );

              return (
                <li key={item.key}>
                  {item.external ? (
                    <a href={item.to} target="_blank" rel="noopener noreferrer" className={linkClass} onClick={onNavigate}>
                      {content}
                    </a>
                  ) : (
                    <Link to={item.to} className={linkClass} aria-current={active ? 'page' : undefined} onClick={onNavigate}>
                      {content}
                    </Link>
                  )}
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

  const currentLabel = (() => {
    for (const section of sections) {
      for (const item of section.items) {
        if (!item.external && isActivePath(location.pathname, item.to)) return item.label;
      }
    }
    return t('admin.title', 'Admin');
  })();

  return (
    <div className="mx-auto max-w-7xl">
      <div className="lg:hidden sticky top-0 z-30 -mx-4 mb-4 flex items-center gap-3 border-b border-surface-200 bg-surface-50/95 px-4 py-3 backdrop-blur dark:border-surface-800 dark:bg-surface-950/95">
        <button
          type="button"
          onClick={() => setDrawerOpen(true)}
          aria-label={t('admin.openNav', 'Open admin navigation')}
          className="rounded-md p-2 text-surface-700 hover:bg-surface-100 dark:text-surface-200 dark:hover:bg-surface-800"
        >
          <ListIcon weight="bold" className="h-5 w-5" />
        </button>
        <div className="min-w-0 flex-1">
          <div className="text-[11px] font-semibold uppercase tracking-wide text-surface-500 dark:text-surface-400">
            {t('admin.title', 'Admin')}
          </div>
          <div className="truncate text-sm font-semibold text-surface-950 dark:text-white">{currentLabel}</div>
        </div>
      </div>

      <header className="mb-5 hidden border-b border-surface-200 pb-4 dark:border-surface-800 lg:flex lg:items-end lg:justify-between lg:gap-6">
        <div className="min-w-0">
          <div className="text-[11px] font-semibold uppercase tracking-wide text-surface-500 dark:text-surface-400">
            {t('admin.title', 'Admin')}
          </div>
          <h1 className="mt-1 text-2xl font-semibold tracking-tight text-surface-950 dark:text-white">
            {currentLabel}
          </h1>
          <p className="mt-1 max-w-2xl text-sm text-surface-500 dark:text-surface-400">
            {t('admin.subtitle', 'Manage the ParkHub instance')}
          </p>
        </div>
      </header>

      <div className="flex gap-6">
        <aside className="hidden w-64 shrink-0 lg:block">
          <div className="sticky top-4 max-h-[calc(100dvh-2rem)] overflow-y-auto pr-2">
            <AdminSidebar sections={sections} />
          </div>
        </aside>

        {drawerOpen && (
          <motion.div
            className="fixed inset-0 z-50 lg:hidden"
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
              transition={{ type: 'spring', stiffness: 360, damping: 34 }}
              className="relative flex h-full w-[min(22rem,88vw)] flex-col bg-white shadow-xl dark:bg-surface-950"
            >
              <div className="flex items-center justify-between border-b border-surface-200 px-4 py-3 dark:border-surface-800">
                <div>
                  <div className="text-[11px] font-semibold uppercase tracking-wide text-surface-500 dark:text-surface-400">
                    {t('admin.title', 'Admin')}
                  </div>
                  <div className="text-sm font-semibold text-surface-950 dark:text-white">{currentLabel}</div>
                </div>
                <button
                  type="button"
                  onClick={() => setDrawerOpen(false)}
                  aria-label={t('admin.closeNav', 'Close admin navigation')}
                  className="rounded-md p-2 text-surface-600 hover:bg-surface-100 dark:text-surface-300 dark:hover:bg-surface-800"
                >
                  <XIcon weight="bold" className="h-5 w-5" />
                </button>
              </div>
              <div className="flex-1 overflow-y-auto px-3 py-4">
                <AdminSidebar sections={sections} onNavigate={() => setDrawerOpen(false)} />
              </div>
            </motion.aside>
          </motion.div>
        )}

        <main className="min-w-0 flex-1">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
