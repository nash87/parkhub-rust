import { useState } from 'react';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  ToggleLeft, ToggleRight, Info, ShieldCheck, ArrowLeft,
  ArrowClockwise, FloppyDisk, Check,
} from '@phosphor-icons/react';
import { Link } from 'react-router-dom';
import {
  useFeatures,
  FEATURE_REGISTRY,
  USE_CASE_PRESETS,
  type FeatureModule,
} from '../context/FeaturesContext';
import { useUseCase } from '../context/UseCaseContext';
import { AnimatePresence } from 'framer-motion';
import toast from 'react-hot-toast';

const CATEGORY_ORDER = ['core', 'collaboration', 'billing', 'admin', 'experience'] as const;

export function AdminFeaturesPage() {
  const { t } = useTranslation();
  const { features, setFeatures } = useFeatures();
  const { useCase } = useUseCase();
  const [local, setLocal] = useState<FeatureModule[]>([...features]);
  const [expandedHelp, setExpandedHelp] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  const hasChanges = JSON.stringify([...local].sort()) !== JSON.stringify([...features].sort());

  function toggle(id: FeatureModule) {
    setLocal(prev =>
      prev.includes(id) ? prev.filter(f => f !== id) : [...prev, id]
    );
    setSaved(false);
  }

  function handleSave() {
    setFeatures(local);
    setSaved(true);
    toast.success(t('features.saved'));
    setTimeout(() => setSaved(false), 2000);
  }

  function handleReset() {
    const preset = USE_CASE_PRESETS[useCase] || USE_CASE_PRESETS.business;
    setLocal([...preset]);
    setSaved(false);
  }

  function handleEnableAll() {
    setLocal(FEATURE_REGISTRY.map(f => f.id));
    setSaved(false);
  }

  function handleDisableAll() {
    setLocal([]);
    setSaved(false);
  }

  const grouped = CATEGORY_ORDER.map(cat => ({
    category: cat,
    modules: FEATURE_REGISTRY.filter(f => f.category === cat),
  })).filter(g => g.modules.length > 0);

  const container = { hidden: { opacity: 0 }, show: { opacity: 1, transition: { staggerChildren: 0.06 } } };
  const item = { hidden: { opacity: 0, y: 12 }, show: { opacity: 1, y: 0, transition: { ease: [0.22, 1, 0.36, 1] as const } } };

  return (
    <motion.div variants={container} initial="hidden" animate="show" className="space-y-6">
      {/* Header */}
      <motion.div variants={item}>
        <Link to="/" className="inline-flex items-center gap-2 text-xs text-surface-500 hover:text-accent-600 mb-4 transition-colors cursor-pointer uppercase tracking-wider font-semibold">
          <ArrowLeft weight="bold" className="w-3.5 h-3.5" /> {t('nav.dashboard')}
        </Link>
        <p className="text-xs font-semibold text-accent-600 dark:text-accent-400 uppercase tracking-widest mb-1">
          {t('nav.admin')}
        </p>
        <h1 className="text-2xl font-bold text-surface-900 dark:text-white tracking-tight">
          {t('features.title')}
        </h1>
        <p className="text-surface-500 dark:text-surface-400 mt-0.5 text-sm">
          {t('features.subtitle')}
        </p>
      </motion.div>

      {/* Actions bar */}
      <motion.div variants={item} className="flex flex-wrap items-center gap-2">
        <button onClick={handleEnableAll} className="btn btn-secondary text-xs cursor-pointer">
          {t('features.enableAll')}
        </button>
        <button onClick={handleDisableAll} className="btn btn-secondary text-xs cursor-pointer">
          {t('features.disableAll')}
        </button>
        <button onClick={handleReset} className="btn btn-secondary text-xs cursor-pointer">
          <ArrowClockwise weight="bold" className="w-3.5 h-3.5" />
          {t('features.resetToPreset')}
        </button>
        <div className="flex-1" />
        <span className="text-[11px] text-surface-400 font-mono">
          {local.length}/{FEATURE_REGISTRY.length} {t('features.enabled').toLowerCase()}
        </span>
      </motion.div>

      {/* Feature toggles by category */}
      {grouped.map(({ category, modules }, gi) => (
        <motion.div
          key={category}
          variants={item}
          className="card overflow-hidden"
        >
          <div className="px-4 py-2.5 bg-surface-50 dark:bg-surface-800/50 border-b border-surface-200/50 dark:border-surface-700/50">
            <span className="text-[10px] font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-widest">
              {t(`features.categories.${category}`)}
            </span>
          </div>
          <div className="divide-y divide-surface-100 dark:divide-surface-800/50">
            {modules.map(mod => {
              const enabled = local.includes(mod.id);
              const helpOpen = expandedHelp === mod.id;
              return (
                <div key={mod.id} className="px-4 py-3.5">
                  <div className="flex items-center gap-3">
                    <button
                      onClick={() => toggle(mod.id)}
                      className="flex-shrink-0 cursor-pointer"
                      aria-label={`Toggle ${mod.id}`}
                    >
                      {enabled ? (
                        <ToggleRight weight="fill" className="w-8 h-8 text-accent-500 transition-colors" />
                      ) : (
                        <ToggleLeft weight="regular" className="w-8 h-8 text-surface-300 dark:text-surface-600 transition-colors" />
                      )}
                    </button>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className={`text-sm font-semibold transition-colors ${enabled ? 'text-surface-900 dark:text-white' : 'text-surface-400 dark:text-surface-500'}`}>
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
                      className={`flex-shrink-0 w-8 h-8 flex items-center justify-center rounded-md transition-colors cursor-pointer ${
                        helpOpen ? 'bg-accent-100 dark:bg-accent-900/20 text-accent-600' : 'text-surface-400 hover:text-surface-600 dark:hover:text-surface-300'
                      }`}
                      aria-label="More info"
                    >
                      <Info weight={helpOpen ? 'fill' : 'regular'} className="w-4.5 h-4.5" />
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
                        <div className="mt-2.5 ml-11 p-3 bg-surface-50 dark:bg-surface-800/40 rounded-md border border-surface-200/50 dark:border-surface-700/50">
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

      {/* Compliance */}
      <motion.div variants={item} className="card p-4 border-l-3 border-l-emerald-500">
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

      {/* Save bar */}
      <AnimatePresence>
        {hasChanges && (
          <motion.div
            initial={{ y: 60, opacity: 0 }}
            animate={{ y: 0, opacity: 1 }}
            exit={{ y: 60, opacity: 0 }}
            className="fixed bottom-6 left-1/2 -translate-x-1/2 z-50"
          >
            <button
              onClick={handleSave}
              className="btn btn-primary shadow-lg px-6 py-3 text-sm cursor-pointer"
            >
              {saved ? (
                <><Check weight="bold" className="w-4 h-4" /> {t('features.saved')}</>
              ) : (
                <><FloppyDisk weight="bold" className="w-4 h-4" /> {t('features.saveChanges')}</>
              )}
            </button>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}
