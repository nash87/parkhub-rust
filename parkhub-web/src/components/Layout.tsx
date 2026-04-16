import { useState, useCallback, useEffect, useRef, useMemo } from 'react';
import { Outlet, NavLink, useNavigate, useLocation } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  House, CalendarCheck, Car, Calendar, CalendarX, Coins, UserCircle, Users, Bell,
  GearSix, SignOut, List, X, CarSimple, SunDim, Moon, Translate, Star, Globe, CaretDown, MapPin,
  ClockCounterClockwise, Swap, QrCode, UserPlus, Trophy, Sparkle, CalendarPlus,
} from '@phosphor-icons/react';
import { useAuth } from '../context/AuthContext';
import { useTheme } from '../context/ThemeContext';
import { useKeyboardShortcuts } from '../hooks/useKeyboardShortcuts';
import { usePageTitle } from '../hooks/usePageTitle';
import { CommandPalette } from './CommandPalette';
import { NotificationCenter } from './NotificationCenter';
import { ThemeSwitcher, ThemeSwitcherFab } from './ThemeSwitcher';
import { Breadcrumb } from './ui/Breadcrumb';
import { NotificationBadge } from './ui/NotificationBadge';
import { languages } from '../i18n/index';
import { getInMemoryToken } from '../api/client';
import { preloadRoute } from '../lib/routePreload';

type NavItem = {
  to: string;
  icon: React.ElementType;
  key: string;
  end?: boolean;
};

type NavSection = {
  id: 'core' | 'fleet' | 'settings';
  labelKey: string;
  defaultOpen: boolean;
  collapsible: boolean;
  items: readonly NavItem[];
};

// 3-section layout. Core is always visible. Fleet defaults open. Settings defaults closed.
// Routes, icons, and per-item i18n keys are preserved from the previous flat list.
const NAV_SECTIONS: readonly NavSection[] = [
  {
    id: 'core',
    labelKey: 'nav.sections.core',
    defaultOpen: true,
    collapsible: false,
    items: [
      { to: '/', icon: House, key: 'dashboard', end: true },
      { to: '/bookings', icon: CalendarCheck, key: 'bookings' },
      { to: '/book', icon: CalendarPlus, key: 'bookSpot' },
      { to: '/vehicles', icon: Car, key: 'vehicles' },
      { to: '/calendar', icon: Calendar, key: 'calendar' },
      { to: '/credits', icon: Coins, key: 'credits' },
    ],
  },
  {
    id: 'fleet',
    labelKey: 'nav.sections.fleet',
    defaultOpen: true,
    collapsible: true,
    items: [
      { to: '/favorites', icon: Star, key: 'favorites' },
      { to: '/absences', icon: CalendarX, key: 'absences' },
      { to: '/team', icon: Users, key: 'team' },
      { to: '/leaderboard', icon: Trophy, key: 'leaderboard' },
      { to: '/map', icon: MapPin, key: 'map' },
      { to: '/history', icon: ClockCounterClockwise, key: 'history' },
      { to: '/swap-requests', icon: Swap, key: 'swapRequests' },
      { to: '/guest-pass', icon: UserPlus, key: 'guestPass' },
      { to: '/checkin', icon: QrCode, key: 'checkin' },
      { to: '/predict', icon: Sparkle, key: 'predictions' },
    ],
  },
  {
    id: 'settings',
    labelKey: 'nav.sections.settings',
    defaultOpen: false,
    collapsible: true,
    items: [
      { to: '/notifications', icon: Bell, key: 'notifications' },
      { to: '/translations', icon: Translate, key: 'translations' },
      { to: '/profile', icon: UserCircle, key: 'profile' },
    ],
  },
] as const;

const SECTION_STORAGE_PREFIX = 'parkhub_sidebar_';
const SECTION_STORAGE_SUFFIX = '_open';
const sectionStorageKey = (id: NavSection['id']) =>
  `${SECTION_STORAGE_PREFIX}${id}${SECTION_STORAGE_SUFFIX}`;

// Section spring matches the rest of the UI (stiffness 300, damping 30).
const SECTION_SPRING = { type: 'spring', stiffness: 300, damping: 30 } as const;

function readInitialSectionState(): Record<NavSection['id'], boolean> {
  const initial: Record<NavSection['id'], boolean> = {
    core: true,
    fleet: true,
    settings: false,
  };
  if (typeof window === 'undefined') return initial;
  for (const section of NAV_SECTIONS) {
    initial[section.id] = section.defaultOpen;
    if (!section.collapsible) continue;
    try {
      const stored = window.localStorage.getItem(sectionStorageKey(section.id));
      if (stored === 'true') initial[section.id] = true;
      else if (stored === 'false') initial[section.id] = false;
    } catch {
      /* localStorage unavailable — ignore */
    }
  }
  return initial;
}

interface SidebarNavProps {
  variant: 'desktop' | 'mobile';
  openSections: Record<NavSection['id'], boolean>;
  onToggleSection: (id: NavSection['id']) => void;
  onItemClick?: () => void;
  unreadCount: number;
  location: { pathname: string };
  t: (key: string) => string;
  isAdmin: boolean;
}

