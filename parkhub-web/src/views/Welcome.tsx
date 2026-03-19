import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { languages } from '../i18n';
import {
  CarSimple, CalendarCheck, ChartLineUp, Sparkle,
  ArrowRight, Globe, SunDim, Moon,
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
    <div className="min-h-dvh mesh-gradient relative overflow-hidden">
      {/* Floating decorative elements */}
      <div className="absolute inset-0 overflow-hidden pointer-events-none">
        <motion.div
          animate={{ y: [0, -20, 0], rotate: [0, 5, 0] }}
          transition={{ duration: 6, repeat: Infinity, ease: 'easeInOut' }}
          className="absolute top-[15%] right-[10%] w-20 h-20 rounded-3xl bg-primary-500/10 backdrop-blur-sm"
        />
        <motion.div
          animate={{ y: [0, 15, 0], rotate: [0, -3, 0] }}
          transition={{ duration: 8, repeat: Infinity, ease: 'easeInOut', delay: 1 }}
          className="absolute bottom-[20%] left-[5%] w-32 h-32 rounded-full bg-accent-400/10 backdrop-blur-sm"
        />
        <motion.div
          animate={{ y: [0, -10, 0] }}
          transition={{ duration: 5, repeat: Infinity, ease: 'easeInOut', delay: 2 }}
          className="absolute top-[40%] left-[15%] w-16 h-16 rounded-2xl bg-info/10 backdrop-blur-sm"
        />
      </div>

      {/* Theme toggle */}
      <div className="absolute top-6 right-6 z-10">
        <button
          onClick={() => setTheme(resolved === 'dark' ? 'light' : 'dark')}
          className="btn btn-ghost btn-icon glass-card w-12 h-12"
          aria-label="Toggle theme"
        >
          {resolved === 'dark' ? <SunDim weight="fill" className="w-5 h-5 text-accent-400" /> : <Moon weight="fill" className="w-5 h-5 text-surface-600" />}
        </button>
      </div>

      <div className="relative z-10 flex flex-col items-center justify-center min-h-dvh px-6 py-12">
        {/* Logo */}
        <motion.div
          initial={{ opacity: 0, scale: 0.8 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.6, ease: [0.22, 1, 0.36, 1] }}
          className="mb-8"
        >
          <div className="w-20 h-20 rounded-3xl bg-gradient-to-br from-primary-500 to-primary-600 flex items-center justify-center shadow-xl shadow-primary-500/25">
            <CarSimple weight="fill" className="w-10 h-10 text-white" />
          </div>
        </motion.div>

        {/* Cycling greeting */}
        <div className="h-16 mb-2 flex items-center justify-center">
          <AnimatePresence mode="wait">
            <motion.div
              key={greeting.lang}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -20 }}
              transition={{ duration: 0.4 }}
              className="text-center"
            >
              <span className="text-4xl sm:text-5xl font-extrabold tracking-tight bg-gradient-to-r from-primary-600 via-primary-500 to-primary-400 bg-clip-text text-transparent">
                {greeting.text}
              </span>
              <span className="ml-3 text-3xl">{greeting.flag}</span>
            </motion.div>
          </AnimatePresence>
        </div>

        <motion.p
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.3 }}
          className="text-surface-500 dark:text-surface-400 text-lg text-center max-w-md mb-10"
        >
          {t('welcome.subtitle')}
        </motion.p>

        {/* Feature cards */}
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.5 }}
          className="grid grid-cols-1 sm:grid-cols-3 gap-4 max-w-2xl w-full mb-10"
        >
          {[
            { icon: CalendarCheck, title: t('welcome.features.booking'), desc: t('welcome.features.bookingDesc'), color: 'primary' },
            { icon: Sparkle, title: t('welcome.features.credits'), desc: t('welcome.features.creditsDesc'), color: 'accent' },
            { icon: ChartLineUp, title: t('welcome.features.analytics'), desc: t('welcome.features.analyticsDesc'), color: 'info' },
          ].map((feat, i) => (
            <motion.div
              key={i}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.6 + i * 0.1 }}
              className="glass-card p-5 text-center group hover:shadow-xl transition-shadow"
            >
              <div className={`w-12 h-12 rounded-2xl mx-auto mb-3 flex items-center justify-center ${
                feat.color === 'primary' ? 'bg-primary-100 dark:bg-primary-900/30' :
                feat.color === 'accent' ? 'bg-accent-100 dark:bg-accent-900/30' :
                'bg-blue-100 dark:bg-blue-900/30'
              }`}>
                <feat.icon weight="fill" className={`w-6 h-6 ${
                  feat.color === 'primary' ? 'text-primary-600 dark:text-primary-400' :
                  feat.color === 'accent' ? 'text-accent-600 dark:text-accent-400' :
                  'text-blue-600 dark:text-blue-400'
                }`} />
              </div>
              <h3 className="font-semibold text-surface-900 dark:text-white mb-1">{feat.title}</h3>
              <p className="text-sm text-surface-500 dark:text-surface-400">{feat.desc}</p>
            </motion.div>
          ))}
        </motion.div>

        {/* Language selector */}
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ delay: 0.8 }}
          className="mb-8"
        >
          <button
            onClick={() => setShowLanguages(!showLanguages)}
            className="btn btn-secondary gap-2"
          >
            <Globe weight="bold" className="w-4 h-4" />
            {languages.find(l => l.code === selectedLang)?.flag} {languages.find(l => l.code === selectedLang)?.native}
          </button>

          <AnimatePresence>
            {showLanguages && (
              <motion.div
                initial={{ opacity: 0, scale: 0.95, y: -10 }}
                animate={{ opacity: 1, scale: 1, y: 0 }}
                exit={{ opacity: 0, scale: 0.95, y: -10 }}
                className="glass-card mt-3 p-3 grid grid-cols-2 sm:grid-cols-5 gap-2"
              >
                {languages.map(lang => (
                  <button
                    key={lang.code}
                    onClick={() => { selectLanguage(lang.code); setShowLanguages(false); }}
                    className={`flex items-center gap-2 px-3 py-2 rounded-xl text-sm font-medium transition-all ${
                      selectedLang === lang.code
                        ? 'bg-primary-100 dark:bg-primary-900/40 text-primary-700 dark:text-primary-300 ring-2 ring-primary-500'
                        : 'hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-600 dark:text-surface-400'
                    }`}
                  >
                    <span className="text-lg">{lang.flag}</span>
                    <span>{lang.native}</span>
                  </button>
                ))}
              </motion.div>
            )}
          </AnimatePresence>
        </motion.div>

        {/* CTA */}
        <motion.button
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 1 }}
          whileHover={{ scale: 1.02 }}
          whileTap={{ scale: 0.98 }}
          onClick={handleGetStarted}
          className="btn btn-primary text-base px-8 py-3.5 shadow-xl shadow-primary-500/25"
        >
          {t('welcome.getStarted')}
          <ArrowRight weight="bold" className="w-5 h-5" />
        </motion.button>
      </div>
    </div>
  );
}
