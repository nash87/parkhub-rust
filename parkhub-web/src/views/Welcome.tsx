import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { languages } from '../i18n';
import {
  Car, CalendarCheck, ChartLineUp, CoinVertical,
  ArrowRight, Globe, SunDim, Moon,
} from '@phosphor-icons/react';
import { useTheme } from '../context/ThemeContext';
import { useUseCase } from '../context/UseCaseContext';

const CYCLE_GREETINGS = [
  { lang: 'en', text: 'Welcome' },
  { lang: 'de', text: 'Willkommen' },
  { lang: 'fr', text: 'Bienvenue' },
  { lang: 'es', text: 'Bienvenido' },
  { lang: 'it', text: 'Benvenuto' },
  { lang: 'pt', text: 'Bem-vindo' },
  { lang: 'tr', text: 'Ho\u015fgeldiniz' },
  { lang: 'pl', text: 'Witamy' },
  { lang: 'ja', text: '\u3088\u3046\u3053\u305d' },
  { lang: 'zh', text: '\u6b22\u8fce' },
];

export function WelcomePage() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { resolved, setTheme } = useTheme();
  const { useCase } = useUseCase();
  const [greetingIdx, setGreetingIdx] = useState(0);
  const [showLanguages, setShowLanguages] = useState(false);
  const [selectedLang, setSelectedLang] = useState(i18n.language?.slice(0, 2) || 'en');

  useEffect(() => {
    const interval = setInterval(() => {
      setGreetingIdx(i => (i + 1) % CYCLE_GREETINGS.length);
    }, 2500);
    return () => clearInterval(interval);
  }, []);

  function selectLanguage(code: string) {
    setSelectedLang(code);
    i18n.changeLanguage(code);
  }

  const greeting = CYCLE_GREETINGS[greetingIdx];

  return (
    <div className="min-h-dvh parking-grid relative overflow-hidden">
      {/* Architectural accent lines */}
      <div className="absolute inset-0 overflow-hidden pointer-events-none">
        <div className="absolute top-0 left-1/2 -translate-x-1/2 w-px h-40 bg-gradient-to-b from-accent-500/30 to-transparent" />
        <div className="absolute top-[12%] right-[8%] w-px h-20 bg-accent-500/15" />
        <div className="absolute bottom-[15%] left-[6%] w-20 h-px bg-accent-500/10" />
        <div className="absolute top-[60%] right-[5%] w-16 h-px bg-surface-300/30 dark:bg-surface-700/30" />
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
        {/* Logo mark */}
        <motion.div
          initial={{ opacity: 0, y: -16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.4, ease: [0.22, 1, 0.36, 1] }}
          className="mb-8"
        >
          <div className="w-14 h-14 bg-primary-900 dark:bg-surface-800 flex items-center justify-center border border-primary-800 dark:border-surface-700">
            <Car weight="fill" className="w-7 h-7 text-accent-500" />
          </div>
        </motion.div>

        {/* Cycling greeting */}
        <div className="h-14 mb-2 flex items-center justify-center">
          <AnimatePresence mode="wait">
            <motion.div
              key={greeting.lang}
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -12 }}
              transition={{ duration: 0.3 }}
              className="text-center"
            >
              <span className="text-4xl sm:text-5xl font-extrabold tracking-tighter text-primary-900 dark:text-surface-100">
                {greeting.text}
              </span>
            </motion.div>
          </AnimatePresence>
        </div>

        <motion.p
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.2 }}
          className="text-surface-500 dark:text-surface-400 text-base text-center max-w-md mb-10"
        >
          {t(`welcome.subtitle.${useCase}`)}
        </motion.p>

        {/* Feature cards */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.35 }}
          className="grid grid-cols-1 sm:grid-cols-3 gap-3 max-w-xl w-full mb-10"
        >
          {[
            { icon: CalendarCheck, title: t(`welcome.features.${useCase}.booking`), desc: t(`welcome.features.${useCase}.bookingDesc`), accent: 'bg-accent-100 dark:bg-accent-900/20 text-accent-600 dark:text-accent-400' },
            { icon: CoinVertical, title: t(`welcome.features.${useCase}.credits`), desc: t(`welcome.features.${useCase}.creditsDesc`), accent: 'bg-primary-100 dark:bg-primary-900/20 text-primary-600 dark:text-primary-400' },
            { icon: ChartLineUp, title: t(`welcome.features.${useCase}.analytics`), desc: t(`welcome.features.${useCase}.analyticsDesc`), accent: 'bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400' },
          ].map((feat, i) => (
            <motion.div
              key={i}
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.45 + i * 0.08 }}
              className="card p-4 text-center hover:border-accent-300 dark:hover:border-accent-700 transition-colors"
            >
              <div className={`w-10 h-10 mx-auto mb-3 flex items-center justify-center ${feat.accent}`}>
                <feat.icon weight="bold" className="w-5 h-5" />
              </div>
              <h3 className="font-semibold text-surface-800 dark:text-surface-100 text-sm mb-1">{feat.title}</h3>
              <p className="text-xs text-surface-500 dark:text-surface-400 leading-relaxed">{feat.desc}</p>
            </motion.div>
          ))}
        </motion.div>

        {/* Language selector */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.6 }}
          className="mb-7"
        >
          <button
            onClick={() => setShowLanguages(!showLanguages)}
            className="btn btn-secondary gap-2"
          >
            <Globe weight="bold" className="w-3.5 h-3.5" />
            {languages.find(l => l.code === selectedLang)?.native || 'English'}
          </button>

          <AnimatePresence>
            {showLanguages && (
              <motion.div
                initial={{ opacity: 0, scale: 0.97, y: -6 }}
                animate={{ opacity: 1, scale: 1, y: 0 }}
                exit={{ opacity: 0, scale: 0.97, y: -6 }}
                className="card mt-2 p-2.5 grid grid-cols-2 sm:grid-cols-5 gap-1.5"
              >
                {languages.map(lang => (
                  <button
                    key={lang.code}
                    onClick={() => { selectLanguage(lang.code); setShowLanguages(false); }}
                    className={`flex items-center gap-2 px-2.5 py-1.5 rounded-md text-xs font-medium transition-all cursor-pointer ${
                      selectedLang === lang.code
                        ? 'bg-accent-100 dark:bg-accent-900/20 text-accent-700 dark:text-accent-300 ring-1 ring-accent-500'
                        : 'hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-600 dark:text-surface-400'
                    }`}
                  >
                    <span className="text-[10px] font-mono uppercase tracking-wider text-surface-400">{lang.code}</span>
                    <span>{lang.native}</span>
                  </button>
                ))}
              </motion.div>
            )}
          </AnimatePresence>
        </motion.div>

        {/* CTA */}
        <motion.button
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.7 }}
          whileHover={{ scale: 1.02 }}
          whileTap={{ scale: 0.98 }}
          onClick={() => navigate('/login')}
          className="btn btn-primary text-sm px-7 py-3 cursor-pointer"
        >
          {t('welcome.getStarted')}
          <ArrowRight weight="bold" className="w-4 h-4" />
        </motion.button>
      </div>
    </div>
  );
}