function SidebarNav({
  variant,
  openSections,
  onToggleSection,
  onItemClick,
  unreadCount,
  location,
  t,
  isAdmin,
}: SidebarNavProps) {
  const isDesktop = variant === 'desktop';
  const itemPadding = isDesktop ? 'px-3 py-2' : 'px-3 py-2.5';

  const renderItem = (item: NavItem) => (
    <NavLink
      key={item.key}
      to={item.to}
      end={item.end}
      onClick={onItemClick}
      onMouseEnter={() => preloadRoute(item.to)}
      onFocus={() => preloadRoute(item.to)}
      aria-current={
        location.pathname === item.to ||
        (item.to !== '/' && location.pathname.startsWith(item.to))
          ? 'page'
          : undefined
      }
      className={({ isActive }) =>
        `relative flex items-center gap-3 ${itemPadding} text-sm font-medium rounded-lg transition-all ${
          isActive
            ? 'text-primary-700 dark:text-primary-300 bg-primary-50/80 dark:bg-primary-950/30'
            : 'text-surface-600 dark:text-surface-400 hover:text-surface-900 dark:hover:text-white hover:bg-surface-100/60 dark:hover:bg-surface-800/40'
        }`
      }
    >
      {({ isActive }) => (
        <>
          {isActive && isDesktop && (
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
  );

  return (
    <nav className="flex-1 space-y-3" aria-label={isDesktop ? 'Main navigation' : undefined}>
      {NAV_SECTIONS.map((section, sectionIndex) => {
        const open = openSections[section.id];
        const collapsible = section.collapsible;
        const sectionLabel = t(section.labelKey);
        const regionId = `sidebar-section-${variant}-${section.id}`;

        const header = collapsible ? (
          <button
            type="button"
            onClick={() => onToggleSection(section.id)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                onToggleSection(section.id);
              }
            }}
            aria-expanded={open}
            aria-controls={regionId}
            className="flex items-center gap-2 w-full px-3 py-1.5 text-[11px] font-semibold uppercase tracking-wider text-surface-500 dark:text-surface-500 hover:text-surface-700 dark:hover:text-surface-300 rounded-md transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-primary-500/40"
          >
            <motion.div
              animate={{ rotate: open ? 90 : 0 }}
              transition={SECTION_SPRING}
              className="inline-flex"
            >
              <CaretDown weight="bold" className="w-3 h-3 -rotate-90" />
            </motion.div>
            <span className="flex-1 text-left">{sectionLabel}</span>
          </button>
        ) : (
          <div
            className="px-3 py-1.5 text-[11px] font-semibold uppercase tracking-wider text-surface-500 dark:text-surface-500"
            aria-hidden={sectionIndex === 0 && !isDesktop ? undefined : undefined}
          >
            {sectionLabel}
          </div>
        );

        return (
          <div key={section.id} className="space-y-0.5">
            {header}
            <AnimatePresence initial={false}>
              {open && (
                <motion.div
                  key="section-body"
                  id={regionId}
                  initial={collapsible ? { height: 0, opacity: 0 } : false}
                  animate={{ height: 'auto', opacity: 1 }}
                  exit={{ height: 0, opacity: 0 }}
                  transition={SECTION_SPRING}
                  className="overflow-hidden"
                  role="group"
                  aria-label={sectionLabel}
                >
                  <div className="space-y-0.5 pt-0.5">
                    {section.items.map(renderItem)}
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        );
      })}

      {isAdmin && (
        <div className="space-y-0.5 pt-1">
          <NavLink
            to="/admin"
            onClick={onItemClick}
            className={({ isActive }) =>
              `relative flex items-center gap-3 ${itemPadding} text-sm font-medium rounded-lg transition-all ${
                isActive
                  ? 'text-primary-700 dark:text-primary-300 bg-primary-50/80 dark:bg-primary-950/30'
                  : 'text-surface-600 dark:text-surface-400 hover:text-surface-900 dark:hover:text-white hover:bg-surface-100/60 dark:hover:bg-surface-800/40'
              }`
            }
          >
            <GearSix weight="fill" className="w-5 h-5" />
            {t('nav.admin')}
          </NavLink>
        </div>
      )}
    </nav>
  );
}

function LanguageSelector() {
  const { i18n } = useTranslation();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function onClickOutside(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener('mousedown', onClickOutside);
    return () => document.removeEventListener('mousedown', onClickOutside);
  }, []);

  const current = languages.find(l => l.code === i18n?.language) ?? languages[0];

  return (
    <div ref={ref} className="relative">
      <button
        onClick={() => setOpen(o => !o)}
        className="flex items-center gap-2 px-3 py-2 text-sm font-medium text-surface-600 dark:text-surface-400 hover:text-surface-900 dark:hover:text-white rounded-lg hover:bg-surface-100/60 dark:hover:bg-surface-800/40 transition-all w-full"
        aria-label="Change language"
        aria-expanded={open}
      >
        <Globe weight="fill" className="w-5 h-5" />
        <span className="flex-1 text-left">{current.flag} {current.native}</span>
        <CaretDown weight="bold" className={`w-3.5 h-3.5 transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>
      {open && (
        <div className="absolute bottom-full left-0 mb-1 w-full bg-white dark:bg-surface-800 border border-surface-200 dark:border-surface-700 rounded-lg shadow-lg py-1 z-50 max-h-64 overflow-y-auto">
          {languages.map(lang => (
            <button
              key={lang.code}
              onClick={() => { i18n?.changeLanguage(lang.code); setOpen(false); }}
              className={`flex items-center gap-2 w-full px-3 py-2 text-sm transition-colors ${
                lang.code === i18n?.language
                  ? 'bg-primary-50 dark:bg-primary-950/30 text-primary-700 dark:text-primary-300 font-medium'
                  : 'text-surface-700 dark:text-surface-300 hover:bg-surface-50 dark:hover:bg-surface-700/50'
              }`}
            >
              <span>{lang.flag}</span>
              <span>{lang.native}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

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
  const [openSections, setOpenSections] = useState<Record<NavSection['id'], boolean>>(
    () => readInitialSectionState(),
  );

  const toggleSection = useCallback((id: NavSection['id']) => {
    setOpenSections(prev => {
      const next = { ...prev, [id]: !prev[id] };
      try {
        window.localStorage.setItem(sectionStorageKey(id), String(next[id]));
      } catch {
        /* ignore */
      }
      return next;
    });
  }, []);

  const toggleCommandPalette = useCallback(
    () => setCommandPaletteOpen(prev => !prev),
    [],
  );

  useKeyboardShortcuts({ onToggleCommandPalette: toggleCommandPalette });
  usePageTitle();

  // Global "open command palette" event so pages (e.g. empty states) can
  // request the palette without drilling props or lifting state.
  useEffect(() => {
    function openFromEvent() {
      setCommandPaletteOpen(true);
    }
    window.addEventListener('parkhub:open-command-palette', openFromEvent);
    return () => window.removeEventListener('parkhub:open-command-palette', openFromEvent);
  }, []);

  useEffect(() => {
    const token = getInMemoryToken();
    if (!token) return;
    fetch('/api/v1/notifications/unread-count', {
      headers: {
        'Authorization': `Bearer ${token}`,
        'X-Requested-With': 'XMLHttpRequest',
      },
      credentials: 'include',
    })
      .then(r => { if (!r.ok) return null; return r.json(); })
      .then(res => { if (res?.data?.count !== undefined) setUnreadCount(res.data.count); })
      .catch(() => {});
  }, [location.pathname]);

  function handleLogout() {
    logout();
    navigate('/welcome');
  }

  const isAdmin = useMemo(
    () => !!(user?.role && ['admin', 'superadmin'].includes(user.role)),
    [user?.role],
  );

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

        <div className="flex-1 overflow-y-auto -mx-1 px-1">
          <SidebarNav
            variant="desktop"
            openSections={openSections}
            onToggleSection={toggleSection}
            unreadCount={unreadCount}
            location={location}
            t={t}
            isAdmin={isAdmin}
          />
        </div>

        <LanguageSelector />

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
            <div className="flex items-center gap-1">
              <NotificationCenter />
              <button
                onClick={() => setTheme(resolved === 'dark' ? 'light' : 'dark')}
                className="btn btn-ghost btn-icon"
                aria-label={resolved === 'dark' ? t('nav.switchToLight') : t('nav.switchToDark')}
              >
                {resolved === 'dark' ? <SunDim weight="fill" className="w-5 h-5" /> : <Moon weight="fill" className="w-5 h-5" />}
              </button>
            </div>
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
                className="fixed inset-y-0 left-0 w-72 bg-white/90 dark:bg-surface-900/90 backdrop-blur-2xl z-50 p-4 lg:hidden touch-pan-y flex flex-col"
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
                <div className="flex-1 overflow-y-auto -mx-1 px-1 pb-16">
                  <SidebarNav
                    variant="mobile"
                    openSections={openSections}
                    onToggleSection={toggleSection}
                    onItemClick={() => setSidebarOpen(false)}
                    unreadCount={unreadCount}
                    location={location}
                    t={t}
                    isAdmin={isAdmin}
                  />
                </div>
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
          <div className="hidden lg:flex justify-between items-center mb-2">
            <Breadcrumb />
            <NotificationCenter />
          </div>
          <div className="lg:hidden"><Breadcrumb /></div>
          <Outlet />
        </main>
        <footer className="py-3 text-center text-xs text-surface-400 dark:text-surface-600 border-t border-surface-200/40 dark:border-surface-800/40">
          ParkHub v4.9.0
        </footer>
      </div>

      <CommandPalette open={commandPaletteOpen} onClose={() => setCommandPaletteOpen(false)} />
      <ThemeSwitcherFab onClick={() => setThemeSwitcherOpen(true)} />
      <ThemeSwitcher open={themeSwitcherOpen} onClose={() => setThemeSwitcherOpen(false)} />
    </div>
  );
}
