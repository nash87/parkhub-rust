import { Outlet, Link, useLocation } from 'react-router-dom';
import { motion } from 'framer-motion';
import {
  ChartBar, GearSix, Users, Megaphone, ChartLine,
} from '@phosphor-icons/react';

const tabs = [
  { name: 'Overview', path: '/admin', icon: ChartBar },
  { name: 'Settings', path: '/admin/settings', icon: GearSix },
  { name: 'Users', path: '/admin/users', icon: Users },
  { name: 'Announcements', path: '/admin/announcements', icon: Megaphone },
  { name: 'Reports', path: '/admin/reports', icon: ChartLine },
];

function AdminNav() {
  const location = useLocation();

  function isActive(path: string) {
    if (path === '/admin') return location.pathname === '/admin';
    return location.pathname.startsWith(path);
  }

  return (
    <nav className="flex gap-1 overflow-x-auto pb-1 scrollbar-hide">
      {tabs.map(tab => {
        const active = isActive(tab.path);
        return (
          <Link
            key={tab.path}
            to={tab.path}
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
  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-surface-900 dark:text-white">Admin</h1>
        <p className="text-surface-500 dark:text-surface-400 mt-1">Manage your ParkHub instance</p>
      </div>

      {/* Tab navigation */}
      <AdminNav />

      {/* Divider */}
      <div className="border-t border-surface-200 dark:border-surface-700" />

      {/* Content */}
      <Outlet />
    </div>
  );
}
