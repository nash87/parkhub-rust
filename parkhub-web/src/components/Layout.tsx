import { useState } from 'react';
import { Outlet, NavLink, useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  House, CalendarCheck, Car, Calendar, CalendarX, Coins, UserCircle, Users, Bell,
  GearSix, SignOut, List, X, CarSimple, SunDim, Moon,
} from '@phosphor-icons/react';
import { useAuth } from '../context/AuthContext';
import { useTheme } from '../context/ThemeContext';

const NAV_ITEMS = [
  { to: '/', icon: House, key: 'dashboard', end: true },
  { to: '/bookings', icon: CalendarCheck, key: 'bookings' },
  { to: '/vehicles', icon: Car, key: 'vehicles' },
  { to: '/absences', icon: CalendarX, key: 'absences' },
  { to: '/team', icon: Users, key: 'team' },
  { to: '/calendar', icon: Calendar, key: 'calendar' },
  { to: '/credits', icon: Coins, key: 'credits' },
  { to: '/notifications', icon: Bell, key: 'notifications' },
  { to: '/profile', icon: UserCircle, key: 'profile' },
] as const;

export function Layout() {
  const { t } = useTranslation();
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const { resolved, setTheme } = useTheme();
  const [sidebarOpen, setSidebarOpen] = useState(false);

  function handleLogout() {
    logout();
    navigate('/welcome');
  }

  return (
    <div className="min-h-dvh bg-surface-50 dark:bg-surface-950 flex">
      {/* Skip to content */}
      <a href="#main-content" className="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-primary-600 focus:text-white focus:rounded-lg">Skip to content</a>

      {/* Sidebar — desktop */}
      <aside className="hidden lg:flex flex-col w-64 bg-white dark:bg-surface-900 border-r border-surface-200 dark:border-surface-800 p-4 sticky top-0 h-dvh">
        {/* Logo */}
        <div className="flex items-center gap-3 px-3 mb-8">
          <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-primary-500 to-primary-600 flex items-center justify-center shadow-md shadow-primary-500/20">
            <CarSimple weight="fill" className="w-5 h-5 text-white" />
          </div>
          <span className="text-xl font-bold text-surface-900 dark:text-white tracking-tight">ParkHub</span>
        </div>

        {/* Nav */}
        <nav className="flex-1 space-y-1">
          {NAV_ITEMS.map(item => (
            <NavLink
              key={item.key}
              to={item.to}
              end={item.end}
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm font-medium transition-all ${
                  isActive
                    ? 'bg-primary-50 dark:bg-primary-900/20 text-primary-700 dark:text-primary-300'
                    : 'text-surface-600 dark:text-surface-400 hover:bg-surface-50 dark:hover:bg-surface-800'
                }`
              }
            >
              <item.icon weight="fill" className="w-5 h-5" />
              {t(`nav.${item.key}`)}
            </NavLink>
          ))}

          {user?.role && ['admin', 'superadmin'].includes(user.role) && (
            <NavLink
              to="/admin"
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm font-medium transition-all ${
                  isActive
                    ? 'bg-primary-50 dark:bg-primary-900/20 text-primary-700 dark:text-primary-300'
                    : 'text-surface-600 dark:text-surface-400 hover:bg-surface-50 dark:hover:bg-surface-800'
                }`
              }
            >
              <GearSix weight="fill" className="w-5 h-5" />
              {t('nav.admin')}
            </NavLink>
          )}
        </nav>

        {/* Theme toggle */}
        <button
          onClick={() => setTheme(resolved === 'dark' ? 'light' : 'dark')}
          className="flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm font-medium text-surface-600 dark:text-surface-400 hover:bg-surface-50 dark:hover:bg-surface-800 transition-all mb-2"
        >
          {resolved === 'dark' ? <SunDim weight="fill" className="w-5 h-5" /> : <Moon weight="fill" className="w-5 h-5" />}
          {resolved === 'dark' ? 'Light Mode' : 'Dark Mode'}
        </button>

        {/* User + logout */}
        <div className="border-t border-surface-200 dark:border-surface-800 pt-4 mt-2">
          <div className="flex items-center gap-3 px-3 mb-3">
            <div className="w-9 h-9 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center">
              <span className="text-sm font-bold text-primary-700 dark:text-primary-300">
                {(user?.name || user?.username || 'U').charAt(0).toUpperCase()}
              </span>
            </div>
            <div className="min-w-0">
              <p className="text-sm font-medium text-surface-900 dark:text-white truncate">{user?.name || user?.username}</p>
              <p className="text-xs text-surface-500 dark:text-surface-400 truncate">{user?.email}</p>
            </div>
          </div>
          <button
            onClick={handleLogout}
            className="flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm font-medium text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20 transition-all w-full"
          >
            <SignOut weight="bold" className="w-5 h-5" />
            {t('nav.logout')}
          </button>
        </div>
      </aside>

      {/* Mobile top bar */}
      <div className="flex-1 flex flex-col">
        <header className="lg:hidden sticky top-0 z-30 bg-white/80 dark:bg-surface-900/80 backdrop-blur-lg border-b border-surface-200 dark:border-surface-800 px-4 py-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <button onClick={() => setSidebarOpen(true)} className="btn btn-ghost btn-icon" aria-label="Open navigation menu">
                <List weight="bold" className="w-5 h-5" />
              </button>
              <div className="flex items-center gap-2">
                <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-primary-500 to-primary-600 flex items-center justify-center">
                  <CarSimple weight="fill" className="w-4 h-4 text-white" />
                </div>
                <span className="font-bold text-surface-900 dark:text-white">ParkHub</span>
              </div>
            </div>
            <button
              onClick={() => setTheme(resolved === 'dark' ? 'light' : 'dark')}
              className="btn btn-ghost btn-icon"
              aria-label={resolved === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'}
            >
              {resolved === 'dark' ? <SunDim weight="fill" className="w-5 h-5" /> : <Moon weight="fill" className="w-5 h-5" />}
            </button>
          </div>
        </header>

        {/* Mobile sidebar overlay */}
        <AnimatePresence>
          {sidebarOpen && (
            <>
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="fixed inset-0 bg-black/40 z-40 lg:hidden"
                onClick={() => setSidebarOpen(false)}
              />
              <motion.aside
                initial={{ x: '-100%' }}
                animate={{ x: 0 }}
                exit={{ x: '-100%' }}
                transition={{ type: 'spring', damping: 25, stiffness: 300 }}
                className="fixed inset-y-0 left-0 w-72 bg-white dark:bg-surface-900 z-50 p-4 lg:hidden"
                role="dialog"
                aria-label="Navigation menu"
              >
                <div className="flex items-center justify-between mb-6">
                  <div className="flex items-center gap-2">
                    <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-primary-500 to-primary-600 flex items-center justify-center">
                      <CarSimple weight="fill" className="w-5 h-5 text-white" />
                    </div>
                    <span className="text-xl font-bold text-surface-900 dark:text-white">ParkHub</span>
                  </div>
                  <button onClick={() => setSidebarOpen(false)} className="btn btn-ghost btn-icon" aria-label="Close navigation menu">
                    <X weight="bold" className="w-5 h-5" />
                  </button>
                </div>
                <nav className="space-y-1">
                  {NAV_ITEMS.map(item => (
                    <NavLink
                      key={item.key}
                      to={item.to}
                      end={item.end}
                      onClick={() => setSidebarOpen(false)}
                      className={({ isActive }) =>
                        `flex items-center gap-3 px-3 py-3 rounded-xl text-sm font-medium transition-all ${
                          isActive
                            ? 'bg-primary-50 dark:bg-primary-900/20 text-primary-700 dark:text-primary-300'
                            : 'text-surface-600 dark:text-surface-400'
                        }`
                      }
                    >
                      <item.icon weight="fill" className="w-5 h-5" />
                      {t(`nav.${item.key}`)}
                    </NavLink>
                  ))}

                  {user?.role && ['admin', 'superadmin'].includes(user.role) && (
                    <NavLink
                      to="/admin"
                      onClick={() => setSidebarOpen(false)}
                      className={({ isActive }) =>
                        `flex items-center gap-3 px-3 py-3 rounded-xl text-sm font-medium transition-all ${
                          isActive
                            ? 'bg-primary-50 dark:bg-primary-900/20 text-primary-700 dark:text-primary-300'
                            : 'text-surface-600 dark:text-surface-400'
                        }`
                      }
                    >
                      <GearSix weight="fill" className="w-5 h-5" />
                      {t('nav.admin')}
                    </NavLink>
                  )}
                </nav>
                <div className="absolute bottom-4 left-4 right-4">
                  <button onClick={handleLogout} className="flex items-center gap-3 px-3 py-3 rounded-xl text-sm font-medium text-red-600 w-full">
                    <SignOut weight="bold" className="w-5 h-5" /> {t('nav.logout')}
                  </button>
                </div>
              </motion.aside>
            </>
          )}
        </AnimatePresence>

        {/* Main content */}
        <main id="main-content" className="flex-1 p-4 sm:p-6 lg:p-8 max-w-6xl mx-auto w-full">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
