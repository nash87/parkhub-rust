import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import en from './locales/en';
import de from './locales/de';
import fr from './locales/fr';
import es from './locales/es';
import it from './locales/it';
import pt from './locales/pt';
import tr from './locales/tr';
import pl from './locales/pl';
import ja from './locales/ja';
import zh from './locales/zh';

export const languages = [
  { code: 'en', name: 'English', native: 'English' },
  { code: 'de', name: 'German', native: 'Deutsch' },
  { code: 'fr', name: 'French', native: 'Fran\u00e7ais' },
  { code: 'es', name: 'Spanish', native: 'Espa\u00f1ol' },
  { code: 'it', name: 'Italian', native: 'Italiano' },
  { code: 'pt', name: 'Portuguese', native: 'Portugu\u00eas' },
  { code: 'tr', name: 'Turkish', native: 'T\u00fcrk\u00e7e' },
  { code: 'pl', name: 'Polish', native: 'Polski' },
  { code: 'ja', name: 'Japanese', native: '\u65e5\u672c\u8a9e' },
  { code: 'zh', name: 'Chinese', native: '\u4e2d\u6587' },
] as const;

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: { en, de, fr, es, it, pt, tr, pl, ja, zh },
    fallbackLng: 'en',
    interpolation: { escapeValue: false },
    detection: {
      order: ['localStorage', 'navigator'],
      caches: ['localStorage'],
    },
  });

export default i18n;
