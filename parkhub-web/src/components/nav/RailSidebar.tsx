/**
 * Rail layout — 72px icon-only sidebar. Ported from the claude.ai/design
 * v4 nav-variants bundle. Shares NAV_SECTIONS with the classic layout so
 * every new route added in Layout.tsx appears here automatically.
 *
 * UX notes:
 *  - Hovering an icon reveals a side-popping label — no drawer, stays
 *    lightweight. Matches Linear/GitHub Desktop rail patterns.
 *  - Section dividers replace the uppercase labels used by Classic
 *    (no room for text in a rail).
 *  - Active-state is a glowing left accent bar that animates between
 *    items via framer-motion's layoutId (matches the Classic indicator).
 */
import { useRef, useState } from 'react';
import { NavLink, useLocation } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  CarSimple, GearSix, SignOut, SunDim, Moon,
} from '@phosphor-icons/react';
import { useAuth } from '../../context/AuthContext';
import { useTheme } from '../../context/ThemeContext';
import { NotificationCenter } from '../NotificationCenter';
import { NotificationBadge } from '../ui/NotificationBadge';
import { preloadRoute } from '../../lib/routePreload';
import { NAV_SECTIONS, type NavItem } from '../Layout';
import { isActivePath } from './navActive';

interface RailSidebarProps {
  unreadCount: number;
  onLogout: () => void;
  isAdmin: boolean;
}

export function RailSidebar({ unreadCount, onLogout, isAdmin }: RailSidebarProps) {
  const { t } = useTranslation();
  const { user } = useAuth();
  const { resolved, setTheme } = useTheme();
  const location = useLocation();

  const renderItem = (item: NavItem) => {
    const isActive = isActivePath(location.pathname, item.to);
    return (
      <RailIconButton
        key={item.key}
        to={item.to}
        icon={item.icon}
        label={t(`nav.${item.key}`)}
        badge={item.key === 'notifications' ? unreadCount : 0}
        active={isActive}
        end={item.end}
      />
    );
  };

  return (
    <aside
      aria-label="Main navigation"
      className="hidden lg:flex flex-col items-center w-[72px] bg-white/70 dark:bg-surface-900/70 backdrop-blur-2xl border-r border-surface-200/40 dark:border-surface-800/40 py-4 sticky top-0 h-dvh"
    >
      {/* Brand mark */}
      <div
        className="w-10 h-10 rounded-xl bg-gradient-to-br from-primary-600 to-primary-500 flex items-center justify-center shadow-lg shadow-primary-500/20 mb-4"
        title="ParkHub"
      >
        <CarSimple weight="fill" className="w-5 h-5 text-white" />
      </div>

      {/* Nav — sections separated by thin dividers since labels won't fit */}
      <nav className="flex-1 flex flex-col items-center gap-0.5 w-full overflow-y-auto px-2">
        {NAV_SECTIONS.map((section, index) => (
          <div key={section.id} className="flex flex-col items-center gap-0.5 w-full">
            {index > 0 && (
              <div
                aria-hidden="true"
                className="w-8 h-px my-2 bg-surface-200 dark:bg-surface-800"
              />
            )}
            {section.items.map(renderItem)}
          </div>
        ))}

        {isAdmin && (
          <>
            <div
              aria-hidden="true"
              className="w-8 h-px my-2 bg-surface-200 dark:bg-surface-800"
            />
            <RailIconButton
              to="/admin"
              icon={GearSix}
              label={t('nav.admin')}
              active={isActivePath(location.pathname, '/admin')}
            />
          </>
        )}
      </nav>

      {/* Footer cluster */}
      <div className="flex flex-col items-center gap-1 w-full pt-2 border-t border-surface-200/60 dark:border-surface-800/60">
        <button
          onClick={() => setTheme(resolved === 'dark' ? 'light' : 'dark')}
          className="flex items-center justify-center w-10 h-10 rounded-lg text-surface-500 hover:text-surface-900 dark:hover:text-white hover:bg-surface-100/60 dark:hover:bg-surface-800/40 transition-all"
          aria-label={resolved === 'dark' ? t('nav.lightMode') : t('nav.darkMode')}
          title={resolved === 'dark' ? t('nav.lightMode') : t('nav.darkMode')}
        >
          {resolved === 'dark' ? <SunDim weight="fill" className="w-5 h-5" /> : <Moon weight="fill" className="w-5 h-5" />}
        </button>

        <div className="my-1 scale-90">
          <NotificationCenter />
        </div>

        <div
          className="relative"
          title={user?.name || user?.username}
        >
          <div className="w-9 h-9 rounded-full bg-gradient-to-br from-primary-200 to-primary-100 dark:from-primary-800 dark:to-primary-900 flex items-center justify-center ring-2 ring-primary-500/20 dark:ring-primary-400/20">
            <span className="text-sm font-bold text-primary-700 dark:text-primary-300">
              {(user?.name || user?.username || 'U').charAt(0).toUpperCase()}
            </span>
          </div>
          <span className="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full bg-emerald-500 border-2 border-white dark:border-surface-900" />
        </div>

        <button
          onClick={onLogout}
          className="flex items-center justify-center w-10 h-10 rounded-lg text-red-500 hover:text-red-600 dark:hover:text-red-400 hover:bg-red-50/60 dark:hover:bg-red-950/20 transition-all mt-1"
          aria-label={t('nav.logout')}
          title={t('nav.logout')}
        >
          <SignOut weight="bold" className="w-5 h-5" />
        </button>
      </div>
    </aside>
  );
}

