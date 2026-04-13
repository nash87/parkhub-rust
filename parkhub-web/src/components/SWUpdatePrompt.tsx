import { useState, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ArrowsClockwise, X } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';

export function SWUpdatePrompt() {
  const { t } = useTranslation();
  const [waitingWorker, setWaitingWorker] = useState<ServiceWorker | null>(null);
  const [show, setShow] = useState(false);

  useEffect(() => {
    if (!('serviceWorker' in navigator)) return;

    let registration: ServiceWorkerRegistration | null = null;

    function onStateChange(this: ServiceWorker) {
      if (this.state === 'installed' && navigator.serviceWorker.controller) {
        setWaitingWorker(this);
        setShow(true);
      }
    }

    navigator.serviceWorker.ready.then((reg) => {
      registration = reg;

      // Check if there's already a waiting worker
      if (reg.waiting) {
        setWaitingWorker(reg.waiting);
        setShow(true);
        return;
      }

      // Listen for new installing workers
      reg.addEventListener('updatefound', () => {
        const newWorker = reg.installing;
        if (newWorker) {
          newWorker.addEventListener('statechange', onStateChange);
        }
      });
    });

    // When the new SW takes over, reload
    let refreshing = false;
    function onControllerChange() {
      if (refreshing) return;
      refreshing = true;
      window.location.reload();
    }
    navigator.serviceWorker.addEventListener('controllerchange', onControllerChange);

    return () => {
      navigator.serviceWorker.removeEventListener('controllerchange', onControllerChange);
    };
  }, []);

  const handleUpdate = useCallback(() => {
    if (waitingWorker) {
      waitingWorker.postMessage({ type: 'SKIP_WAITING' });
    }
  }, [waitingWorker]);

  const handleDismiss = useCallback(() => {
    setShow(false);
  }, []);

  return (
    <AnimatePresence>
      {show && (
        <motion.div
          initial={{ opacity: 0, y: 40 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: 40 }}
          transition={{ type: 'spring', damping: 25, stiffness: 300 }}
          className="fixed bottom-20 left-1/2 -translate-x-1/2 z-[9999] max-w-sm w-[calc(100%-2rem)]"
        >
          <div className="flex items-center gap-3 px-4 py-3 rounded-xl bg-surface-900 dark:bg-surface-100 text-white dark:text-surface-900 shadow-2xl border border-surface-700 dark:border-surface-300">
            <ArrowsClockwise weight="bold" className="w-5 h-5 text-primary-400 dark:text-primary-600 flex-shrink-0 animate-spin" style={{ animationDuration: '3s' }} />
            <p className="text-sm font-medium flex-1">
              {t('pwa.updateAvailable', 'A new version is available')}
            </p>
            <button
              onClick={handleUpdate}
              className="px-3 py-1.5 text-xs font-semibold rounded-lg bg-primary-500 hover:bg-primary-400 text-white transition-colors"
            >
              {t('pwa.reload', 'Reload')}
            </button>
            <button
              onClick={handleDismiss}
              className="p-1 rounded-lg hover:bg-surface-700 dark:hover:bg-surface-200 transition-colors"
              aria-label={t('common.dismiss', 'Dismiss')}
            >
              <X weight="bold" className="w-4 h-4" />
            </button>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
