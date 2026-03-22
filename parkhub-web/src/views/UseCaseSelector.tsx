import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  Buildings, House, UsersThree, Car, ArrowRight, ArrowLeft, Check,
  SunDim, Moon, ToggleLeft, ToggleRight, Info, ShieldCheck,
} from '@phosphor-icons/react';
import { useUseCase, type UseCase } from '../context/UseCaseContext';
import { useTheme } from '../context/ThemeContext';
import { useBgClass } from '../components/GenerativeBg';
import {
  useFeatures,
  FEATURE_REGISTRY,
  USE_CASE_PRESETS,
  type FeatureModule,
} from '../context/FeaturesContext';

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

const CATEGORY_ORDER = ['core', 'collaboration', 'billing', 'admin', 'experience'] as const;

export function UseCaseSelectorPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { setUseCase } = useUseCase();
  const { resolved, setTheme } = useTheme();
  const { setFeatures, applyPreset } = useFeatures();
  const [selected, setSelected] = useState<UseCase | null>(null);
  const [step, setStep] = useState<'usecase' | 'features'>('usecase');
  const [localFeatures, setLocalFeatures] = useState<FeatureModule[]>([]);
  const [confirmed, setConfirmed] = useState(false);
  const [expandedHelp, setExpandedHelp] = useState<string | null>(null);

  function handleUseCaseContinue() {
    if (!selected) return;
    const preset = USE_CASE_PRESETS[selected];
    setLocalFeatures([...preset]);
    setStep('features');
  }

  function handleFeatureToggle(id: FeatureModule) {
    setLocalFeatures(prev =>
      prev.includes(id) ? prev.filter(f => f !== id) : [...prev, id]
    );
  }

  function handleFinish() {
    if (!selected) return;
    setConfirmed(true);
    setUseCase(selected);
    setFeatures(localFeatures);
    setTimeout(() => navigate('/welcome'), 350);
  }

  function handleBack() {
    setStep('usecase');
    setExpandedHelp(null);
  }

  const grouped = CATEGORY_ORDER.map(cat => ({
    category: cat,
    modules: FEATURE_REGISTRY.filter(f => f.category === cat),
  })).filter(g => g.modules.length > 0);
  const bgClass = useBgClass('hatch');

  return (
    <main className={`min-h-dvh ${bgClass || 'parking-grid'} relative overflow-hidden`}>
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
          aria-label={resolved === 'dark' ? t('nav.switchToLight') : t('nav.switchToDark')}
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
          transition={{ duration: 0.4, ease: [0.22, 1, 0.36, 1] as const }}
          className="mb-7"
        >
          <div className="w-12 h-12 bg-primary-900 dark:bg-surface-800 flex items-center justify-center border border-primary-800 dark:border-surface-700">
            <Car weight="fill" className="w-6 h-6 text-accent-500" />
          </div>
        </motion.div>

        {/* Step indicator */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          className="flex items-center gap-2 mb-6"
        >
          <div className={`w-2 h-2 rounded-full transition-colors ${step === 'usecase' ? 'bg-accent-500' : 'bg-surface-300 dark:bg-surface-600'}`} />
          <div className={`w-8 h-px ${step === 'features' ? 'bg-accent-500' : 'bg-surface-300 dark:bg-surface-600'}`} />
          <div className={`w-2 h-2 rounded-full transition-colors ${step === 'features' ? 'bg-accent-500' : 'bg-surface-300 dark:bg-surface-600'}`} />
        </motion.div>

        <AnimatePresence mode="wait">
          {step === 'usecase' ? (
            <motion.div
              key="usecase"
              initial={{ opacity: 0, x: -20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -20 }}
              className="flex flex-col items-center w-full"
            >
              {/* Heading */}
              <div className="text-center mb-8 max-w-lg">
                <h1 className="text-2xl sm:text-3xl font-bold tracking-tight text-primary-900 dark:text-surface-100 mb-2">
                  {t('useCase.title')}
                </h1>
                <p className="text-surface-500 dark:text-surface-400 text-base">
                  {t('useCase.subtitle')}
                </p>
              </div>

              {/* Use-case cards */}
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-3 max-w-2xl w-full mb-8">
                {USE_CASES.map((uc, i) => {
                  const isSelected = selected === uc.id;
                  return (
                    <motion.button
                      key={uc.id}
                      initial={{ opacity: 0, y: 12 }}
                      animate={{ opacity: 1, y: 0 }}
                      transition={{ delay: 0.15 + i * 0.08 }}
                      whileHover={{ y: -3 }}
                      whileTap={{ scale: 0.98 }}
                      onClick={() => setSelected(uc.id)}
                      className={`relative card p-5 text-left transition-all cursor-pointer group ${
                        isSelected
                          ? `ring-2 ${uc.ring} ${uc.border} shadow-md`
                          : 'hover:shadow-sm'
                      }`}
                    >
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
              </div>

              {/* Continue */}
              <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} transition={{ delay: 0.4 }}>
                <button
                  onClick={handleUseCaseContinue}
                  disabled={!selected}
                  className="btn btn-primary text-sm px-7 py-3 disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
                >
                  {t('useCase.continue')}
                  <ArrowRight weight="bold" className="w-4 h-4" />
                </button>
              </motion.div>

              {/* Skip */}
              <button
                onClick={() => {
                  setUseCase('business');
                  applyPreset('business');
                  navigate('/welcome');
                }}
                className="mt-4 text-xs text-surface-400 hover:text-surface-600 dark:hover:text-surface-300 transition-colors cursor-pointer uppercase tracking-wider"
              >
                {t('useCase.skip')}
              </button>
            </motion.div>
          ) : (
            <motion.div
              key="features"
              initial={{ opacity: 0, x: 20 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: 20 }}
              className="flex flex-col items-center w-full max-w-2xl"
            >
              {/* Heading */}
              <div className="text-center mb-6">
                <h1 className="text-2xl sm:text-3xl font-bold tracking-tight text-primary-900 dark:text-surface-100 mb-2">
                  {t('features.onboardingTitle')}
                </h1>
                <p className="text-surface-500 dark:text-surface-400 text-sm max-w-md mx-auto">
                  {t('features.onboardingSubtitle')}
                </p>
              </div>

              {/* Feature toggles by category */}
              <div className="w-full space-y-4 mb-6">
                {grouped.map(({ category, modules }, gi) => (
                  <motion.div
                    key={category}
                    initial={{ opacity: 0, y: 12 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: gi * 0.08 }}
                    className="card overflow-hidden"
                  >
                    <div className="px-4 py-2.5 bg-surface-50 dark:bg-surface-800/50 border-b border-surface-200/50 dark:border-surface-700/50">
                      <span className="text-[10px] font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-widest">
                        {t(`features.categories.${category}`)}
                      </span>
                    </div>
                    <div className="divide-y divide-surface-100 dark:divide-surface-800/50">
                      {modules.map(mod => {
                        const enabled = localFeatures.includes(mod.id);
                        const helpOpen = expandedHelp === mod.id;
                        return (
                          <div key={mod.id} className="px-4 py-3">
                            <div className="flex items-center gap-3">
                              <button
                                onClick={() => handleFeatureToggle(mod.id)}
                                className="flex-shrink-0 cursor-pointer"
                                aria-label={t(`features.modules.${mod.id}.name`)}
                              >
                                {enabled ? (
                                  <ToggleRight weight="fill" className="w-7 h-7 text-accent-500" />
                                ) : (
                                  <ToggleLeft weight="regular" className="w-7 h-7 text-surface-300 dark:text-surface-600" />
                                )}
                              </button>
                              <div className="flex-1 min-w-0">
                                <div className="flex items-center gap-2">
                                  <span className={`text-sm font-medium transition-colors ${enabled ? 'text-surface-900 dark:text-white' : 'text-surface-400 dark:text-surface-500'}`}>
                                    {t(`features.modules.${mod.id}.name`)}
                                  </span>
                                  <span className={`text-[9px] font-semibold uppercase tracking-wider px-1.5 py-0.5 transition-colors ${
                                    enabled
                                      ? 'bg-accent-100 dark:bg-accent-900/20 text-accent-700 dark:text-accent-400'
                                      : 'bg-surface-100 dark:bg-surface-800 text-surface-400 dark:text-surface-500'
                                  }`}>
                                    {enabled ? t('features.enabled') : t('features.disabled')}
                                  </span>
                                </div>
                                <p className={`text-xs mt-0.5 leading-relaxed transition-colors ${enabled ? 'text-surface-500 dark:text-surface-400' : 'text-surface-400 dark:text-surface-600'}`}>
                                  {t(`features.modules.${mod.id}.desc`)}
                                </p>
                              </div>
                              <button
                                onClick={() => setExpandedHelp(helpOpen ? null : mod.id)}
                                className={`flex-shrink-0 w-7 h-7 flex items-center justify-center rounded-md transition-colors cursor-pointer ${
                                  helpOpen ? 'bg-accent-100 dark:bg-accent-900/20 text-accent-600' : 'text-surface-400 hover:text-surface-600 dark:hover:text-surface-300'
                                }`}
                                aria-label={t('common.info', 'More info')}
                              >
                                <Info weight={helpOpen ? 'fill' : 'regular'} className="w-4 h-4" />
                              </button>
                            </div>
                            <AnimatePresence>
                              {helpOpen && (
                                <motion.div
                                  initial={{ height: 0, opacity: 0 }}
                                  animate={{ height: 'auto', opacity: 1 }}
                                  exit={{ height: 0, opacity: 0 }}
                                  className="overflow-hidden"
                                >
                                  <div className="mt-2 ml-10 p-3 bg-surface-50 dark:bg-surface-800/40 rounded-md border border-surface-200/50 dark:border-surface-700/50">
                                    <p className="text-xs text-surface-600 dark:text-surface-400 leading-relaxed">
                                      {t(`features.modules.${mod.id}.help`)}
                                    </p>
                                  </div>
                                </motion.div>
                              )}
                            </AnimatePresence>
                          </div>
                        );
                      })}
                    </div>
                  </motion.div>
                ))}
              </div>

              {/* Compliance info */}
              <motion.div
                initial={{ opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ delay: 0.2 }}
                className="w-full card p-4 mb-6 border-l-3 border-l-emerald-500"
              >
                <div className="flex items-start gap-3">
                  <ShieldCheck weight="fill" className="w-5 h-5 text-emerald-500 flex-shrink-0 mt-0.5" />
                  <div>
                    <p className="text-xs font-semibold text-surface-900 dark:text-white uppercase tracking-wider mb-1.5">
                      {t('features.compliance.title')}
                    </p>
                    <ul className="space-y-1">
                      <li className="text-[11px] text-surface-500 dark:text-surface-400 leading-relaxed">
                        {t('features.compliance.gdpr')}
                      </li>
                      <li className="text-[11px] text-surface-500 dark:text-surface-400 leading-relaxed">
                        {t('features.compliance.audit')}
                      </li>
                      <li className="text-[11px] text-surface-500 dark:text-surface-400 leading-relaxed">
                        {t('features.compliance.encryption')}
                      </li>
                    </ul>
                  </div>
                </div>
              </motion.div>

              {/* Actions */}
              <div className="flex items-center gap-3">
                <button
                  onClick={handleBack}
                  className="btn btn-secondary text-sm px-5 py-3 cursor-pointer"
                >
                  <ArrowLeft weight="bold" className="w-4 h-4" />
                  {t('onboarding.back')}
                </button>

                <AnimatePresence mode="wait">
                  {confirmed ? (
                    <motion.div
                      key="confirmed"
                      initial={{ scale: 0.9, opacity: 0 }}
                      animate={{ scale: 1, opacity: 1 }}
                      className="flex items-center gap-2 text-accent-600 dark:text-accent-400 font-semibold text-sm px-5 py-3"
                    >
                      <Check weight="bold" className="w-4 h-4" />
                      {t('useCase.applying')}
                    </motion.div>
                  ) : (
                    <motion.button
                      key="finish"
                      whileHover={{ scale: 1.02 }}
                      whileTap={{ scale: 0.98 }}
                      onClick={handleFinish}
                      className="btn btn-primary text-sm px-7 py-3 cursor-pointer"
                    >
                      {t('onboarding.finish')}
                      <ArrowRight weight="bold" className="w-4 h-4" />
                    </motion.button>
                  )}
                </AnimatePresence>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </main>
  );
}
