import { useState, useCallback, useEffect } from 'react';
import { Outlet, NavLink, useNavigate, useLocation } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  House, CalendarCheck, Car, Calendar, CalendarX, Coins, UserCircle, Users, Bell,
  GearSix, SignOut, List, X, CarSimple, SunDim, Moon, Translate, Star,
} from '@phosphor-icons/react';
import { useAuth } from '../context/AuthContext';
import { useTheme } from '../context/ThemeContext';
import { useKeyboardShortcuts } from '../hooks/useKeyboardShortcuts';
import { CommandPalette } from './CommandPalette';
import { ThemeSwitcher, ThemeSwitcherFab } from './ThemeSwitcher';
import { Breadcrumb } from './ui/Breadcrumb';
import { NotificationBadge } from './ui/NotificationBadge';

const NAV_ITEMS = [
  { to: '/', icon: House, key: 'dashboard', end: true },
  { to: '/bookings', icon: CalendarCheck, key: 'bookings' },
  { to: '/vehicles', icon: Car, key: 'vehicles' },
  { to: '/favorites', icon: Star, key: 'favorites' },
  { to: '/absences', icon: CalendarX, key: 'absences' },
  { to: '/team', icon: Users, key: 'team' },
  { to: '/calendar', icon: Calendar, key: 'calendar' },
  { to: '/credits', icon: Coins, key: 'credits' },
  { to: '/notifications', icon: Bell, key: 'notifications' },
  { to: '/translations', icon: Translate, key: 'translations' },
  { to: '/profile', icon: UserCircle, key: 'profile' },
] as const;

