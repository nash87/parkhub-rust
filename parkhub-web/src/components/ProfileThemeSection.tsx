import { Check, Palette } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import { useTheme, type DesignThemeId, type DesignThemeInfo } from '../context/ThemeContext';

function MiniThemeCard({ theme, isActive, onApply, resolved }: {
  theme: DesignThemeInfo;
  isActive: boolean;
  onApply: (id: DesignThemeId) => void;
  resolved: 'light' | 'dark';
}) {
  const { t } = useTranslation();
  const colors = resolved === 'dark' ? theme.previewColors.dark : theme.previewColors.light;
  const [bg, card, accent, text, border] = colors;

  return (
    <button
      onClick={() => onApply(theme.id)}
      className={`relative text-left rounded-lg border-2 p-3 transition-all hover:shadow-sm ${
        isActive
          ? 'border-primary-500 ring-1 ring-primary-500/20'
          : 'border-surface-200 dark:border-surface-700 hover:border-surface-300 dark:hover:border-surface-600'
      }`}
      aria-pressed={isActive}
      aria-label={t('themes.applyTheme', { name: theme.name })}
    >
      {isActive && (
        <div className="absolute top-1.5 right-1.5 w-5 h-5 rounded-full bg-primary-500 flex items-center justify-center">
          <Check weight="bold" className="w-3 h-3 text-white" />
        </div>
      )}

      {/* Mini preview */}
      <div
        className="w-full aspect-[3/2] rounded overflow-hidden mb-2 border"
        style={{ backgroundColor: bg, borderColor: border }}
      >
        <div className="flex h-full">
          <div className="w-1/4 h-full p-1" style={{ borderRight: `1px solid ${border}` }}>
            <div className="w-full h-1 rounded-full mb-0.5" style={{ backgroundColor: accent }} />
            <div className="w-3/4 h-0.5 rounded-full opacity-30" style={{ backgroundColor: text }} />
            <div className="w-3/4 h-0.5 rounded-full opacity-30 mt-0.5" style={{ backgroundColor: text }} />
          </div>
          <div className="flex-1 p-1.5 flex flex-col gap-1">
            <div className="w-1/2 h-1 rounded-full" style={{ backgroundColor: text }} />
            <div className="flex gap-1 flex-1">
              <div className="flex-1 rounded-sm" style={{ backgroundColor: card, border: `1px solid ${border}` }} />
              <div className="flex-1 rounded-sm" style={{ backgroundColor: card, border: `1px solid ${border}` }} />
            </div>
          </div>
        </div>
      </div>

      <p className="text-xs font-semibold text-surface-900 dark:text-white">
        {t(`themes.names.${theme.id}`, theme.name)}
      </p>
    </button>
  );
}

export function ProfileThemeSection() {
  const { t } = useTranslation();
  const { designTheme, setDesignTheme, designThemes, resolved } = useTheme();

  return (
    <div>
      <div className="flex items-center gap-2 mb-4">
        <Palette weight="fill" className="w-5 h-5 text-primary-500" />
        <h3 className="text-base font-semibold text-surface-900 dark:text-white">
          {t('themes.title', 'Design Themes')}
        </h3>
      </div>
      <p className="text-sm text-surface-500 dark:text-surface-400 mb-4">
        {t('themes.subtitle', 'Choose a visual design for your ParkHub experience.')}
      </p>
      <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
        {designThemes.map(theme => (
          <MiniThemeCard
            key={theme.id}
            theme={theme}
            isActive={designTheme === theme.id}
            onApply={setDesignTheme}
            resolved={resolved}
          />
        ))}
      </div>
    </div>
  );
}
