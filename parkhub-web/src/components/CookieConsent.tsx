import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { CookieIcon, ShieldCheck, X } from '@phosphor-icons/react';

const STORAGE_KEY = 'cookie_consent_accepted';

export function CookieConsent() {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    // Only show if not yet accepted
    const accepted = localStorage.getItem(STORAGE_KEY);
    if (!accepted) {
      // Small delay so it doesn't flash on initial render
      const timer = setTimeout(() => setVisible(true), 800);
      return () => clearTimeout(timer);
    }
  }, []);

  function handleAccept() {
    localStorage.setItem(STORAGE_KEY, 'true');
    setVisible(false);
  }

  return (
    <AnimatePresence>
      {visible && (
        <motion.div
          initial={{ opacity: 0, y: 40 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: 40 }}
          transition={{ type: 'spring', stiffness: 300, damping: 30 }}
          role="dialog"
          aria-label="Cookie-Hinweis"
          aria-live="polite"
          className="fixed bottom-4 left-4 right-4 md:left-auto md:right-6 md:bottom-6 md:max-w-sm z-50"
        >
          <div className="card p-5 shadow-xl border border-gray-200 dark:border-gray-700">
            <div className="flex items-start justify-between gap-3 mb-3">
              <div className="flex items-center gap-2">
                <div className="w-8 h-8 bg-primary-100 dark:bg-primary-900/30 rounded-lg flex items-center justify-center shrink-0">
                  <CookieIcon weight="fill" className="w-4 h-4 text-primary-600 dark:text-primary-400" aria-hidden="true" />
                </div>
                <h2 className="text-sm font-semibold text-gray-900 dark:text-white">
                  Cookie-Hinweis
                </h2>
              </div>
              <button
                onClick={handleAccept}
                className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-200 transition-colors p-0.5 rounded"
                aria-label="Schließen"
              >
                <X weight="bold" className="w-4 h-4" aria-hidden="true" />
              </button>
            </div>

            <p className="text-sm text-gray-600 dark:text-gray-300 mb-4 leading-relaxed">
              Diese Website verwendet ausschließlich{' '}
              <span className="font-medium text-gray-900 dark:text-white">technisch notwendige</span>{' '}
              Cookies und lokalen Speicher für die Authentifizierung. Keine Tracking-Cookies,
              kein Google Analytics, keine Werbung.
            </p>

            <div className="flex items-center gap-2 mb-4 text-xs text-gray-500 dark:text-gray-400">
              <ShieldCheck weight="fill" className="w-3.5 h-3.5 text-emerald-500 shrink-0" aria-hidden="true" />
              <span>TTDSG §25 — nur technisch notwendige Dienste</span>
            </div>

            <div className="flex items-center gap-2">
              <button
                onClick={handleAccept}
                className="btn btn-primary btn-sm flex-1"
                autoFocus
              >
                Verstanden
              </button>
              <a
                href="/datenschutz"
                className="btn btn-ghost btn-sm text-gray-500 dark:text-gray-400"
              >
                Mehr erfahren
              </a>
            </div>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
