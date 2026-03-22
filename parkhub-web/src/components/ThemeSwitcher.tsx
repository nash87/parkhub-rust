import { motion, AnimatePresence } from 'framer-motion';
import { Check, X, Palette } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import { useTheme, type DesignThemeId, type DesignThemeInfo } from '../context/ThemeContext';

// ── Theme Preview Card ──

function ThemePreviewCard({ theme, isActive, onApply }: {
  theme: DesignThemeInfo;
  isActive: boolean;
  onApply: (id: DesignThemeId) => void;
}) {
  const { t } = useTranslation();
  const { resolved } = useTheme();
  const colors = resolved === 'dark' ? theme.previewColors.dark : theme.previewColors.light;
  const [bg, card, accent, text, border] = colors;

  return (
    <motion.button
      onClick={() => onApply(theme.id)}
      whileHover={{ scale: 1.02 }}
      whileTap={{ scale: 0.98 }}
      transition={{ type: 'spring', stiffness: 400, damping: 25 }}
      className={`relative group text-left rounded-xl border-2 p-4 transition-colors ${
        isActive
          ? 'border-primary-500 ring-2 ring-primary-500/20'
          : 'border-surface-200 dark:border-surface-700 hover:border-surface-300 dark:hover:border-surface-600'
      }`}
      aria-pressed={isActive}
      aria-label={t('themes.applyTheme', { name: theme.name })}
    >
      {/* Active indicator */}
      {isActive && (
        <motion.div
          initial={{ scale: 0 }}
          animate={{ scale: 1 }}
          className="absolute top-2 right-2 w-6 h-6 rounded-full bg-primary-500 flex items-center justify-center"
        >
          <Check weight="bold" className="w-3.5 h-3.5 text-white" />
        </motion.div>
      )}

      {/* Mini layout preview */}
      <div
        className="w-full aspect-[16/10] rounded-lg overflow-hidden mb-3 border"
        style={{ backgroundColor: bg, borderColor: border }}
      >
        <div className="flex h-full">
          {/* Mini sidebar */}
          <div
            className="w-1/4 h-full p-1.5 flex flex-col gap-1"
            style={{ borderRight: `1px solid ${border}` }}
          >
            <div className="w-full h-1.5 rounded-full" style={{ backgroundColor: accent }} />
            <div className="w-3/4 h-1 rounded-full opacity-40" style={{ backgroundColor: text }} />
            <div className="w-3/4 h-1 rounded-full opacity-40" style={{ backgroundColor: text }} />
            <div className="w-3/4 h-1 rounded-full opacity-40" style={{ backgroundColor: text }} />
          </div>
          {/* Mini content */}
          <div className="flex-1 p-2 flex flex-col gap-1.5">
            <div className="w-2/3 h-1.5 rounded-full" style={{ backgroundColor: text }} />
            <div className="flex gap-1.5 flex-1">
              <div className="flex-1 rounded" style={{ backgroundColor: card, border: `1px solid ${border}` }} />
              <div className="flex-1 rounded" style={{ backgroundColor: card, border: `1px solid ${border}` }} />
            </div>
          </div>
        </div>
      </div>

      {/* Theme info */}
      <h3 className="font-semibold text-sm text-surface-900 dark:text-white">
        {t(`themes.names.${theme.id}`, theme.name)}
      </h3>
      <p className="text-xs text-surface-500 dark:text-surface-400 mt-0.5 line-clamp-2">
        {t(`themes.descriptions.${theme.id}`, theme.description)}
      </p>

      {/* Color swatches */}
      <div className="flex gap-1 mt-2">
        {colors.map((color, i) => (
          <div
            key={i}
            className="w-4 h-4 rounded-full border border-surface-200 dark:border-surface-600"
            style={{ backgroundColor: color }}
            aria-hidden="true"
          />
        ))}
      </div>
    </motion.button>
  );
}

// ── Theme Switcher Panel ──

export function ThemeSwitcher({ open, onClose }: { open: boolean; onClose: () => void }) {
  const { t } = useTranslation();
  const { designTheme, setDesignTheme, designThemes } = useTheme();

  return (
    <AnimatePresence>
      {open && (
        <>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black/30 backdrop-blur-sm z-50"
            onClick={onClose}
            aria-hidden="true"
          />
          {/* Panel */}
          <motion.div
            initial={{ opacity: 0, y: 20, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 20, scale: 0.96 }}
            transition={{ type: 'spring', stiffness: 400, damping: 30 }}
            className="fixed inset-x-4 bottom-4 sm:inset-auto sm:right-4 sm:bottom-4 sm:w-[480px] z-50
              bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-700
              shadow-2xl max-h-[80vh] overflow-hidden flex flex-col"
            role="dialog"
            aria-label={t('themes.title', 'Design Themes')}
          >
            {/* Header */}
            <div className="flex items-center justify-between p-4 border-b border-surface-200 dark:border-surface-800">
              <div className="flex items-center gap-2">
                <Palette weight="fill" className="w-5 h-5 text-primary-500" />
                <h2 className="text-lg font-bold text-surface-900 dark:text-white">
                  {t('themes.title', 'Design Themes')}
                </h2>
              </div>
              <button
                onClick={onClose}
                className="btn btn-ghost btn-icon"
                aria-label={t('common.close', 'Close')}
              >
                <X weight="bold" className="w-4 h-4" />
              </button>
            </div>

            {/* Theme grid */}
            <div className="p-4 overflow-y-auto flex-1">
              <p className="text-sm text-surface-500 dark:text-surface-400 mb-4">
                {t('themes.subtitle', 'Choose a visual design for your ParkHub experience.')}
              </p>
              <div className="grid grid-cols-2 gap-3">
                {designThemes.map(theme => (
                  <ThemePreviewCard
                    key={theme.id}
                    theme={theme}
                    isActive={designTheme === theme.id}
                    onApply={setDesignTheme}
                  />
                ))}
              </div>
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}

// ── Floating Theme Button ──

export function ThemeSwitcherFab({ onClick }: { onClick: () => void }) {
  const { t } = useTranslation();

  return (
    <motion.button
      onClick={onClick}
      whileHover={{ scale: 1.05 }}
      whileTap={{ scale: 0.95 }}
      className="fixed bottom-4 left-4 z-40 w-10 h-10 rounded-full bg-surface-100 dark:bg-surface-800
        border border-surface-200 dark:border-surface-700 shadow-lg
        flex items-center justify-center hover:bg-surface-200 dark:hover:bg-surface-700 transition-colors"
      aria-label={t('themes.openSwitcher', 'Change theme')}
    >
      <Palette weight="fill" className="w-5 h-5 text-surface-600 dark:text-surface-300" />
    </motion.button>
  );
}
