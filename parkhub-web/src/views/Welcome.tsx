import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { languages } from '../i18n';
import {
  ArrowRight, Globe, SunDim, Moon, CarSimple,
} from '@phosphor-icons/react';
import { useTheme } from '../context/ThemeContext';

const CYCLE_GREETINGS = [
  { lang: 'en', text: 'Welcome', flag: '🇬🇧' },
  { lang: 'de', text: 'Willkommen', flag: '🇩🇪' },
  { lang: 'fr', text: 'Bienvenue', flag: '🇫🇷' },
  { lang: 'es', text: 'Bienvenido', flag: '🇪🇸' },
  { lang: 'it', text: 'Benvenuto', flag: '🇮🇹' },
  { lang: 'pt', text: 'Bem-vindo', flag: '🇵🇹' },
  { lang: 'tr', text: 'Hosgeldiniz', flag: '🇹🇷' },
  { lang: 'pl', text: 'Witamy', flag: '🇵🇱' },
  { lang: 'ja', text: 'ようこそ', flag: '🇯🇵' },
  { lang: 'zh', text: '欢迎', flag: '🇨🇳' },
];

export function WelcomePage() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { resolved, setTheme } = useTheme();
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

  function handleGetStarted() {
    localStorage.setItem('parkhub_welcome_seen', '1');
    navigate('/login');
  }

  const greeting = CYCLE_GREETINGS[greetingIdx];

  return (
    <div className="min-h-dvh bg-white dark:bg-surface-950 relative overflow-hidden">
      {/* Subtle background — single gradient strip, not floating blobs */}
      <div className="absolute top-0 right-0 w-1/2 h-full bg-gradient-to-bl from-primary-50 via-transparent to-transparent dark:from-primary-950/30 dark:via-transparent pointer-events-none" />

      {/* Top bar */}
      <header className="relative z-10 flex items-center justify-between px-6 sm:px-10 py-5">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-lg bg-primary-600 flex items-center justify-center">
            <CarSimple weight="fill" className="w-5 h-5 text-white" />
          </div>
          <span className="text-lg font-bold text-surface-900 dark:text-white tracking-tight">ParkHub</span>
        </div>
        <button
          onClick={() => setTheme(resolved === 'dark' ? 'light' : 'dark')}
          className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors"
          aria-label={resolved === 'dark' ? t('nav.switchToLight') : t('nav.switchToDark')}
        >
          {resolved === 'dark' ? <SunDim weight="bold" className="w-5 h-5 text-surface-400" /> : <Moon weight="bold" className="w-5 h-5 text-surface-500" />}
        </button>
      </header>

      {/* Main content — left-aligned, not centered */}
      <main className="relative z-10 flex flex-col justify-center min-h-[calc(100dvh-80px)] px-6 sm:px-10 lg:px-20 max-w-3xl">
        {/* Cycling greeting — large, left-aligned */}
        <div className="h-20 sm:h-24 mb-4 flex items-center" aria-live="polite" aria-atomic="true">
          <AnimatePresence mode="wait">
            <motion.div
              key={greeting.lang}
              initial={{ opacity: 0, y: 16 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -16 }}
              transition={{ duration: 0.3 }}
            >
              <span className="text-5xl sm:text-6xl lg:text-7xl font-extrabold tracking-tight text-surface-900 dark:text-white">
                {greeting.text}
              </span>
              <span className="ml-4 text-4xl sm:text-5xl">{greeting.flag}</span>
            </motion.div>
          </AnimatePresence>
        </div>

        {/* Subtitle — specific, not generic */}
        <motion.p
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.2 }}
          className="text-lg sm:text-xl text-surface-500 dark:text-surface-400 max-w-lg mb-10 leading-relaxed"
        >
          {t('welcome.subtitle')}
        </motion.p>

        {/* Key stats — horizontal, not 3-column cards */}
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.4 }}
          className="flex flex-wrap gap-x-8 gap-y-3 mb-10 text-sm text-surface-500 dark:text-surface-400"
        >
          <span className="flex items-center gap-2">
            <span className="w-1.5 h-1.5 rounded-full bg-primary-500" />
            {t('welcome.features.booking')}
          </span>
          <span className="flex items-center gap-2">
            <span className="w-1.5 h-1.5 rounded-full bg-primary-500" />
            {t('welcome.features.credits')}
          </span>
          <span className="flex items-center gap-2">
            <span className="w-1.5 h-1.5 rounded-full bg-primary-500" />
            {t('welcome.features.analytics')}
          </span>
          <span className="flex items-center gap-2">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
            {t('welcome.selfHosted')}
          </span>
        </motion.div>

        {/* Actions — left-aligned row */}
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.6 }}
          className="flex flex-wrap items-center gap-4 mb-10"
        >
          <button
            onClick={handleGetStarted}
            className="btn btn-primary text-base px-7 py-3"
          >
            {t('welcome.getStarted')}
            <ArrowRight weight="bold" className="w-5 h-5" />
          </button>

          <button
            onClick={() => setShowLanguages(!showLanguages)}
            className="btn btn-secondary gap-2"
          >
            <Globe weight="bold" className="w-4 h-4" />
            {languages.find(l => l.code === selectedLang)?.flag} {languages.find(l => l.code === selectedLang)?.native}
          </button>
        </motion.div>

        {/* Language picker — inline dropdown, not a floating card */}
        <AnimatePresence>
          {showLanguages && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: 'auto' }}
              exit={{ opacity: 0, height: 0 }}
              className="overflow-hidden mb-8"
            >
              <div className="flex flex-wrap gap-2 pb-2">
                {languages.map(lang => (
                  <button
                    key={lang.code}
                    onClick={() => { selectLanguage(lang.code); setShowLanguages(false); }}
                    className={`flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                      selectedLang === lang.code
                        ? 'bg-primary-600 text-white'
                        : 'bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400 hover:bg-surface-200 dark:hover:bg-surface-700'
                    }`}
                  >
                    <span>{lang.flag}</span>
                    <span>{lang.native}</span>
                  </button>
                ))}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </main>
    </div>
  );
}
