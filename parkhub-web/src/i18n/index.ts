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
  { code: 'en', name: 'English', flag: '🇬🇧', native: 'English' },
  { code: 'de', name: 'German', flag: '🇩🇪', native: 'Deutsch' },
  { code: 'fr', name: 'French', flag: '🇫🇷', native: 'Francais' },
  { code: 'es', name: 'Spanish', flag: '🇪🇸', native: 'Espanol' },
  { code: 'it', name: 'Italian', flag: '🇮🇹', native: 'Italiano' },
  { code: 'pt', name: 'Portuguese', flag: '🇵🇹', native: 'Portugues' },
  { code: 'tr', name: 'Turkish', flag: '🇹🇷', native: 'Turkce' },
  { code: 'pl', name: 'Polish', flag: '🇵🇱', native: 'Polski' },
  { code: 'ja', name: 'Japanese', flag: '🇯🇵', native: '日本語' },
  { code: 'zh', name: 'Chinese', flag: '🇨🇳', native: '中文' },
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

/** Fetch approved translation overrides from the API and patch into i18n bundles. */
export async function loadTranslationOverrides(): Promise<void> {
  try {
    const base = (import.meta as Record<string, any>).env?.VITE_API_URL || '';
    const token = typeof localStorage !== 'undefined' ? localStorage.getItem('parkhub_token') : null;
    const headers: Record<string, string> = {
      Accept: 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    };
    const res = await fetch(`${base}/api/v1/translations/overrides`, { headers });
    if (!res.ok) return;
    const json = await res.json();
    const overrides: { language: string; key: string; value: string }[] =
      Array.isArray(json) ? json : json?.data ?? [];
    for (const override of overrides) {
      i18n.addResource(override.language, 'translation', override.key, override.value);
    }
  } catch {
    // Silently ignore — overrides are optional
  }
}

export default i18n;
