/**
 * Floating dock — macOS-style horizontal pill pinned to bottom-center.
 * Ported from claude.ai/design v4 nav-variants bundle.
 *
 * UX notes:
 *  - Curated to the 8 most-used nav items (core + 2 favourites); everything
 *    else lives behind a "More" overflow that pops open a compact grid.
 *  - Pointer-proximity magnification matches the macOS dock feel: icons
 *    under the cursor scale up subtly, neighbors a bit, rest stay flat.
 *  - On mobile the dock replaces the hamburger menu entirely — feels more
 *    native than a drawer.
 */
import { useRef, useState } from 'react';
import { NavLink, useLocation } from 'react-router-dom';
import { motion, useMotionValue, useSpring, useTransform, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { DotsThree, X } from '@phosphor-icons/react';
import { preloadRoute } from '../../lib/routePreload';
import { NotificationBadge } from '../ui/NotificationBadge';
import { NAV_SECTIONS, type NavItem } from '../Layout';
import { isActivePath } from './navActive';

interface FloatingDockProps {
  unreadCount: number;
  isAdmin: boolean;
}

// 8 most-used items for the main dock. Everything else goes under "More".
// Keys match the NavItem key field so i18n lookups stay consistent.
const PINNED_KEYS = [
  'dashboard', 'bookings', 'bookSpot', 'vehicles', 'calendar',
  'favorites', 'map', 'notifications',
];

export function FloatingDock({ unreadCount, isAdmin }: FloatingDockProps) {
  const { t } = useTranslation();
  const location = useLocation();
  const mouseX = useMotionValue<number | null>(null);
  const [overflowOpen, setOverflowOpen] = useState(false);
  const dockRef = useRef<HTMLDivElement>(null);

  // Flatten NAV_SECTIONS into a single item list, partition into pinned
  // vs overflow. Admin route goes into overflow as "settings".
  const allItems: NavItem[] = NAV_SECTIONS.flatMap(s => s.items);
  const pinned = PINNED_KEYS
    .map(k => allItems.find(i => i.key === k))
    .filter((i): i is NavItem => !!i);
  const pinnedSet = new Set(pinned.map(i => i.key));
  const overflow = allItems.filter(i => !pinnedSet.has(i.key));

  return (
    <>
      <motion.nav
        ref={dockRef}
        aria-label="Main navigation"
        onMouseMove={(e) => {
          const rect = dockRef.current?.getBoundingClientRect();
          if (rect) mouseX.set(e.clientX - rect.left);
        }}
        onMouseLeave={() => mouseX.set(null)}
        className="fixed bottom-4 left-1/2 -translate-x-1/2 z-40 flex items-end gap-1 px-3 py-2 rounded-2xl bg-white/80 dark:bg-surface-900/80 backdrop-blur-2xl border border-surface-200/60 dark:border-surface-800/60 shadow-2xl shadow-black/10"
      >
        {pinned.map(item => (
          <DockIcon
            key={item.key}
            item={item}
            active={isActivePath(location.pathname, item.to)}
            badge={item.key === 'notifications' ? unreadCount : 0}
            mouseX={mouseX}
            label={t(`nav.${item.key}`)}
          />
        ))}

        <div aria-hidden="true" className="w-px h-8 mx-1 bg-surface-200 dark:bg-surface-700" />

        <button
          type="button"
          onClick={() => setOverflowOpen(v => !v)}
          aria-label={t('nav.more', 'More')}
          aria-expanded={overflowOpen}
          className={`flex flex-col items-center justify-center w-12 h-12 rounded-xl transition-all ${
            overflowOpen
              ? 'bg-primary-100 dark:bg-primary-950/50 text-primary-700 dark:text-primary-300'
              : 'text-surface-600 dark:text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-800'
          }`}
        >
          <DotsThree weight="bold" className="w-6 h-6" />
        </button>
      </motion.nav>

      <AnimatePresence>
        {overflowOpen && (
          <motion.div
            initial={{ opacity: 0, y: 12, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 12, scale: 0.95 }}
            transition={{ type: 'spring', stiffness: 500, damping: 40 }}
            role="dialog"
            aria-label={t('nav.more', 'More')}
            className="fixed bottom-[86px] left-1/2 -translate-x-1/2 z-40 w-[min(480px,92vw)] p-3 rounded-2xl bg-white/95 dark:bg-surface-900/95 backdrop-blur-2xl border border-surface-200 dark:border-surface-800 shadow-2xl"
          >
            <div className="flex items-center justify-between mb-2 px-1">
              <span className="text-[11px] font-bold uppercase tracking-wider text-surface-500 dark:text-surface-500">
                {t('nav.more', 'More')}
              </span>
              <button
                type="button"
                onClick={() => setOverflowOpen(false)}
                className="btn btn-ghost btn-icon w-6 h-6"
                aria-label={t('common.close', 'Close')}
              >
                <X weight="bold" className="w-3.5 h-3.5" />
              </button>
            </div>
            <div className="grid grid-cols-3 sm:grid-cols-4 gap-1">
              {overflow.map(item => (
                <NavLink
                  key={item.key}
                  to={item.to}
                  end={item.end}
                  onClick={() => setOverflowOpen(false)}
                  onMouseEnter={() => preloadRoute(item.to)}
                  className={({ isActive }) =>
                    `flex flex-col items-center gap-1 p-2 rounded-lg text-[11px] font-medium transition-colors ${
                      isActive
                        ? 'text-primary-700 dark:text-primary-300 bg-primary-50 dark:bg-primary-950/30'
                        : 'text-surface-600 dark:text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-800'
                    }`
                  }
                >
                  <item.icon weight="fill" className="w-5 h-5" />
                  <span className="truncate w-full text-center">{t(`nav.${item.key}`)}</span>
                </NavLink>
              ))}
              {isAdmin && (
                <NavLink
                  to="/admin"
                  onClick={() => setOverflowOpen(false)}
                  className={({ isActive }) =>
                    `flex flex-col items-center gap-1 p-2 rounded-lg text-[11px] font-medium transition-colors ${
                      isActive
                        ? 'text-primary-700 dark:text-primary-300 bg-primary-50 dark:bg-primary-950/30'
                        : 'text-surface-600 dark:text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-800'
                    }`
                  }
                >
                  <span className="text-lg">⚙</span>
                  <span>{t('nav.admin')}</span>
                </NavLink>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
}

interface DockIconProps {
  item: NavItem;
  active: boolean;
  badge: number;
  mouseX: ReturnType<typeof useMotionValue<number | null>>;
  label: string;
}

/**
 * Single dock icon with pointer-proximity magnification. Uses framer-motion
 * useTransform to map the distance from cursor to a scale (1.0 - 1.4) via
 * a spring for the macOS dock feel.
 */
function DockIcon({ item, active, badge, mouseX, label }: DockIconProps) {
  const ref = useRef<HTMLAnchorElement>(null);
  // Resting distance kept past the magnification ceiling so the scale
  // ramp (0..50..120) bottoms out at 1.0 whenever the cursor isn't on
  // the dock. A naive `return 0` would map the null/idle state to the
  // MAXIMUM scale — every icon permanently zoomed, which is exactly
  // the opposite of the desired "springs up as you approach" feel.
  const REST_DISTANCE = 1_000;
  const distance = useTransform(mouseX, (mx) => {
    if (mx === null || !ref.current) return REST_DISTANCE;
    const rect = ref.current.getBoundingClientRect();
    const parent = ref.current.parentElement?.getBoundingClientRect();
    if (!parent) return REST_DISTANCE;
    const iconCenter = rect.left - parent.left + rect.width / 2;
    return Math.abs(mx - iconCenter);
  });
  const raw = useTransform(distance, [0, 50, 120], [1.35, 1.15, 1]);
  const scale = useSpring(raw, { stiffness: 400, damping: 28 });

  return (
    <motion.div style={{ scale }} className="origin-bottom">
      <NavLink
        ref={ref}
        to={item.to}
        end={item.end}
        aria-label={label}
        aria-current={active ? 'page' : undefined}
        onMouseEnter={() => preloadRoute(item.to)}
        onFocus={() => preloadRoute(item.to)}
        className={`relative flex items-center justify-center w-12 h-12 rounded-xl transition-colors ${
          active
            ? 'text-primary-700 dark:text-primary-300 bg-primary-50 dark:bg-primary-950/30'
            : 'text-surface-600 dark:text-surface-400 hover:text-surface-900 dark:hover:text-white hover:bg-surface-100 dark:hover:bg-surface-800'
        }`}
      >
        <span className="relative">
          <item.icon weight="fill" className="w-6 h-6" />
          {badge > 0 && <NotificationBadge count={badge} />}
        </span>
        {active && (
          <span
            aria-hidden="true"
            className="absolute -bottom-1 left-1/2 -translate-x-1/2 w-1 h-1 rounded-full bg-primary-500"
          />
        )}
      </NavLink>
    </motion.div>
  );
}
