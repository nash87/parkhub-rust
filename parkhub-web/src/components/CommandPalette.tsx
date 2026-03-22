import { useState, useRef, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  CalendarCheck, Car, UserCircle, Users, GearSix, Coins, Calendar, CalendarPlus,
} from '@phosphor-icons/react';

interface Action {
  labelKey: string;
  path: string;
  icon: React.ComponentType<any>;
  shortcut?: string;
}

const ACTIONS: Action[] = [
  { labelKey: 'dashboard.bookSpot', path: '/book', icon: CalendarPlus, shortcut: 'Ctrl+B' },
  { labelKey: 'nav.bookings', path: '/bookings', icon: CalendarCheck },
  { labelKey: 'nav.vehicles', path: '/vehicles', icon: Car },
  { labelKey: 'nav.profile', path: '/profile', icon: UserCircle },
  { labelKey: 'nav.admin', path: '/admin', icon: GearSix },
  { labelKey: 'nav.credits', path: '/credits', icon: Coins },
  { labelKey: 'nav.calendar', path: '/calendar', icon: Calendar },
  { labelKey: 'nav.team', path: '/team', icon: Users },
];

export function CommandPalette({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);

  const filtered = ACTIONS.filter(a => t(a.labelKey).toLowerCase().includes(query.toLowerCase()));

  useEffect(() => {
    if (open) { setQuery(''); setSelectedIndex(0); requestAnimationFrame(() => inputRef.current?.focus()); }
  }, [open]);

  const go = useCallback((path: string) => { onClose(); navigate(path); }, [navigate, onClose]);

  useEffect(() => {
    if (!open) return;
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') { e.preventDefault(); onClose(); return; }
      if (e.key === 'ArrowDown') { e.preventDefault(); setSelectedIndex(i => (i + 1) % (filtered.length || 1)); return; }
      if (e.key === 'ArrowUp') { e.preventDefault(); setSelectedIndex(i => (i - 1 + (filtered.length || 1)) % (filtered.length || 1)); return; }
      if (e.key === 'Enter') { e.preventDefault(); if (filtered[selectedIndex]) go(filtered[selectedIndex].path); return; }
    }
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [open, filtered, selectedIndex, go, onClose]);

  useEffect(() => {
    if (selectedIndex >= filtered.length) setSelectedIndex(Math.max(0, filtered.length - 1));
  }, [filtered.length, selectedIndex]);

  return (
    <AnimatePresence>
      {open && (
        <>
          <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.15 }}
            className="fixed inset-0 bg-black/40 backdrop-blur-sm z-50" onClick={onClose} data-testid="command-palette-backdrop" />
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: -10 }} animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95, y: -10 }} transition={{ type: 'spring', stiffness: 400, damping: 30 }}
            className="fixed inset-0 z-50 flex items-start justify-center pt-[20vh]"
          >
            <div className="w-full max-w-md glass-modal overflow-hidden shadow-2xl" role="dialog" aria-label="Command palette">
              <div className="relative border-b border-surface-200/50 dark:border-surface-700/50 px-4 py-3">
                <input ref={inputRef} type="text" placeholder={t('commandPalette.placeholder')} value={query}
                  onChange={e => setQuery(e.target.value)}
                  className="w-full bg-transparent text-sm text-surface-900 dark:text-white placeholder:text-surface-400 outline-none"
                  data-testid="command-palette-input" />
                <motion.div className="absolute bottom-0 left-0 right-0 h-[2px]"
                  style={{ background: 'linear-gradient(90deg, var(--color-primary-500), var(--color-accent-400), var(--color-primary-500))', backgroundSize: '200% 100%' }}
                  animate={{ backgroundPosition: ['0% 0%', '200% 0%'] }} transition={{ duration: 3, repeat: Infinity, ease: 'linear' }} />
              </div>
              <ul className="max-h-64 overflow-y-auto py-1">
                {filtered.length === 0 && (
                  <li className="px-4 py-3 text-sm text-surface-500 dark:text-surface-400">{t('commandPalette.noResults')}</li>
                )}
                {filtered.map((action, i) => (
                  <li key={action.path}>
                    <button onClick={() => go(action.path)}
                      className={`w-full flex items-center gap-3 px-4 py-2.5 text-sm text-left transition-all ${
                        i === selectedIndex ? 'bg-primary-50/80 dark:bg-primary-950/30 text-primary-700 dark:text-primary-300'
                        : 'text-surface-700 dark:text-surface-300 hover:bg-surface-50 dark:hover:bg-surface-800'
                      }`}
                      data-testid={`command-action-${t(action.labelKey).toLowerCase().replace(/\s+/g, '-')}`}
                    >
                      <action.icon weight="fill" className={`w-4 h-4 shrink-0 transition-colors ${i === selectedIndex ? 'text-primary-500' : ''}`} />
                      <span className="flex-1">{t(action.labelKey)}</span>
                      {action.shortcut && (
                        <kbd className="text-xs font-mono px-1.5 py-0.5 rounded bg-surface-100 dark:bg-surface-700 text-surface-500 dark:text-surface-400 border border-surface-200 dark:border-surface-600">
                          {action.shortcut}
                        </kbd>
                      )}
                    </button>
                  </li>
                ))}
              </ul>
              <div className="border-t border-surface-200/50 dark:border-surface-700/50 px-4 py-2 flex items-center gap-4 text-xs text-surface-400 dark:text-surface-500">
                <span><kbd className="font-mono bg-surface-100 dark:bg-surface-800 px-1 rounded">↑↓</kbd> {t('commandPalette.navigate')}</span>
                <span><kbd className="font-mono bg-surface-100 dark:bg-surface-800 px-1 rounded">↵</kbd> {t('commandPalette.select')}</span>
                <span><kbd className="font-mono bg-surface-100 dark:bg-surface-800 px-1 rounded">esc</kbd> {t('commandPalette.closeLabel')}</span>
              </div>
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
