import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import en from './locales/en';

// Only the fallback (en) ships in the main bundle. The other nine locale
// files are registered with Vite as lazy imports so they become their own
// chunks — a German user now downloads en (1.7 kLOC) + de (1.2 kLOC)
// instead of all 10 locales (~7 kLOC / ~500 KB raw).
//
// import.meta.glob produces a static map {path -> () => dynamic import()}.
// The `!./en.ts` exclude keeps en from double-bundling as a lazy chunk.
const lazyLocales = import.meta.glob<{ default: unknown }>(
  ['./locales/*.ts', '!./locales/en.ts'],
);

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
    resources: { en },
    fallbackLng: 'en',
    partialBundledLanguages: true,
    interpolation: { escapeValue: false },
    detection: {
      order: ['localStorage', 'navigator'],
      caches: ['localStorage'],
    },
  });

// Fetch the detected locale (and the first Accept-Language fallback) in the
// background. Until it arrives, i18n serves en as the fallback — users see a
// brief English→local flicker on the very first visit, but the saved KB of
// JS parse time more than pays for itself on slow mobile connections, and
// subsequent visits hit the locale chunk straight from the HTTP cache.
const primary = i18n.language?.split('-')[0];
const secondary = (() => {
  if (typeof navigator === 'undefined') return undefined;
  const alt = navigator.languages?.find((l) => l.split('-')[0] !== primary);
  return alt?.split('-')[0];
})();

async function hydrate(code: string | undefined): Promise<void> {
  if (!code || code === 'en') return;
  const loader = lazyLocales[`./locales/${code}.ts`];
  if (!loader) return;
  try {
    const mod = await loader();
    const bundle = (mod.default as { translation?: Record<string, unknown> })?.translation;
    if (bundle) {
      i18n.addResourceBundle(code, 'translation', bundle, true, true);
      if (i18n.language?.split('-')[0] === code) {
        await i18n.changeLanguage(code);
      }
    }
  } catch {
    // Network hiccup — user stays on fallback, no crash
  }
}

void hydrate(primary);
if (secondary) void hydrate(secondary);

/** Fetch approved translation overrides from the API and patch into i18n bundles. */
export async function loadTranslationOverrides(): Promise<void> {
  try {
    const base = (import.meta as Record<string, any>).env?.VITE_API_URL || '';
    const { getInMemoryToken } = await import('../api/client');
    const token = getInMemoryToken();
    const headers: Record<string, string> = {
      Accept: 'application/json',
      'X-Requested-With': 'XMLHttpRequest',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    };
    const res = await fetch(`${base}/api/v1/translations/overrides`, { headers, credentials: 'include' });
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
