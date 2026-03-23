#!/usr/bin/env node
// Script to find missing i18n keys by comparing locale files structurally
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const localesDir = join(__dirname, 'src/i18n/locales');

// Import locale files using dynamic import
async function run() {
  const en = (await import(join(localesDir, 'en.ts'))).default;

  // Flatten nested object to dot-separated keys
  function flattenKeys(obj, prefix = '') {
    const keys = {};
    for (const [key, value] of Object.entries(obj)) {
      const fullKey = prefix ? `${prefix}.${key}` : key;
      if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
        Object.assign(keys, flattenKeys(value, fullKey));
      } else {
        keys[fullKey] = value;
      }
    }
    return keys;
  }

  const enFlat = flattenKeys(en);
  console.log(`EN has ${Object.keys(enFlat).length} total keys`);

  const locales = ['de', 'fr', 'es', 'it', 'pt', 'tr', 'pl', 'ja', 'zh'];

  for (const lang of locales) {
    try {
      const locale = (await import(join(localesDir, `${lang}.ts`))).default;
      const localeFlat = flattenKeys(locale);

      const missingKeys = [];
      for (const key of Object.keys(enFlat)) {
        if (!(key in localeFlat)) {
          missingKeys.push(key);
        }
      }

      console.log(`\n${lang.toUpperCase()}: ${Object.keys(localeFlat).length} keys, missing ${missingKeys.length}`);
      if (missingKeys.length > 0) {
        const sections = {};
        for (const key of missingKeys) {
          const section = key.split('.').slice(0, 2).join('.');
          if (!sections[section]) sections[section] = [];
          sections[section].push(key);
        }
        for (const [section, keys] of Object.entries(sections)) {
          console.log(`  ${section}: ${keys.length} keys`);
          for (const k of keys) {
            console.log(`    - ${k} = "${enFlat[k]}"`);
          }
        }
      }
    } catch (e) {
      console.log(`\n${lang.toUpperCase()}: ERROR - ${e.message}`);
    }
  }
}

run().catch(console.error);
