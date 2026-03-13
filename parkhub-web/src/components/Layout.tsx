import { useState } from 'react';
import { Outlet, NavLink, useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  House, CalendarCheck, Car, Coins,
  GearSix, SignOut, List, X, SunDim, Moon,
  Buildings, HouseSimple, UsersThree,
} from '@phosphor-icons/react';
import { useAuth } from '../context/AuthContext';
import { useTheme } from '../context/ThemeContext';
import { useUseCase, type UseCase } from '../context/UseCaseContext';
import { useFeatures, type FeatureModule } from '../context/FeaturesContext';

const NAV_ITEMS: { to: string; icon: React.ElementType; key: string; end?: boolean; feature?: FeatureModule }[] = [
  { to: '/', icon: House, key: 'dashboard', end: true },
  { to: '/bookings', icon: CalendarCheck, key: 'bookings' },
  { to: '/credits', icon: Coins, key: 'credits', feature: 'credits' },
];

export function Layout() {
  const { t } = useTranslation();
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const { resolved, setTheme } = useTheme();
  const { useCase } = useUseCase();
  const { isEnabled } = useFeatures();
  const [sidebarOpen, setSidebarOpen] = useState(false);

  const visibleNav = NAV_ITEMS.filter(item => !item.feature || isEnabled(item.feature));

  const useCaseIcons: Record<UseCase, React.ElementType> = {
    business: Buildings,
    residential: HouseSimple,
    personal: UsersThree,
  };
  const UseCaseIcon = useCaseIcons[useCase];

  function handleLogout() {
    logout();
    navigate('/welcome');
  }

  const activeClass = 'bg-accent-100/60 dark:bg-accent-900/15 text-accent-800 dark:text-accent-400 font-semibold border-l-2 border-accent-500';
  const inactiveClass = 'text-surface-600 dark:text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-800/60 border-l-2 border-transparent';

  return (
    <div className="min-h-dvh bg-surface-50 dark:bg-surface-950 flex">
      {/* Sidebar — desktop */}
      <aside className="hidden lg:flex flex-col w-60 bg-white dark:bg-surface-900 border-r border-surface-200 dark:border-surface-800 sticky top-0 h-dvh">
        {/* Logo */}
        <div className="flex items-center gap-3 px-5 pt-6 pb-5">
          <div className="w-9 h-9 bg-primary-900 dark:bg-surface-800 flex items-center justify-center border border-primary-800 dark:border-surface-700">
            <Car weight="fill" className="w-4 h-4 text-accent-500" />
          </div>
          <div className="min-w-0">
            <span className="text-lg font-bold text-surface-900 dark:text-white tracking-tight font-[Outfit]">ParkHub</span>
            <div className="flex items-center gap-1.5">
              <UseCaseIcon weight="regular" className="w-3 h-3 text-accent-500" />
              <span className="text-[10px] font-semibold text-surface-400 uppercase tracking-widest">{t(`useCase.${useCase}.name`)}</span>
            </div>
          </div>
        </div>

        {/* Divider */}
        <div className="divider-industrial mx-5 mb-4" />

        {/* Nav */}
        <nav className="flex-1 px-3 space-y-0.5">
          {visibleNav.map(item => (
            <NavLink
              key={item.key}
              to={item.to}
              end={item.end}
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2 rounded-md text-[0.8125rem] font-medium transition-all cursor-pointer ${isActive ? activeClass : inactiveClass}`
              }
            >
              <item.icon weight="bold" className="w-[18px] h-[18px]" />
              {t(`nav.${item.key}`)}
            </NavLink>
          ))}

          {user?.role && ['admin', 'superadmin'].includes(user.role) && (
            <NavLink
              to="/admin/features"
              className={({ isActive }) =>
                `flex items-center gap-3 px-3 py-2 rounded-md text-[0.8125rem] font-medium transition-all cursor-pointer ${isActive ? activeClass : inactiveClass}`
              }
            >
              <GearSix weight="bold" className="w-[18px] h-[18px]" />
              {t('nav.admin')}
            </NavLink>
          )}
        </nav>

        {/* Bottom section */}
        <div className="px-3 pb-4">
          {/* Theme toggle */}
          <button
            onClick={() => setTheme(resolved === 'dark' ? 'light' : 'dark')}
            className="flex items-center gap-3 px-3 py-2 rounded-md text-[0.8125rem] font-medium text-surface-500 dark:text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-800/60 transition-all mb-3 w-full cursor-pointer"
          >
            {resolved === 'dark' ? <SunDim weight="bold" className="w-[18px] h-[18px]" /> : <Moon weight="bold" className="w-[18px] h-[18px]" />}
            {resolved === 'dark' ? t('common.lightMode') || 'Light' : t('common.darkMode') || 'Dark'}
          </button>

          <div className="divider-industrial mb-3" />

          {/* User info */}
          <div className="flex items-center gap-3 px-3 mb-2">
            <div className="w-8 h-8 bg-accent-100 dark:bg-accent-900/20 flex items-center justify-center">
              <span className="text-xs font-bold text-accent-700 dark:text-accent-400 font-[Outfit]">
                {(user?.name || user?.username || 'U').charAt(0).toUpperCase()}
              </span>
            </div>
            <div className="min-w-0">
              <p className="text-sm font-medium text-surface-900 dark:text-white truncate">{user?.name || user?.username}</p>
              <p className="text-[11px] text-surface-400 truncate">{user?.email}</p>
            </div>
          </div>
          <button
            onClick={handleLogout}
            className="flex items-center gap-3 px-3 py-2 rounded-md text-[0.8125rem] font-medium text-danger hover:bg-red-50 dark:hover:bg-red-900/15 transition-all w-full cursor-pointer"
          >
            <SignOut weight="bold" className="w-[18px] h-[18px]" />
            {t('nav.logout')}
          </button>
        </div>
      </aside>

      {/* Mobile top bar */}
      <div className="flex-1 flex flex-col">
        <header className="lg:hidden sticky top-0 z-30 bg-white/95 dark:bg-surface-900/95 backdrop-blur-md border-b border-surface-200 dark:border-surface-800 px-4 py-2.5">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <button onClick={() => setSidebarOpen(true)} className="btn btn-ghost btn-icon cursor-pointer" aria-label="Open menu">
                <List weight="bold" className="w-5 h-5" />
              </button>
              <div className="flex items-center gap-2">
                <div className="w-7 h-7 bg-primary-900 dark:bg-surface-800 flex items-center justify-center border border-primary-800 dark:border-surface-700">
                  <Car weight="fill" className="w-3.5 h-3.5 text-accent-500" />
                </div>
                <span className="font-bold text-surface-900 dark:text-white font-[Outfit] text-sm tracking-tight">ParkHub</span>
              </div>
            </div>
            <button
              onClick={() => setTheme(resolved === 'dark' ? 'light' : 'dark')}
              className="btn btn-ghost btn-icon cursor-pointer"
              aria-label="Toggle theme"
            >
              {resolved === 'dark' ? <SunDim weight="bold" className="w-5 h-5" /> : <Moon weight="bold" className="w-5 h-5" />}
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
                className="fixed inset-0 bg-black/60 z-40 lg:hidden"
                onClick={() => setSidebarOpen(false)}
              />
              <motion.aside
                initial={{ x: '-100%' }}
                animate={{ x: 0 }}
                exit={{ x: '-100%' }}
                transition={{ type: 'spring', damping: 28, stiffness: 350 }}
                className="fixed inset-y-0 left-0 w-64 bg-white dark:bg-surface-900 z-50 lg:hidden shadow-xl border-r border-surface-200 dark:border-surface-800"
              >
                <div className="flex items-center justify-between px-5 pt-5 pb-4">
                  <div className="flex items-center gap-2">
                    <div className="w-9 h-9 bg-primary-900 dark:bg-surface-800 flex items-center justify-center border border-primary-800 dark:border-surface-700">
                      <Car weight="fill" className="w-4 h-4 text-accent-500" />
                    </div>
                    <span className="text-lg font-bold text-surface-900 dark:text-white font-[Outfit] tracking-tight">ParkHub</span>
                  </div>
                  <button onClick={() => setSidebarOpen(false)} className="btn btn-ghost btn-icon cursor-pointer" aria-label="Close menu">
                    <X weight="bold" className="w-5 h-5" />
                  </button>
                </div>
                <div className="divider-industrial mx-5 mb-4" />
                <nav className="px-3 space-y-0.5">
                  {visibleNav.map(item => (
                    <NavLink
                      key={item.key}
                      to={item.to}
                      end={item.end}
                      onClick={() => setSidebarOpen(false)}
                      className={({ isActive }) =>
                        `flex items-center gap-3 px-3 py-2.5 rounded-md text-sm font-medium transition-all cursor-pointer ${isActive ? activeClass : inactiveClass}`
                      }
                    >
                      <item.icon weight="bold" className="w-5 h-5" />
                      {t(`nav.${item.key}`)}
                    </NavLink>
                  ))}

                  {user?.role && ['admin', 'superadmin'].includes(user.role) && (
                    <NavLink
                      to="/admin/features"
                      onClick={() => setSidebarOpen(false)}
                      className={({ isActive }) =>
                        `flex items-center gap-3 px-3 py-2.5 rounded-md text-sm font-medium transition-all cursor-pointer ${isActive ? activeClass : inactiveClass}`
                      }
                    >
                      <GearSix weight="bold" className="w-5 h-5" />
                      {t('nav.admin')}
                    </NavLink>
                  )}
                </nav>
                <div className="absolute bottom-4 left-3 right-3 safe-bottom">
                  <button onClick={handleLogout} className="flex items-center gap-3 px-3 py-2.5 rounded-md text-sm font-medium text-danger w-full cursor-pointer">
                    <SignOut weight="bold" className="w-5 h-5" /> {t('nav.logout')}
                  </button>
                </div>
              </motion.aside>
            </>
          )}
        </AnimatePresence>

        {/* Main content */}
        <main className="flex-1 p-4 sm:p-6 lg:p-8 pb-[calc(1rem+env(safe-area-inset-bottom,0px))] max-w-6xl mx-auto w-full">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
