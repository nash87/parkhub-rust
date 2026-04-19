/**
 * Top tabs — horizontal navigation strip across the top of the viewport.
 * Ported from the claude.ai/design v4 nav-variants bundle. Same footprint
 * as the mobile header but wider and used on desktop.
 *
 * UX notes:
 *  - Too many items for a single row, so we curate 6 "core + favourites"
 *    and push everything else behind a More dropdown.
 *  - Animated underline indicator mirrors the Classic sidebar's side
 *    accent bar — framer-motion layoutId keeps motion continuous.
 *  - On viewport < lg, the top-tabs layout still uses the mobile drawer
 *    (same as Classic) so this component only has to reason about
 *    desktop sizing.
 */
import { useRef, useState, useEffect } from 'react';
import { NavLink, useLocation } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  CarSimple, CaretDown, GearSix, SignOut, SunDim, Moon,
} from '@phosphor-icons/react';
import { useAuth } from '../../context/AuthContext';
import { useTheme } from '../../context/ThemeContext';
import { NotificationCenter } from '../NotificationCenter';
import { NotificationBadge } from '../ui/NotificationBadge';
import { preloadRoute } from '../../lib/routePreload';
import { NAV_SECTIONS, type NavItem } from '../Layout';
import { isActivePath } from './navActive';

interface TopTabsProps {
  unreadCount: number;
  onLogout: () => void;
  isAdmin: boolean;
}

const PRIMARY_KEYS = [
  'dashboard', 'bookings', 'bookSpot', 'calendar', 'vehicles', 'favorites',
];

export function TopTabs({ unreadCount, onLogout, isAdmin }: TopTabsProps) {
  const { t } = useTranslation();
  const { user } = useAuth();
  const { resolved, setTheme } = useTheme();
  const location = useLocation();

  const allItems: NavItem[] = NAV_SECTIONS.flatMap(s => s.items);
  const primary = PRIMARY_KEYS
    .map(k => allItems.find(i => i.key === k))
    .filter((i): i is NavItem => !!i);
  const primarySet = new Set(primary.map(i => i.key));
  const overflow = allItems.filter(i => !primarySet.has(i.key));

  return (
    <header
      aria-label="Main navigation"
      className="hidden lg:flex items-center gap-2 h-14 px-6 sticky top-0 z-30 bg-white/75 dark:bg-surface-900/75 backdrop-blur-2xl border-b border-surface-200/40 dark:border-surface-800/40"
    >
      {/* Brand */}
      <NavLink to="/" className="flex items-center gap-2 mr-4">
        <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-primary-600 to-primary-500 flex items-center justify-center shadow-sm">
          <CarSimple weight="fill" className="w-4 h-4 text-white" />
        </div>
        <span className="text-base font-bold text-surface-900 dark:text-white" style={{ letterSpacing: '-0.02em' }}>
          ParkHub
        </span>
      </NavLink>

      <nav className="flex items-center gap-1 flex-1 min-w-0 overflow-x-auto">
        {primary.map(item => {
          const isActive = isActivePath(location.pathname, item.to);
          return (
            <NavLink
              key={item.key}
              to={item.to}
              end={item.end}
              onMouseEnter={() => preloadRoute(item.to)}
              onFocus={() => preloadRoute(item.to)}
              aria-current={isActive ? 'page' : undefined}
              className={`relative flex items-center gap-1.5 px-3 h-full text-sm font-medium transition-colors whitespace-nowrap ${
                isActive
                  ? 'text-primary-700 dark:text-primary-300'
                  : 'text-surface-600 dark:text-surface-400 hover:text-surface-900 dark:hover:text-white'
              }`}
            >
              {isActive && (
                <motion.span
                  layoutId="top-tab-indicator"
                  aria-hidden="true"
                  className="absolute bottom-0 left-0 right-0 h-[2px] bg-gradient-to-r from-primary-500 to-primary-400 rounded-full"
                  transition={{ type: 'spring', stiffness: 380, damping: 30 }}
                />
              )}
              <item.icon weight="fill" className="w-4 h-4" />
              {t(`nav.${item.key}`)}
            </NavLink>
          );
        })}

        <OverflowDropdown items={overflow} unreadCount={unreadCount} isAdmin={isAdmin} />
      </nav>

      <div className="flex items-center gap-1.5 ml-2">
        <NotificationCenter />
        <button
          onClick={() => setTheme(resolved === 'dark' ? 'light' : 'dark')}
          className="flex items-center justify-center w-9 h-9 rounded-lg text-surface-500 hover:text-surface-900 dark:hover:text-white hover:bg-surface-100/60 dark:hover:bg-surface-800/40 transition-all"
          aria-label={resolved === 'dark' ? t('nav.lightMode') : t('nav.darkMode')}
          title={resolved === 'dark' ? t('nav.lightMode') : t('nav.darkMode')}
        >
          {resolved === 'dark' ? <SunDim weight="fill" className="w-4 h-4" /> : <Moon weight="fill" className="w-4 h-4" />}
        </button>
        <UserMenu user={user} onLogout={onLogout} />
      </div>
    </header>
  );
}

