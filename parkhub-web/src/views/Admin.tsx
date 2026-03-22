import { Outlet, Link, useLocation } from 'react-router-dom';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  ChartBar, GearSix, Users, Megaphone, ChartLine, MapPin, Translate, PresentationChart, Gauge, Buildings,
} from '@phosphor-icons/react';

function AdminNav() {
  const { t } = useTranslation();
  const location = useLocation();

  const tabs = [
    { name: t('admin.overview'), path: '/admin', icon: ChartBar },
    { name: t('admin.settings'), path: '/admin/settings', icon: GearSix },
    { name: t('admin.users'), path: '/admin/users', icon: Users },
    { name: t('admin.lots'), path: '/admin/lots', icon: MapPin },
    { name: t('admin.announcements'), path: '/admin/announcements', icon: Megaphone },
    { name: t('admin.reports'), path: '/admin/reports', icon: ChartLine },
    { name: t('admin.translations'), path: '/admin/translations', icon: Translate },
    { name: 'Analytics', path: '/admin/analytics', icon: PresentationChart },
    { name: t('admin.rateLimits', 'Rate Limits'), path: '/admin/rate-limits', icon: Gauge },
    { name: t('admin.tenants', 'Tenants'), path: '/admin/tenants', icon: Buildings },
  ];

  function isActive(path: string) {
    if (path === '/admin') return location.pathname === '/admin';
    return location.pathname.startsWith(path);
  }

  return (
    <nav aria-label="Admin navigation" className="flex gap-1 overflow-x-auto pb-1 scrollbar-hide -webkit-overflow-scrolling-touch">
      {tabs.map(tab => {
        const active = isActive(tab.path);
        return (
          <Link
            key={tab.path}
            to={tab.path}
            aria-current={active ? 'page' : undefined}
            className={`relative flex items-center gap-2 px-4 py-2.5 rounded-xl text-sm font-medium whitespace-nowrap transition-colors ${
              active
                ? 'text-primary-600 dark:text-primary-400 bg-primary-50 dark:bg-primary-900/20'
                : 'text-surface-500 dark:text-surface-400 hover:text-surface-900 dark:hover:text-white hover:bg-surface-100 dark:hover:bg-surface-800'
            }`}
          >
            <tab.icon weight={active ? 'fill' : 'regular'} className="w-4.5 h-4.5" />
            {tab.name}
            {active && (
              <motion.div
                layoutId="admin-tab-indicator"
                className="absolute bottom-0 left-3 right-3 h-0.5 bg-primary-500 rounded-full"
                transition={{ type: 'spring', stiffness: 500, damping: 30 }}
              />
            )}
          </Link>
        );
      })}
    </nav>
  );
}

export function AdminPage() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-surface-900 dark:text-white">{t('admin.title')}</h1>
        <p className="text-surface-500 dark:text-surface-400 mt-1">{t('admin.subtitle')}</p>
      </div>

      {/* Tab navigation */}
      <div className="relative">
        <AdminNav />
        <div className="absolute right-0 top-0 bottom-0 w-8 bg-gradient-to-l from-surface-50 dark:from-surface-950 to-transparent pointer-events-none sm:hidden" />
      </div>

      {/* Divider */}
      <div className="border-t border-surface-200 dark:border-surface-700" />

      {/* Content */}
      <Outlet />
    </div>
  );
}
