import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  Buildings, House, UsersThree, Car, ArrowRight, Check,
  SunDim, Moon,
} from '@phosphor-icons/react';
import { useUseCase, type UseCase } from '../context/UseCaseContext';
import { useTheme } from '../context/ThemeContext';

const USE_CASES: {
  id: UseCase;
  icon: React.ElementType;
  iconBg: string;
  ring: string;
  check: string;
  border: string;
}[] = [
  {
    id: 'business',
    icon: Buildings,
    iconBg: 'bg-accent-100 dark:bg-accent-900/20 text-accent-600 dark:text-accent-400',
    ring: 'ring-accent-500',
    check: 'bg-accent-500',
    border: 'border-accent-400 dark:border-accent-600',
  },
  {
    id: 'residential',
    icon: House,
    iconBg: 'bg-emerald-100 dark:bg-emerald-900/20 text-emerald-600 dark:text-emerald-400',
    ring: 'ring-emerald-500',
    check: 'bg-emerald-500',
    border: 'border-emerald-400 dark:border-emerald-600',
  },
  {
    id: 'personal',
    icon: UsersThree,
    iconBg: 'bg-indigo-100 dark:bg-indigo-900/20 text-indigo-600 dark:text-indigo-400',
    ring: 'ring-indigo-500',
    check: 'bg-indigo-500',
    border: 'border-indigo-400 dark:border-indigo-600',
  },
];

export function UseCaseSelectorPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { setUseCase } = useUseCase();
  const { resolved, setTheme } = useTheme();
  const [selected, setSelected] = useState<UseCase | null>(null);
  const [confirmed, setConfirmed] = useState(false);

  function handleContinue() {
    if (!selected) return;
    setConfirmed(true);
    setUseCase(selected);
    setTimeout(() => navigate('/welcome'), 350);
  }

  return (
    <div className="min-h-dvh parking-grid relative overflow-hidden">
      {/* Architectural lines */}
      <div className="absolute inset-0 overflow-hidden pointer-events-none">
        <div className="absolute top-0 left-1/2 -translate-x-1/2 w-px h-32 bg-gradient-to-b from-accent-500/30 to-transparent" />
        <div className="absolute top-[10%] right-[7%] w-px h-16 bg-accent-500/10" />
        <div className="absolute bottom-[18%] left-[5%] w-16 h-px bg-surface-300/20 dark:bg-surface-700/20" />
      </div>

      {/* Theme toggle */}
      <div className="absolute top-5 right-5 z-10">
        <button
          onClick={() => setTheme(resolved === 'dark' ? 'light' : 'dark')}
          className="btn btn-ghost btn-icon border border-surface-200 dark:border-surface-800"
          aria-label="Toggle theme"
        >
          {resolved === 'dark'
            ? <SunDim weight="bold" className="w-4 h-4 text-accent-400" />
            : <Moon weight="bold" className="w-4 h-4 text-surface-500" />}
        </button>
      </div>

      <div className="relative z-10 flex flex-col items-center justify-center min-h-dvh px-6 py-12">
        {/* Logo */}
        <motion.div
          initial={{ opacity: 0, y: -16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.4, ease: [0.22, 1, 0.36, 1] }}
          className="mb-7"
        >
          <div className="w-12 h-12 bg-primary-900 dark:bg-surface-800 flex items-center justify-center border border-primary-800 dark:border-surface-700">
            <Car weight="fill" className="w-6 h-6 text-accent-500" />
          </div>
        </motion.div>

        {/* Heading */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="text-center mb-8 max-w-lg"
        >
          <h1 className="text-2xl sm:text-3xl font-bold tracking-tight text-primary-900 dark:text-surface-100 mb-2">
            {t('useCase.title')}
          </h1>
          <p className="text-surface-500 dark:text-surface-400 text-base">
            {t('useCase.subtitle')}
          </p>
        </motion.div>

        {/* Use-case cards */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.25 }}
          className="grid grid-cols-1 sm:grid-cols-3 gap-3 max-w-2xl w-full mb-8"
        >
          {USE_CASES.map((uc, i) => {
            const isSelected = selected === uc.id;
            return (
              <motion.button
                key={uc.id}
                initial={{ opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.35 + i * 0.08 }}
                whileHover={{ y: -3 }}
                whileTap={{ scale: 0.98 }}
                onClick={() => setSelected(uc.id)}
                className={`relative card p-5 text-left transition-all cursor-pointer group ${
                  isSelected
                    ? `ring-2 ${uc.ring} ${uc.border} shadow-md`
                    : 'hover:shadow-sm'
                }`}
              >
                {/* Check indicator */}
                <AnimatePresence>
                  {isSelected && (
                    <motion.div
                      initial={{ scale: 0 }}
                      animate={{ scale: 1 }}
                      exit={{ scale: 0 }}
                      className={`absolute top-3 right-3 w-5 h-5 ${uc.check} flex items-center justify-center`}
                    >
                      <Check weight="bold" className="w-3 h-3 text-white" />
                    </motion.div>
                  )}
                </AnimatePresence>

                <div className="relative z-10">
                  <div className={`w-10 h-10 mb-3 flex items-center justify-center ${uc.iconBg}`}>
                    <uc.icon weight="bold" className="w-5 h-5" />
                  </div>
                  <h3 className="text-base font-semibold text-surface-900 dark:text-white mb-1">
                    {t(`useCase.${uc.id}.name`)}
                  </h3>
                  <p className="text-xs text-surface-500 dark:text-surface-400 leading-relaxed mb-3">
                    {t(`useCase.${uc.id}.desc`)}
                  </p>

                  {/* Feature bullets */}
                  <ul className="space-y-1">
                    {[1, 2, 3].map(n => (
                      <li key={n} className="flex items-center gap-2 text-[11px] text-surface-500 dark:text-surface-400">
                        <div className={`w-1 h-1 ${isSelected ? uc.check : 'bg-surface-300 dark:bg-surface-600'}`} />
                        {t(`useCase.${uc.id}.feature${n}`)}
                      </li>
                    ))}
                  </ul>
                </div>
              </motion.button>
            );
          })}
        </motion.div>

        {/* Continue button */}
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.55 }}
        >
          <AnimatePresence mode="wait">
            {confirmed ? (
              <motion.div
                key="confirmed"
                initial={{ scale: 0.9, opacity: 0 }}
                animate={{ scale: 1, opacity: 1 }}
                className="flex items-center gap-2 text-accent-600 dark:text-accent-400 font-semibold text-sm"
              >
                <Check weight="bold" className="w-4 h-4" />
                {t('useCase.applying')}
              </motion.div>
            ) : (
              <motion.button
                key="continue"
                whileHover={{ scale: selected ? 1.02 : 1 }}
                whileTap={{ scale: selected ? 0.98 : 1 }}
                onClick={handleContinue}
                disabled={!selected}
                className="btn btn-primary text-sm px-7 py-3 disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
              >
                {t('useCase.continue')}
                <ArrowRight weight="bold" className="w-4 h-4" />
              </motion.button>
            )}
          </AnimatePresence>
        </motion.div>

        {/* Skip link */}
        <motion.button
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.8 }}
          onClick={() => {
            setUseCase('business');
            navigate('/welcome');
          }}
          className="mt-4 text-xs text-surface-400 hover:text-surface-600 dark:hover:text-surface-300 transition-colors cursor-pointer uppercase tracking-wider"
        >
          {t('useCase.skip')}
        </motion.button>
      </div>
    </div>
  );
}
