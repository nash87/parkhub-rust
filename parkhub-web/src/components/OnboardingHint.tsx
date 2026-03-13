import { useState, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { X, Lightbulb } from '@phosphor-icons/react';
import { useFeatures } from '../context/FeaturesContext';

const STORAGE_PREFIX = 'parkhub_hint_';

interface Props {
  /** Unique ID for this hint — used to track dismissal */
  id: string;
  /** The hint message */
  message: string;
  /** Optional icon override */
  icon?: React.ElementType;
  /** Position relative to the trigger. Default: 'bottom' */
  position?: 'top' | 'bottom';
  /** Additional className */
  className?: string;
}

/**
 * Onboarding hint tooltip.
 * Shows once per hint ID, remembers dismissal in localStorage.
 * Only active when `onboarding_hints` feature is enabled.
 */
export function OnboardingHint({ id, message, icon: Icon = Lightbulb, position = 'bottom', className = '' }: Props) {
  const { isEnabled } = useFeatures();
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (!isEnabled('onboarding_hints')) return;
    const dismissed = localStorage.getItem(STORAGE_PREFIX + id);
    if (!dismissed) {
      // Delay to let page content render first
      const timer = setTimeout(() => setVisible(true), 800);
      return () => clearTimeout(timer);
    }
  }, [id, isEnabled]);

  const dismiss = useCallback(() => {
    setVisible(false);
    localStorage.setItem(STORAGE_PREFIX + id, '1');
  }, [id]);

  if (!isEnabled('onboarding_hints')) return null;

  return (
    <AnimatePresence>
      {visible && (
        <motion.div
          initial={{ opacity: 0, y: position === 'bottom' ? -6 : 6, scale: 0.95 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: position === 'bottom' ? -6 : 6, scale: 0.95 }}
          transition={{ type: 'spring', damping: 20, stiffness: 300 }}
          className={`absolute ${position === 'bottom' ? 'top-full mt-2' : 'bottom-full mb-2'} left-0 right-0 z-30 ${className}`}
        >
          <div className="bg-accent-50 dark:bg-accent-900/20 border border-accent-200 dark:border-accent-800/40 rounded-lg p-3 shadow-md">
            {/* Arrow */}
            <div
              className={`absolute ${position === 'bottom' ? '-top-1.5' : '-bottom-1.5'} left-6 w-3 h-3 bg-accent-50 dark:bg-accent-900/20 border-accent-200 dark:border-accent-800/40 rotate-45 ${
                position === 'bottom' ? 'border-l border-t' : 'border-r border-b'
              }`}
            />
            <div className="relative flex items-start gap-2.5">
              <Icon weight="fill" className="w-4 h-4 text-accent-500 flex-shrink-0 mt-0.5" />
              <p className="text-xs text-accent-800 dark:text-accent-300 leading-relaxed flex-1 font-medium">
                {message}
              </p>
              <button
                onClick={dismiss}
                className="flex-shrink-0 w-5 h-5 flex items-center justify-center rounded-md text-accent-400 hover:text-accent-600 hover:bg-accent-100 dark:hover:bg-accent-800/30 transition-colors cursor-pointer"
                aria-label="Dismiss hint"
              >
                <X weight="bold" className="w-3 h-3" />
              </button>
            </div>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

/**
 * Reset all hint dismissals (useful for admin/testing).
 */
export function resetAllHints() {
  const keys = Object.keys(localStorage).filter(k => k.startsWith(STORAGE_PREFIX));
  keys.forEach(k => localStorage.removeItem(k));
}
