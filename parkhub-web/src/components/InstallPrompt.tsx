import { useState, useEffect, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { DownloadSimple, X } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';

interface BeforeInstallPromptEvent extends Event {
  prompt(): Promise<void>;
  userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>;
}

const DISMISS_KEY = 'parkhub_install_dismissed';
const DISMISS_DURATION_MS = 7 * 24 * 60 * 60 * 1000; // 7 days

export function InstallPrompt() {
  const { t } = useTranslation();
  const [deferredPrompt, setDeferredPrompt] = useState<BeforeInstallPromptEvent | null>(null);
  const [visible, setVisible] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [syncMessage, setSyncMessage] = useState<string | null>(null);

  useEffect(() => {
    // Check if already dismissed recently
    const dismissed = localStorage.getItem(DISMISS_KEY);
    if (dismissed && Date.now() - parseInt(dismissed, 10) < DISMISS_DURATION_MS) return;

    // Check if already installed (display-mode: standalone)
    if (window.matchMedia('(display-mode: standalone)').matches) return;

    const handler = (e: Event) => {
      e.preventDefault();
      setDeferredPrompt(e as BeforeInstallPromptEvent);
      setVisible(true);
    };

    window.addEventListener('beforeinstallprompt', handler);
    return () => window.removeEventListener('beforeinstallprompt', handler);
  }, []);

  // Listen for background sync messages from SW
  useEffect(() => {
    const handler = (event: MessageEvent) => {
      if (event.data?.type === 'MUTATION_QUEUED') {
        setSyncing(true);
        setSyncMessage(
          t('pwa.mutationQueued', {
            defaultValue: '{{count}} action(s) queued for sync',
            count: event.data.queueLength,
          })
        );
      }
      if (event.data?.type === 'SYNC_RESULT') {
        if (event.data.synced > 0) {
          setSyncMessage(
            t('pwa.syncComplete', {
              defaultValue: '{{count}} queued action(s) synced successfully',
              count: event.data.synced,
            })
          );
          setTimeout(() => {
            setSyncing(false);
            setSyncMessage(null);
          }, 3000);
        } else {
          setSyncing(false);
          setSyncMessage(null);
        }
      }
    };

    navigator.serviceWorker?.addEventListener('message', handler);
    return () => navigator.serviceWorker?.removeEventListener('message', handler);
  }, [t]);

  // Replay sync queue when coming back online
  useEffect(() => {
    const handler = () => {
      navigator.serviceWorker?.controller?.postMessage({ type: 'REPLAY_SYNC_QUEUE' });
    };
    window.addEventListener('online', handler);
    return () => window.removeEventListener('online', handler);
  }, []);

  const handleInstall = useCallback(async () => {
    if (!deferredPrompt) return;
    await deferredPrompt.prompt();
    const { outcome } = await deferredPrompt.userChoice;
    if (outcome === 'accepted') {
      setVisible(false);
    }
    setDeferredPrompt(null);
  }, [deferredPrompt]);

  const handleDismiss = useCallback(() => {
    setVisible(false);
    setDeferredPrompt(null);
    localStorage.setItem(DISMISS_KEY, Date.now().toString());
  }, []);

  return (
    <>
      {/* Install banner */}
      <AnimatePresence>
        {visible && (
          <motion.div
            initial={{ y: 100, opacity: 0 }}
            animate={{ y: 0, opacity: 1 }}
            exit={{ y: 100, opacity: 0 }}
            transition={{ type: 'spring', damping: 25, stiffness: 300 }}
            className="fixed bottom-4 left-4 right-4 z-50 sm:left-auto sm:right-4 sm:w-96"
            role="complementary"
            aria-label={t('pwa.installBanner', { defaultValue: 'Install ParkHub' })}
          >
            <div className="bg-white dark:bg-surface-800 rounded-2xl shadow-xl border border-surface-200 dark:border-surface-700 p-4">
              <div className="flex items-start gap-3">
                <div className="w-10 h-10 rounded-xl bg-primary-600 flex items-center justify-center flex-shrink-0">
                  <DownloadSimple weight="bold" className="w-5 h-5 text-white" />
                </div>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-semibold text-surface-900 dark:text-white">
                    {t('pwa.installTitle', { defaultValue: 'Install ParkHub' })}
                  </p>
                  <p className="text-xs text-surface-500 dark:text-surface-400 mt-0.5">
                    {t('pwa.installDescription', {
                      defaultValue: 'Add to your home screen for quick access and offline support',
                    })}
                  </p>
                  <div className="flex items-center gap-2 mt-3">
                    <button
                      onClick={handleInstall}
                      className="px-4 py-1.5 text-xs font-semibold text-white bg-primary-600 hover:bg-primary-700 rounded-lg transition-colors"
                    >
                      {t('pwa.install', { defaultValue: 'Install' })}
                    </button>
                    <button
                      onClick={handleDismiss}
                      className="px-3 py-1.5 text-xs font-medium text-surface-500 hover:text-surface-700 dark:hover:text-surface-300 transition-colors"
                    >
                      {t('pwa.notNow', { defaultValue: 'Not now' })}
                    </button>
                  </div>
                </div>
                <button
                  onClick={handleDismiss}
                  className="text-surface-400 hover:text-surface-600 dark:hover:text-surface-300 transition-colors"
                  aria-label={t('pwa.dismiss', { defaultValue: 'Dismiss' })}
                >
                  <X weight="bold" className="w-4 h-4" />
                </button>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Sync status toast */}
      <AnimatePresence>
        {syncing && syncMessage && (
          <motion.div
            initial={{ y: -60, opacity: 0 }}
            animate={{ y: 0, opacity: 1 }}
            exit={{ y: -60, opacity: 0 }}
            transition={{ type: 'spring', damping: 25, stiffness: 300 }}
            className="fixed top-4 left-4 right-4 z-50 sm:left-auto sm:right-4 sm:w-80"
            role="status"
            aria-live="polite"
          >
            <div className="bg-primary-600 text-white rounded-xl shadow-lg px-4 py-3 text-sm font-medium">
              {syncMessage}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
}
