import { useState, useEffect, useCallback } from 'react';
import { Link } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  Plus, X, CalendarPlus, Car, CoinVertical, CalendarCheck,
} from '@phosphor-icons/react';
import { useFeatures } from '../context/FeaturesContext';

/**
 * Floating Action Button for mobile quick actions.
 * Only renders when `fab_quick_actions` feature is enabled.
 * Positioned bottom-right with safe area awareness.
 */
export function QuickActionsFab() {
  const { t } = useTranslation();
  const { isEnabled } = useFeatures();
  const [open, setOpen] = useState(false);

  const handleEscape = useCallback((e: KeyboardEvent) => {
    if (e.key === 'Escape' && open) setOpen(false);
  }, [open]);

  useEffect(() => {
    document.addEventListener('keydown', handleEscape);
    return () => document.removeEventListener('keydown', handleEscape);
  }, [handleEscape]);

  if (!isEnabled('fab_quick_actions')) return null;

  const actions = [
    { to: '/bookings', icon: CalendarCheck, label: t('dashboard.viewBookings'), color: 'bg-blue-500' },
    ...(isEnabled('credits') ? [{ to: '/credits', icon: CoinVertical, label: t('nav.credits'), color: 'bg-emerald-500' }] : []),
    ...(isEnabled('vehicles') ? [{ to: '/vehicles', icon: Car, label: t('dashboard.myVehicles'), color: 'bg-primary-500' }] : []),
    { to: '/book', icon: CalendarPlus, label: t('dashboard.bookSpot'), color: 'bg-accent-500' },
  ];

  return (
    <div className="fixed bottom-6 right-4 z-40 lg:hidden safe-bottom">
      {/* Action items */}
      <AnimatePresence>
        {open && (
          <>
            {/* Backdrop */}
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="fixed inset-0 bg-black/20 backdrop-blur-[2px] -z-10"
              onClick={() => setOpen(false)}
            />

            {/* Action buttons */}
            <div className="flex flex-col-reverse items-end gap-3 mb-3">
              {actions.map((action, i) => (
                <motion.div
                  key={action.to}
                  initial={{ opacity: 0, y: 20, scale: 0.8 }}
                  animate={{ opacity: 1, y: 0, scale: 1 }}
                  exit={{ opacity: 0, y: 10, scale: 0.8 }}
                  transition={{ delay: i * 0.04, type: 'spring', damping: 20, stiffness: 300 }}
                  className="flex items-center gap-2.5"
                >
                  <span className="text-xs font-semibold text-white bg-surface-900/80 dark:bg-surface-800/90 backdrop-blur-sm px-2.5 py-1 rounded-md shadow-sm whitespace-nowrap">
                    {action.label}
                  </span>
                  <Link
                    to={action.to}
                    onClick={() => setOpen(false)}
                    aria-label={action.label}
                    className={`w-11 h-11 rounded-full ${action.color} text-white flex items-center justify-center shadow-lg active:scale-95 transition-transform cursor-pointer`}
                  >
                    <action.icon weight="bold" className="w-5 h-5" aria-hidden="true" />
                  </Link>
                </motion.div>
              ))}
            </div>
          </>
        )}
      </AnimatePresence>

      {/* Main FAB */}
      <motion.button
        onClick={() => setOpen(!open)}
        animate={{ rotate: open ? 135 : 0 }}
        transition={{ type: 'spring', damping: 15, stiffness: 200 }}
        className="w-14 h-14 rounded-full bg-accent-600 text-white flex items-center justify-center shadow-xl shadow-accent-500/30 active:scale-95 transition-transform cursor-pointer"
        aria-label={open ? 'Close quick actions' : 'Open quick actions'}
      >
        {open ? <X weight="bold" className="w-6 h-6" /> : <Plus weight="bold" className="w-6 h-6" />}
      </motion.button>
    </div>
  );
}