export function Layout() {
  const { t } = useTranslation();
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();
  const { resolved, setTheme } = useTheme();
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [themeSwitcherOpen, setThemeSwitcherOpen] = useState(false);
  const [unreadCount, setUnreadCount] = useState(0);

  const toggleCommandPalette = useCallback(
    () => setCommandPaletteOpen(prev => !prev),
    [],
  );

  useKeyboardShortcuts({ onToggleCommandPalette: toggleCommandPalette });

  useEffect(() => {
    fetch('/api/v1/notifications/unread-count')
      .then(r => r.json())
      .then(res => { if (res?.data?.count !== undefined) setUnreadCount(res.data.count); })
      .catch(() => {});
  }, [location.pathname]);

  function handleLogout() {
    logout();
    navigate('/welcome');
  }

  return (
    <div className="min-h-dvh bg-surface-50 dark:bg-surface-950 flex">
      <a href="#main-content" className="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:px-4 focus:py-2 focus:bg-primary-600 focus:text-white focus:rounded-lg">{t('nav.skipToContent')}</a>

      {/* Sidebar — desktop — glass morphism */}
      <aside className="hidden lg:flex flex-col w-64 bg-white/70 dark:bg-surface-900/70 backdrop-blur-2xl border-r border-surface-200/40 dark:border-surface-800/40 p-4 sticky top-0 h-dvh" aria-label="Main navigation">
        <div className="flex items-center gap-3 px-3 mb-8">
          <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-primary-600 to-primary-500 flex items-center justify-center shadow-lg shadow-primary-500/20">
            <CarSimple weight="fill" className="w-5 h-5 text-white" />
          </div>
          <span className="text-xl font-bold text-surface-900 dark:text-white" style={{ letterSpacing: '-0.02em' }}>ParkHub</span>
        </div>

        <nav className="flex-1 space-y-0.5">
          {NAV_ITEMS.map(item => (
            <NavLink
              key={item.key}
              to={item.to}
              end={item.end}
              aria-current={location.pathname === item.to || (item.to !== '/' && location.pathname.startsWith(item.to)) ? 'page' : undefined}
              className={({ isActive }) =>
                `relative flex items-center gap-3 px-3 py-2 text-sm font-medium rounded-lg transition-all ${
                  isActive
                    ? 'text-primary-700 dark:text-primary-300 bg-primary-50/80 dark:bg-primary-950/30'
                    : 'text-surface-600 dark:text-surface-400 hover:text-surface-900 dark:hover:text-white hover:bg-surface-100/60 dark:hover:bg-surface-800/40'
                }`
              }
            >
              {({ isActive }) => (
                <>
                  {isActive && (
                    <motion.div
                      layoutId="nav-indicator"
                      className="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-[60%] rounded-full bg-gradient-to-b from-primary-500 to-primary-400"
                      transition={{ type: 'spring', stiffness: 380, damping: 30 }}
                    />
                  )}
                  <span className="relative">
                    <item.icon weight="fill" className="w-5 h-5" />
                    {item.key === 'notifications' && <NotificationBadge count={unreadCount} />}
                  </span>
                  {t(`nav.${item.key}`)}
                </>
              )}
            </NavLink>
          ))}

          {user?.role && ['admin', 'superadmin'].includes(user.role) && (
            <NavLink
              to="/admin"
              className={({ isActive }) =>
                `relative flex items-center gap-3 px-3 py-2 text-sm font-medium rounded-lg transition-all ${
                  isActive
                    ? 'text-primary-700 dark:text-primary-300 bg-primary-50/80 dark:bg-primary-950/30'
                    : 'text-surface-600 dark:text-surface-400 hover:text-surface-900 dark:hover:text-white hover:bg-surface-100/60 dark:hover:bg-surface-800/40'
                }`
              }
            >
              <GearSix weight="fill" className="w-5 h-5" />
              {t('nav.admin')}
            </NavLink>
          )}
        </nav>

        <button
          onClick={() => setTheme(resolved === 'dark' ? 'light' : 'dark')}
          className="flex items-center gap-3 px-3 py-2 text-sm font-medium text-surface-600 dark:text-surface-400 hover:text-surface-900 dark:hover:text-white rounded-lg hover:bg-surface-100/60 dark:hover:bg-surface-800/40 transition-all mb-2"
        >
          {resolved === 'dark' ? <SunDim weight="fill" className="w-5 h-5" /> : <Moon weight="fill" className="w-5 h-5" />}
          {resolved === 'dark' ? t('nav.lightMode') : t('nav.darkMode')}
        </button>

        <div className="border-t border-surface-200/60 dark:border-surface-800/60 pt-4 mt-2">
          <div className="flex items-center gap-3 px-3 mb-3">
            <div className="relative">
              <div className="w-9 h-9 rounded-full bg-gradient-to-br from-primary-200 to-primary-100 dark:from-primary-800 dark:to-primary-900 flex items-center justify-center ring-2 ring-primary-500/20 dark:ring-primary-400/20">
                <span className="text-sm font-bold text-primary-700 dark:text-primary-300">
                  {(user?.name || user?.username || 'U').charAt(0).toUpperCase()}
                </span>
              </div>
              <span className="absolute -bottom-0.5 -right-0.5 w-3 h-3 rounded-full bg-emerald-500 border-2 border-white dark:border-surface-900" />
            </div>
            <div className="min-w-0">
              <p className="text-sm font-medium text-surface-900 dark:text-white truncate">{user?.name || user?.username}</p>
              <p className="text-xs text-surface-500 dark:text-surface-400 truncate">{user?.email}</p>
            </div>
          </div>
          <button
            onClick={handleLogout}
            className="flex items-center gap-3 px-3 py-2 text-sm font-medium text-red-600 hover:text-red-700 dark:hover:text-red-400 hover:bg-red-50/60 dark:hover:bg-red-950/20 rounded-lg transition-all w-full"
          >
            <SignOut weight="bold" className="w-5 h-5" />
            {t('nav.logout')}
          </button>
        </div>
      </aside>

      <div className="flex-1 flex flex-col">
        <header className="lg:hidden sticky top-0 z-30 bg-white/70 dark:bg-surface-900/70 backdrop-blur-2xl border-b border-surface-200/40 dark:border-surface-800/40 px-4 py-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <button onClick={() => setSidebarOpen(true)} className="btn btn-ghost btn-icon min-w-[44px] min-h-[44px]" aria-label={t('nav.openMenu')}>
                <List weight="bold" className="w-5 h-5" />
              </button>
              <div className="flex items-center gap-2">
                <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-primary-600 to-primary-500 flex items-center justify-center shadow-sm">
                  <CarSimple weight="fill" className="w-4 h-4 text-white" />
                </div>
                <span className="font-bold text-surface-900 dark:text-white">ParkHub</span>
              </div>
            </div>
            <button
              onClick={() => setTheme(resolved === 'dark' ? 'light' : 'dark')}
              className="btn btn-ghost btn-icon"
              aria-label={resolved === 'dark' ? t('nav.switchToLight') : t('nav.switchToDark')}
            >
              {resolved === 'dark' ? <SunDim weight="fill" className="w-5 h-5" /> : <Moon weight="fill" className="w-5 h-5" />}
            </button>
          </div>
        </header>

        <AnimatePresence>
          {sidebarOpen && (
            <>
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="fixed inset-0 bg-black/40 backdrop-blur-sm z-40 lg:hidden"
                onClick={() => setSidebarOpen(false)}
                aria-hidden="true"
              />
              <motion.aside
                initial={{ x: '-100%' }}
                animate={{ x: 0 }}
                exit={{ x: '-100%' }}
                transition={{ type: 'spring', damping: 25, stiffness: 300 }}
                drag="x"
                dragConstraints={{ left: -288, right: 0 }}
                dragElastic={0.1}
                onDragEnd={(_e, info) => {
                  if (info.offset.x < -80 || info.velocity.x < -300) setSidebarOpen(false);
                }}
                className="fixed inset-y-0 left-0 w-72 bg-white/90 dark:bg-surface-900/90 backdrop-blur-2xl z-50 p-4 lg:hidden touch-pan-y"
                role="dialog"
                aria-label="Navigation menu"
              >
                <div className="flex items-center justify-between mb-6">
                  <div className="flex items-center gap-2">
                    <div className="w-10 h-10 rounded-xl bg-gradient-to-br from-primary-600 to-primary-500 flex items-center justify-center shadow-lg shadow-primary-500/20">
                      <CarSimple weight="fill" className="w-5 h-5 text-white" />
                    </div>
                    <span className="text-xl font-bold text-surface-900 dark:text-white">ParkHub</span>
                  </div>
                  <button onClick={() => setSidebarOpen(false)} className="btn btn-ghost btn-icon" aria-label={t('nav.closeMenu')}>
                    <X weight="bold" className="w-5 h-5" />
                  </button>
                </div>
                <nav className="space-y-0.5">
                  {NAV_ITEMS.map(item => (
                    <NavLink
                      key={item.key}
                      to={item.to}
                      end={item.end}
                      onClick={() => setSidebarOpen(false)}
                      className={({ isActive }) =>
                        `flex items-center gap-3 px-3 py-2.5 text-sm font-medium rounded-lg transition-all ${
                          isActive
                            ? 'text-primary-700 dark:text-primary-300 bg-primary-50/80 dark:bg-primary-950/30'
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
                        `flex items-center gap-3 px-3 py-2.5 text-sm font-medium rounded-lg transition-all ${
                          isActive
                            ? 'text-primary-700 dark:text-primary-300 bg-primary-50/80 dark:bg-primary-950/30'
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
                  <button onClick={handleLogout} className="flex items-center gap-3 px-3 py-2.5 text-sm font-medium text-red-600 w-full rounded-lg hover:bg-red-50/60 dark:hover:bg-red-950/20 transition-all">
                    <SignOut weight="bold" className="w-5 h-5" /> {t('nav.logout')}
                  </button>
                </div>
              </motion.aside>
            </>
          )}
        </AnimatePresence>

        <main id="main-content" className="flex-1 p-4 sm:p-6 lg:p-8 max-w-6xl mx-auto w-full">
          <Breadcrumb />
          <Outlet />
        </main>
      </div>

      <CommandPalette open={commandPaletteOpen} onClose={() => setCommandPaletteOpen(false)} />
      <ThemeSwitcherFab onClick={() => setThemeSwitcherOpen(true)} />
      <ThemeSwitcher open={themeSwitcherOpen} onClose={() => setThemeSwitcherOpen(false)} />
    </div>
  );
}