interface RailIconButtonProps {
  to: string;
  icon: React.ElementType;
  label: string;
  badge?: number;
  active?: boolean;
  end?: boolean;
}

/**
 * Single rail cell — NavLink + side-popping label tooltip. Uses a
 * ref-positioned absolute label that animates in on pointer-enter /
 * focus so keyboard users get the same discovery affordance as mouse
 * users.
 */
function RailIconButton({ to, icon: Icon, label, badge = 0, active = false, end }: RailIconButtonProps) {
  const ref = useRef<HTMLAnchorElement>(null);
  const [hover, setHover] = useState(false);

  return (
    <NavLink
      ref={ref}
      to={to}
      end={end}
      onMouseEnter={() => {
        setHover(true);
        preloadRoute(to);
      }}
      onMouseLeave={() => setHover(false)}
      onFocus={() => {
        setHover(true);
        preloadRoute(to);
      }}
      onBlur={() => setHover(false)}
      aria-label={label}
      aria-current={active ? 'page' : undefined}
      className={`relative flex items-center justify-center w-10 h-10 rounded-xl transition-colors ${
        active
          ? 'text-primary-700 dark:text-primary-300 bg-primary-50/80 dark:bg-primary-950/30'
          : 'text-surface-500 dark:text-surface-400 hover:text-surface-900 dark:hover:text-white hover:bg-surface-100/60 dark:hover:bg-surface-800/40'
      }`}
    >
      {active && (
        <motion.span
          layoutId="rail-indicator"
          aria-hidden="true"
          className="absolute left-0 top-1/2 -translate-y-1/2 w-[3px] h-[60%] -ml-[13px] rounded-full bg-gradient-to-b from-primary-500 to-primary-400"
          transition={{ type: 'spring', stiffness: 380, damping: 30 }}
        />
      )}
      <span className="relative">
        <Icon weight="fill" className="w-5 h-5" />
        {badge > 0 && <NotificationBadge count={badge} />}
      </span>

      <AnimatePresence>
        {hover && (
          <motion.span
            initial={{ opacity: 0, x: -6 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -6 }}
            transition={{ duration: 0.12 }}
            role="tooltip"
            className="pointer-events-none absolute left-full ml-3 px-2.5 py-1.5 rounded-lg bg-surface-900 dark:bg-surface-100 text-white dark:text-surface-900 text-[12px] font-semibold whitespace-nowrap shadow-xl z-50"
          >
            {label}
          </motion.span>
        )}
      </AnimatePresence>
    </NavLink>
  );
}
