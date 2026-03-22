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
    <div className="min-h-dvh mesh-gradient-animated relative overflow-hidden">
      {/* Decorative gradient orbs */}
      <div className="absolute top-[-20%] right-[-10%] w-[60%] h-[60%] rounded-full bg-gradient-to-br from-primary-400/20 via-primary-300/10 to-transparent dark:from-primary-600/10 dark:via-primary-500/5 blur-3xl pointer-events-none" />
      <div className="absolute bottom-[-15%] left-[-10%] w-[50%] h-[50%] rounded-full bg-gradient-to-tr from-accent-400/15 via-accent-300/8 to-transparent dark:from-accent-600/8 dark:via-accent-500/3 blur-3xl pointer-events-none" />

      {/* Top bar */}
      <header className="relative z-10 flex items-center justify-between px-6 sm:px-10 py-5">
        <motion.div
          initial={{ opacity: 0, x: -12 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ duration: 0.4 }}
          className="flex items-center gap-3"
        >
          <div className="w-9 h-9 rounded-lg bg-primary-600 flex items-center justify-center shadow-lg shadow-primary-500/20">
            <CarSimple weight="fill" className="w-5 h-5 text-white" />
          </div>
          <span className="text-lg font-bold text-surface-900 dark:text-white tracking-tight">ParkHub</span>
        </motion.div>
        <button
          onClick={() => setTheme(resolved === 'dark' ? 'light' : 'dark')}
          className="p-2 rounded-lg hover:bg-surface-100/80 dark:hover:bg-surface-800/80 transition-colors backdrop-blur-sm"
          aria-label={resolved === 'dark' ? t('nav.switchToLight') : t('nav.switchToDark')}
        >
          {resolved === 'dark' ? <SunDim weight="bold" className="w-5 h-5 text-surface-400" /> : <Moon weight="bold" className="w-5 h-5 text-surface-500" />}
        </button>
      </header>

      {/* Main content */}
      <main className="relative z-10 flex flex-col justify-center min-h-[calc(100dvh-80px)] px-6 sm:px-10 lg:px-20 max-w-3xl">
        {/* Cycling greeting — spring animation */}
        <div className="h-20 sm:h-24 mb-4 flex items-center" aria-live="polite" aria-atomic="true">
          <AnimatePresence mode="wait">
            <motion.div
              key={greeting.lang}
              initial={{ opacity: 0, y: 20, filter: 'blur(4px)' }}
              animate={{ opacity: 1, y: 0, filter: 'blur(0px)' }}
              exit={{ opacity: 0, y: -20, filter: 'blur(4px)' }}
              transition={{ type: 'spring', stiffness: 300, damping: 25 }}
            >
              <span className="text-5xl sm:text-6xl lg:text-7xl font-extrabold text-surface-900 dark:text-white" style={{ letterSpacing: '-0.03em' }}>
                {greeting.text}
              </span>
              <span className="ml-4 text-4xl sm:text-5xl">{greeting.flag}</span>
            </motion.div>
          </AnimatePresence>
        </div>

        {/* Subtitle */}
        <motion.p
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.2 }}
          className="text-lg sm:text-xl text-surface-500 dark:text-surface-400 max-w-lg mb-10 leading-relaxed"
        >
          {t('welcome.subtitle')}
        </motion.p>

        {/* Feature pills */}
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

        {/* Actions */}
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.6 }}
          className="flex flex-wrap items-center gap-4 mb-10"
        >
          <motion.button
            onClick={handleGetStarted}
            whileHover={{ scale: 1.03 }}
            whileTap={{ scale: 0.97 }}
            className="btn btn-primary text-base px-7 py-3 shadow-lg shadow-primary-500/20 dark:shadow-primary-500/10"
          >
            {t('welcome.getStarted')}
            <ArrowRight weight="bold" className="w-5 h-5" />
          </motion.button>

          <button
            onClick={() => setShowLanguages(!showLanguages)}
            className="btn btn-secondary gap-2 backdrop-blur-sm"
          >
            <Globe weight="bold" className="w-4 h-4" />
            {languages.find(l => l.code === selectedLang)?.flag} {languages.find(l => l.code === selectedLang)?.native}
          </button>
        </motion.div>

        {/* Language picker */}
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
                  <motion.button
                    key={lang.code}
                    whileHover={{ scale: 1.05 }}
                    whileTap={{ scale: 0.95 }}
                    onClick={() => { selectLanguage(lang.code); setShowLanguages(false); }}
                    className={`flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium transition-colors backdrop-blur-sm ${
                      selectedLang === lang.code
                        ? 'bg-primary-600 text-white shadow-md shadow-primary-500/20'
                        : 'bg-surface-100/80 dark:bg-surface-800/80 text-surface-600 dark:text-surface-400 hover:bg-surface-200/80 dark:hover:bg-surface-700/80'
                    }`}
                  >
                    <span>{lang.flag}</span>
                    <span>{lang.native}</span>
                  </motion.button>
                ))}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </main>
    </div>
  );
}
