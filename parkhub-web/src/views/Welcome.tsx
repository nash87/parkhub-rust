import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { languages } from '../i18n';
import {
  ArrowRightIcon, GlobeIcon, SunDimIcon, MoonIcon, CarSimpleIcon,
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

const DEFAULT_GREETING = { lang: 'en', text: 'Welcome', flag: '🇬🇧' };

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
    // Freshly-welcomed users go through the 3-step transparency tour
    // (Privacy → Features → Trust) before seeing the login screen.
    const toured = localStorage.getItem('parkhub_onboarding_v5_seen') === '1';
    navigate(toured ? '/login' : '/tour');
  }

  const greeting = CYCLE_GREETINGS[greetingIdx] ?? DEFAULT_GREETING;

  return (
    <div className="min-h-dvh bg-surface-50 dark:bg-surface-950 relative overflow-hidden welcome-page">
      <div className="absolute inset-0 parking-grid opacity-[0.22] dark:opacity-[0.12] pointer-events-none" />
      <div className="absolute inset-x-0 top-0 h-px bg-primary-500/40" />

      {/* Top bar */}
      <header className="relative z-10 flex items-center justify-between px-6 sm:px-10 py-5">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-lg bg-primary-600 flex items-center justify-center shadow-lg shadow-primary-500/20">
            <CarSimpleIcon weight="fill" className="w-5 h-5 text-white" />
          </div>
          <span className="text-lg font-bold text-surface-900 dark:text-white tracking-tight">ParkHub</span>
        </div>
        <button
          onClick={() => setTheme(resolved === 'dark' ? 'light' : 'dark')}
          className="p-2 rounded-lg hover:bg-surface-100/80 dark:hover:bg-surface-800/80 transition-colors backdrop-blur-sm"
          aria-label={resolved === 'dark' ? t('nav.switchToLight') : t('nav.switchToDark')}
        >
          {resolved === 'dark' ? <SunDimIcon weight="bold" className="w-5 h-5 text-surface-400" /> : <MoonIcon weight="bold" className="w-5 h-5 text-surface-500" />}
        </button>
      </header>

      {/* Main content */}
      <main className="relative z-10 flex flex-col justify-center min-h-[calc(100dvh-80px)] px-6 sm:px-10 lg:px-20 max-w-3xl">
        {/* Cycling greeting stays text-only so the first route avoids
            pulling framer-motion into the unauthenticated bundle. */}
        <div className="h-20 sm:h-24 mb-4 flex items-center" aria-live="polite" aria-atomic="true">
          <div key={greeting.lang} className="transition-opacity duration-300 ease-out">
            <span className="text-5xl sm:text-6xl lg:text-7xl font-extrabold text-surface-900 dark:text-white" style={{ letterSpacing: '-0.03em' }}>
              {greeting.text}
            </span>
            <span className="ml-4 text-4xl sm:text-5xl">{greeting.flag}</span>
          </div>
        </div>

        {/* Subtitle — rendered as plain <p> so it's the LCP element without
            waiting on framer-motion's opacity animation. */}
        <p className="text-lg sm:text-xl text-surface-500 dark:text-surface-400 max-w-lg mb-10 leading-relaxed">
          {t('welcome.subtitle')}
        </p>

        {/* Feature pills */}
        <div
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
        </div>

        {/* Actions */}
        <div className="flex flex-wrap items-center gap-4 mb-10">
          <button
            onClick={handleGetStarted}
            className="btn btn-primary text-base px-7 py-3 shadow-lg shadow-primary-500/20 dark:shadow-primary-500/10 transition-transform duration-200 hover:scale-[1.03] active:scale-[0.97]"
          >
            {t('welcome.getStarted')}
            <ArrowRightIcon weight="bold" className="w-5 h-5" />
          </button>

          <button
            onClick={() => setShowLanguages(!showLanguages)}
            className="btn btn-secondary gap-2 backdrop-blur-sm"
          >
            <GlobeIcon weight="bold" className="w-4 h-4" />
            {languages.find(l => l.code === selectedLang)?.flag} {languages.find(l => l.code === selectedLang)?.native}
          </button>
        </div>

        {/* Language picker */}
        {showLanguages && (
          <div className="overflow-hidden mb-8 transition-opacity duration-200 ease-out">
            <div className="flex flex-wrap gap-2 pb-2">
              {languages.map(lang => (
                <button
                  key={lang.code}
                  onClick={() => { selectLanguage(lang.code); setShowLanguages(false); }}
                  className={`flex items-center gap-2 px-3 py-2 rounded-lg text-sm font-medium transition-colors transition-transform duration-200 backdrop-blur-sm hover:scale-[1.03] active:scale-[0.97] ${
                    selectedLang === lang.code
                      ? 'bg-primary-600 text-white shadow-md shadow-primary-500/20'
                      : 'bg-surface-100/80 dark:bg-surface-800/80 text-surface-600 dark:text-surface-400 hover:bg-surface-200/80 dark:hover:bg-surface-700/80'
                  }`}
                >
                  <span>{lang.flag}</span>
                  <span>{lang.native}</span>
                </button>
              ))}
            </div>
          </div>
        )}
      </main>
    </div>
  );
}