function OverflowDropdown({ items, unreadCount, isAdmin }: { items: NavItem[]; unreadCount: number; isAdmin: boolean }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function onDocClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener('mousedown', onDocClick);
    return () => document.removeEventListener('mousedown', onDocClick);
  }, []);

  return (
    <div ref={ref} className="relative">
      <button
        type="button"
        onClick={() => setOpen(v => !v)}
        aria-expanded={open}
        aria-haspopup="menu"
        className={`flex items-center gap-1.5 px-3 h-full text-sm font-medium transition-colors ${
          open
            ? 'text-primary-700 dark:text-primary-300'
            : 'text-surface-600 dark:text-surface-400 hover:text-surface-900 dark:hover:text-white'
        }`}
      >
        {t('nav.more', 'More')}
        <CaretDown weight="bold" className={`w-3 h-3 transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>

      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ opacity: 0, y: -4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -4 }}
            transition={{ duration: 0.12 }}
            role="menu"
            className="absolute top-full left-0 mt-1 w-[280px] p-1 rounded-xl bg-white dark:bg-surface-800 border border-surface-200 dark:border-surface-700 shadow-xl z-50 max-h-[70vh] overflow-y-auto"
          >
            {items.map(item => (
              <NavLink
                key={item.key}
                to={item.to}
                end={item.end}
                onClick={() => setOpen(false)}
                onMouseEnter={() => preloadRoute(item.to)}
                className={({ isActive }) =>
                  `flex items-center gap-2.5 px-2.5 py-2 rounded-lg text-sm transition-colors ${
                    isActive
                      ? 'text-primary-700 dark:text-primary-300 bg-primary-50 dark:bg-primary-950/30'
                      : 'text-surface-700 dark:text-surface-300 hover:bg-surface-100 dark:hover:bg-surface-700/50'
                  }`
                }
              >
                <span className="relative">
                  <item.icon weight="fill" className="w-4 h-4" />
                  {item.key === 'notifications' && unreadCount > 0 && <NotificationBadge count={unreadCount} />}
                </span>
                {t(`nav.${item.key}`)}
              </NavLink>
            ))}
            {isAdmin && (
              <>
                <div className="my-1 h-px bg-surface-200 dark:bg-surface-700" />
                <NavLink
                  to="/admin"
                  onClick={() => setOpen(false)}
                  className={({ isActive }) =>
                    `flex items-center gap-2.5 px-2.5 py-2 rounded-lg text-sm transition-colors ${
                      isActive
                        ? 'text-primary-700 dark:text-primary-300 bg-primary-50 dark:bg-primary-950/30'
                        : 'text-surface-700 dark:text-surface-300 hover:bg-surface-100 dark:hover:bg-surface-700/50'
                    }`
                  }
                >
                  <GearSix weight="fill" className="w-4 h-4" />
                  {t('nav.admin')}
                </NavLink>
              </>
            )}
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

function UserMenu({ user, onLogout }: { user: { name?: string; username?: string; email?: string } | null; onLogout: () => void }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function onDocClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener('mousedown', onDocClick);
    return () => document.removeEventListener('mousedown', onDocClick);
  }, []);

  const initial = (user?.name || user?.username || 'U').charAt(0).toUpperCase();

  return (
    <div ref={ref} className="relative">
      <button
        onClick={() => setOpen(v => !v)}
        aria-expanded={open}
        className="relative flex items-center justify-center w-9 h-9 rounded-full bg-gradient-to-br from-primary-200 to-primary-100 dark:from-primary-800 dark:to-primary-900 ring-2 ring-primary-500/20 dark:ring-primary-400/20 hover:scale-105 transition-transform"
      >
        <span className="text-sm font-bold text-primary-700 dark:text-primary-300">{initial}</span>
        <span className="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-emerald-500 border-2 border-white dark:border-surface-900" />
      </button>

      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ opacity: 0, y: -4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -4 }}
            transition={{ duration: 0.12 }}
            role="menu"
            className="absolute right-0 top-full mt-1 w-[240px] rounded-xl bg-white dark:bg-surface-800 border border-surface-200 dark:border-surface-700 shadow-xl z-50 overflow-hidden"
          >
            <div className="p-3 border-b border-surface-200 dark:border-surface-700">
              <p className="text-sm font-medium text-surface-900 dark:text-white truncate">{user?.name || user?.username}</p>
              <p className="text-xs text-surface-500 dark:text-surface-400 truncate">{user?.email}</p>
            </div>
            <button
              onClick={() => { setOpen(false); onLogout(); }}
              className="flex items-center gap-2.5 w-full px-3 py-2.5 text-sm text-red-600 hover:bg-red-50/60 dark:hover:bg-red-950/20 transition-colors"
            >
              <SignOut weight="bold" className="w-4 h-4" />
              {t('nav.logout')}
            </button>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
