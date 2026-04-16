// Quick coverage check: count nested leaf keys per locale file vs en.ts.
// Usage: npx tsx scripts/locale-coverage.mjs
import en from '../src/i18n/locales/en.ts';

const codes = ['de', 'fr', 'es', 'it', 'pt', 'tr', 'pl', 'ja', 'zh'];

function collect(obj, prefix = '') {
  const keys = [];
  for (const [k, v] of Object.entries(obj)) {
    const p = prefix ? `${prefix}.${k}` : k;
    if (v && typeof v === 'object' && !Array.isArray(v)) {
      keys.push(...collect(v, p));
    } else {
      keys.push(p);
    }
  }
  return keys;
}

const enKeys = new Set(collect(en.translation));
console.log(`en (reference): ${enKeys.size} keys`);

for (const code of codes) {
  const mod = await import(`../src/i18n/locales/${code}.ts`);
  const keys = new Set(collect(mod.default.translation));
  const missing = [...enKeys].filter((k) => !keys.has(k));
  const coverage = (((enKeys.size - missing.length) / enKeys.size) * 100).toFixed(1);
  console.log(`${code}: ${keys.size} keys, ${coverage}% coverage (missing=${missing.length})`);
}
