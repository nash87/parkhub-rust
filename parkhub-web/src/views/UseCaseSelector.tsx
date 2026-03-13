import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  Buildings, House, UsersThree, Car, ArrowRight, Check,
} from '@phosphor-icons/react';
import { useUseCase, type UseCase } from '../context/UseCaseContext';
import { useTheme } from '../context/ThemeContext';
import { SunDim, Moon } from '@phosphor-icons/react';

const USE_CASES: {
  id: UseCase;
  icon: React.ElementType;
  gradient: string;
  iconBg: string;
  ring: string;
  check: string;
}[] = [
  {
    id: 'business',
    icon: Buildings,
    gradient: 'from-orange-500/10 via-amber-500/5 to-transparent',
    iconBg: 'bg-orange-100 dark:bg-orange-900/20 text-orange-600 dark:text-orange-400',
    ring: 'ring-orange-500',
    check: 'bg-orange-500',
  },
  {
    id: 'residential',
    icon: House,
    gradient: 'from-emerald-500/10 via-green-500/5 to-transparent',
    iconBg: 'bg-emerald-100 dark:bg-emerald-900/20 text-emerald-600 dark:text-emerald-400',
    ring: 'ring-emerald-500',
    check: 'bg-emerald-500',
  },
  {
    id: 'personal',
    icon: UsersThree,
    gradient: 'from-indigo-500/10 via-violet-500/5 to-transparent',
    iconBg: 'bg-indigo-100 dark:bg-indigo-900/20 text-indigo-600 dark:text-indigo-400',
    ring: 'ring-indigo-500',
    check: 'bg-indigo-500',
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
    setTimeout(() => navigate('/welcome'), 400);
  }

  return (
    <div className="min-h-dvh parking-grid relative overflow-hidden">
      {/* Decorative */}
      <div className="absolute inset-0 overflow-hidden pointer-events-none">
        <div className="absolute top-0 left-1/2 -translate-x-1/2 w-px h-32 bg-gradient-to-b from-accent-500/30 to-transparent" />
        <motion.div
          animate={{ rotate: [0, 90, 0] }}
          transition={{ duration: 20, repeat: Infinity, ease: 'linear' }}
          className="absolute top-[8%] right-[6%] w-20 h-20 border-2 border-surface-300/20 dark:border-surface-700/20 rounded-lg"
        />
        <motion.div
          animate={{ rotate: [0, -90, 0] }}
          transition={{ duration: 28, repeat: Infinity, ease: 'linear' }}
          className="absolute bottom-[12%] left-[4%] w-32 h-32 border-2 border-surface-300/15 dark:border-surface-700/15 rounded-lg"
        />
      </div>

      {/* Theme toggle */}
      <div className="absolute top-6 right-6 z-10">
        <button
          onClick={() => setTheme(resolved === 'dark' ? 'light' : 'dark')}
          className="btn btn-ghost btn-icon border border-surface-200 dark:border-surface-800"
          aria-label="Toggle theme"
        >
          {resolved === 'dark'
            ? <SunDim weight="bold" className="w-5 h-5 text-accent-400" />
            : <Moon weight="bold" className="w-5 h-5 text-surface-500" />}
        </button>
      </div>

      <div className="relative z-10 flex flex-col items-center justify-center min-h-dvh px-6 py-12">
        {/* Logo */}
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, ease: [0.22, 1, 0.36, 1] }}
          className="mb-8"
        >
          <div className="w-14 h-14 rounded-2xl bg-primary-800 dark:bg-surface-800 flex items-center justify-center shadow-lg border border-primary-700 dark:border-surface-700">
            <Car weight="fill" className="w-7 h-7 text-accent-500" />
          </div>
        </motion.div>

        {/* Heading */}
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.15 }}
          className="text-center mb-10 max-w-lg"
        >
          <h1 className="text-3xl sm:text-4xl font-bold tracking-tight text-primary-800 dark:text-surface-100 mb-3">
            {t('useCase.title')}
          </h1>
          <p className="text-surface-500 dark:text-surface-400 text-lg">
            {t('useCase.subtitle')}
          </p>
        </motion.div>

        {/* Use-case cards */}
        <motion.div
          initial={{ opacity: 0, y: 24 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.3 }}
          className="grid grid-cols-1 sm:grid-cols-3 gap-5 max-w-3xl w-full mb-10"
        >
          {USE_CASES.map((uc, i) => {
            const isSelected = selected === uc.id;
            return (
              <motion.button
                key={uc.id}
                initial={{ opacity: 0, y: 16 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.4 + i * 0.1 }}
                whileHover={{ y: -4 }}
                whileTap={{ scale: 0.97 }}
                onClick={() => setSelected(uc.id)}
                className={`relative card p-6 text-left transition-all cursor-pointer group ${
                  isSelected
                    ? `ring-2 ${uc.ring} shadow-lg`
                    : 'hover:shadow-md'
                }`}
              >
                {/* Gradient overlay */}
                <div className={`absolute inset-0 rounded-2xl bg-gradient-to-br ${uc.gradient} pointer-events-none transition-opacity ${isSelected ? 'opacity-100' : 'opacity-0 group-hover:opacity-60'}`} />

                {/* Check indicator */}
                <AnimatePresence>
                  {isSelected && (
                    <motion.div
                      initial={{ scale: 0 }}
                      animate={{ scale: 1 }}
                      exit={{ scale: 0 }}
                      className={`absolute top-3 right-3 w-6 h-6 rounded-full ${uc.check} flex items-center justify-center`}
                    >
                      <Check weight="bold" className="w-3.5 h-3.5 text-white" />
                    </motion.div>
                  )}
                </AnimatePresence>

                <div className="relative z-10">
                  <div className={`w-12 h-12 rounded-xl mb-4 flex items-center justify-center ${uc.iconBg}`}>
                    <uc.icon weight="bold" className="w-6 h-6" />
                  </div>
                  <h3 className="text-lg font-semibold text-surface-900 dark:text-white mb-1.5">
                    {t(`useCase.${uc.id}.name`)}
                  </h3>
                  <p className="text-sm text-surface-500 dark:text-surface-400 leading-relaxed">
                    {t(`useCase.${uc.id}.desc`)}
                  </p>

                  {/* Feature bullets */}
                  <ul className="mt-4 space-y-1.5">
                    {[1, 2, 3].map(n => (
                      <li key={n} className="flex items-center gap-2 text-xs text-surface-500 dark:text-surface-400">
                        <div className={`w-1 h-1 rounded-full ${isSelected ? uc.check : 'bg-surface-300 dark:bg-surface-600'}`} />
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
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.7 }}
        >
          <AnimatePresence mode="wait">
            {confirmed ? (
              <motion.div
                key="confirmed"
                initial={{ scale: 0.8, opacity: 0 }}
                animate={{ scale: 1, opacity: 1 }}
                className="flex items-center gap-2 text-accent-600 dark:text-accent-400 font-semibold"
              >
                <Check weight="bold" className="w-5 h-5" />
                {t('useCase.applying')}
              </motion.div>
            ) : (
              <motion.button
                key="continue"
                whileHover={{ scale: selected ? 1.02 : 1 }}
                whileTap={{ scale: selected ? 0.98 : 1 }}
                onClick={handleContinue}
                disabled={!selected}
                className="btn btn-primary text-base px-8 py-3.5 disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
              >
                {t('useCase.continue')}
                <ArrowRight weight="bold" className="w-5 h-5" />
              </motion.button>
            )}
          </AnimatePresence>
        </motion.div>

        {/* Skip link */}
        <motion.button
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 1 }}
          onClick={() => {
            setUseCase('business');
            navigate('/welcome');
          }}
          className="mt-4 text-sm text-surface-400 hover:text-surface-600 dark:hover:text-surface-300 transition-colors cursor-pointer"
        >
          {t('useCase.skip')}
        </motion.button>
      </div>
    </div>
  );
}
