import { ReactNode, useState, useEffect, useRef } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import {
  House,
  CalendarPlus,
  ListChecks,
  Car,
  GearSix,
  SignOut,
  Moon,
  Sun,
  List,
  X,
  Bell,
  User,
  CaretDown,
} from '@phosphor-icons/react';
import { useAuth } from '../context/AuthContext';
import { useTheme, applyTheme } from '../stores/theme';

interface LayoutProps {
  children: ReactNode;
}

const navigation = [
  { name: 'Dashboard', href: '/', icon: House },
  { name: 'Buchen', href: '/book', icon: CalendarPlus },
  { name: 'Buchungen', href: '/bookings', icon: ListChecks },
  { name: 'Fahrzeuge', href: '/vehicles', icon: Car },
];

const adminNav = [
  { name: 'Admin', href: '/admin', icon: GearSix },
];

export function Layout({ children }: LayoutProps) {
  const { user, logout } = useAuth();
  const location = useLocation();
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [userMenuOpen, setUserMenuOpen] = useState(false);
  const { isDark, toggle } = useTheme();
  const userMenuRef = useRef<HTMLDivElement>(null);

  const isAdmin = user?.role === 'admin' || user?.role === 'superadmin';

  useEffect(() => {
    applyTheme(isDark);
  }, [isDark]);

  // Close menus on route change
  useEffect(() => {
    setMobileMenuOpen(false);
    setUserMenuOpen(false);
  }, [location.pathname]);

  // Close user menu on outside click
  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (userMenuRef.current && !userMenuRef.current.contains(e.target as Node)) {
        setUserMenuOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClick);
    return () => document.removeEventListener('mousedown', handleClick);
  }, []);

  return (
    <div className="min-h-screen flex flex-col">
      {/* Header */}
      <header className="sticky top-0 z-50 bg-white/80 dark:bg-gray-900/80 backdrop-blur-lg border-b border-gray-100 dark:border-gray-800">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            {/* Logo */}
            <Link to="/" className="flex items-center gap-3" aria-label="ParkHub – Startseite">
              <div className="w-9 h-9 bg-primary-600 rounded-xl flex items-center justify-center">
                <Car weight="fill" className="w-5 h-5 text-white" aria-hidden="true" />
              </div>
              <span className="text-lg font-bold text-gray-900 dark:text-white">
                ParkHub
              </span>
            </Link>

            {/* Desktop Navigation */}
            <nav role="navigation" aria-label="Hauptnavigation" className="hidden md:flex items-center gap-1">
              {navigation.map((item) => {
                const Icon = item.icon;
                const isActive = location.pathname === item.href;
                return (
                  <Link
                    key={item.href}
                    to={item.href}
                    aria-current={isActive ? 'page' : undefined}
                    className={`flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-medium transition-colors ${
                      isActive
                        ? 'bg-primary-50 text-primary-700 dark:bg-primary-900/30 dark:text-primary-400'
                        : 'text-gray-600 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-800'
                    }`}
                  >
                    <Icon weight={isActive ? 'fill' : 'regular'} className="w-5 h-5" aria-hidden="true" />
                    {item.name}
                  </Link>
                );
              })}
              {isAdmin && adminNav.map((item) => {
                const Icon = item.icon;
                const isActive = location.pathname.startsWith(item.href);
                return (
                  <Link
                    key={item.href}
                    to={item.href}
                    aria-current={isActive ? 'page' : undefined}
                    className={`flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-medium transition-colors ${
                      isActive
                        ? 'bg-primary-50 text-primary-700 dark:bg-primary-900/30 dark:text-primary-400'
                        : 'text-gray-600 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-800'
                    }`}
                  >
                    <Icon weight={isActive ? 'fill' : 'regular'} className="w-5 h-5" aria-hidden="true" />
                    {item.name}
                  </Link>
                );
              })}
            </nav>

            {/* Right Side */}
            <div className="flex items-center gap-2">
              {/* Theme Toggle */}
              <button
                onClick={toggle}
                className="btn btn-ghost btn-icon"
                aria-label={isDark ? 'Helles Design aktivieren' : 'Dunkles Design aktivieren'}
              >
                {isDark ? (
                  <Sun weight="fill" className="w-5 h-5" aria-hidden="true" />
                ) : (
                  <Moon weight="fill" className="w-5 h-5" aria-hidden="true" />
                )}
              </button>

              {/* Notifications */}
              <button className="btn btn-ghost btn-icon relative" aria-label="Benachrichtigungen">
                <Bell weight="regular" className="w-5 h-5" aria-hidden="true" />
                <span className="absolute top-1.5 right-1.5 w-2 h-2 bg-red-500 rounded-full" aria-hidden="true" />
              </button>

              {/* User Menu */}
              <div className="relative hidden md:block" ref={userMenuRef}>
                <button
                  onClick={() => setUserMenuOpen(!userMenuOpen)}
                  aria-expanded={userMenuOpen}
                  aria-haspopup="menu"
                  aria-label={`Benutzermenü für ${user?.name}`}
                  className="flex items-center gap-2 p-1.5 pr-3 rounded-xl hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
                >
                  <div className="avatar text-sm" aria-hidden="true">
                    {user?.name?.charAt(0).toUpperCase()}
                  </div>
                  <span className="text-sm font-medium text-gray-700 dark:text-gray-300">
                    {user?.name?.split(' ')[0]}
                  </span>
                  <CaretDown weight="bold" className="w-4 h-4 text-gray-400" aria-hidden="true" />
                </button>

                <AnimatePresence>
                  {userMenuOpen && (
                    <motion.div
                      initial={{ opacity: 0, y: 10 }}
                      animate={{ opacity: 1, y: 0 }}
                      exit={{ opacity: 0, y: 10 }}
                      role="menu"
                      aria-label="Benutzeroptionen"
                      className="absolute right-0 mt-2 w-56 card p-2 shadow-lg"
                    >
                      <div className="px-3 py-2 border-b border-gray-100 dark:border-gray-800 mb-2">
                        <p className="font-medium text-gray-900 dark:text-white">{user?.name}</p>
                        <p className="text-sm text-gray-500">{user?.email}</p>
                      </div>
                      <Link
                        to="/profile"
                        role="menuitem"
                        className="flex items-center gap-2 px-3 py-2 rounded-lg text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800"
                      >
                        <User weight="regular" className="w-4 h-4" aria-hidden="true" />
                        Profil
                      </Link>
                      <button
                        onClick={logout}
                        role="menuitem"
                        className="flex items-center gap-2 w-full px-3 py-2 rounded-lg text-sm text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
                      >
                        <SignOut weight="regular" className="w-4 h-4" aria-hidden="true" />
                        Abmelden
                      </button>
                    </motion.div>
                  )}
                </AnimatePresence>
              </div>

              {/* Mobile Menu Button */}
              <button
                onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
                aria-expanded={mobileMenuOpen}
                aria-controls="mobile-nav"
                aria-label={mobileMenuOpen ? 'Menü schließen' : 'Menü öffnen'}
                className="md:hidden btn btn-ghost btn-icon"
              >
                {mobileMenuOpen ? (
                  <X weight="bold" className="w-5 h-5" aria-hidden="true" />
                ) : (
                  <List weight="bold" className="w-5 h-5" aria-hidden="true" />
                )}
              </button>
            </div>
          </div>
        </div>

        {/* Mobile Navigation */}
        <AnimatePresence>
          {mobileMenuOpen && (
            <motion.div
              id="mobile-nav"
              initial={{ height: 0, opacity: 0 }}
              animate={{ height: 'auto', opacity: 1 }}
              exit={{ height: 0, opacity: 0 }}
              className="md:hidden overflow-hidden border-t border-gray-100 dark:border-gray-800"
            >
              <nav role="navigation" aria-label="Mobile Navigation" className="px-4 py-3 space-y-1">
                {navigation.map((item) => {
                  const Icon = item.icon;
                  const isActive = location.pathname === item.href;
                  return (
                    <Link
                      key={item.href}
                      to={item.href}
                      aria-current={isActive ? 'page' : undefined}
                      className={`flex items-center gap-3 px-4 py-3 rounded-xl text-base font-medium ${
                        isActive
                          ? 'bg-primary-50 text-primary-700 dark:bg-primary-900/30 dark:text-primary-400'
                          : 'text-gray-600 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-800'
                      }`}
                    >
                      <Icon weight={isActive ? 'fill' : 'regular'} className="w-5 h-5" aria-hidden="true" />
                      {item.name}
                    </Link>
                  );
                })}
                {isAdmin && adminNav.map((item) => {
                  const Icon = item.icon;
                  return (
                    <Link
                      key={item.href}
                      to={item.href}
                      className="flex items-center gap-3 px-4 py-3 rounded-xl text-base font-medium text-gray-600 hover:bg-gray-100 dark:text-gray-400 dark:hover:bg-gray-800"
                    >
                      <Icon weight="regular" className="w-5 h-5" aria-hidden="true" />
                      {item.name}
                    </Link>
                  );
                })}
                <div className="pt-3 border-t border-gray-100 dark:border-gray-800">
                  <button
                    onClick={logout}
                    className="flex items-center gap-3 w-full px-4 py-3 rounded-xl text-base font-medium text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
                  >
                    <SignOut weight="regular" className="w-5 h-5" aria-hidden="true" />
                    Abmelden
                  </button>
                </div>
              </nav>
            </motion.div>
          )}
        </AnimatePresence>
      </header>

      {/* Main Content */}
      <main className="flex-1" id="main-content">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
          <motion.div
            key={location.pathname}
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.2 }}
            style={{ willChange: 'opacity, transform' }}
            className="motion-safe:transition-all"
          >
            {children}
          </motion.div>
        </div>
      </main>

      {/* Footer */}
      <footer className="bg-white dark:bg-gray-900 border-t border-gray-100 dark:border-gray-800">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6 flex flex-col sm:flex-row items-center justify-between gap-3">
          <p className="text-sm text-gray-500 dark:text-gray-400">
            ParkHub — Open Source Parking Management
          </p>
          <nav aria-label="Rechtliche Links" className="flex items-center gap-4 flex-wrap justify-center">
            <a
              href="/impressum"
              className="text-sm text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
            >
              Impressum
            </a>
            <a
              href="/datenschutz"
              className="text-sm text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
            >
              Datenschutz
            </a>
            <a
              href="/agb"
              className="text-sm text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
            >
              AGB
            </a>
          </nav>
        </div>
      </footer>
    </div>
  );
}
