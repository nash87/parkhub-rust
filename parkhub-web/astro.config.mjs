// @ts-check
import { defineConfig, fontProviders } from 'astro/config';
import react from '@astrojs/react';
import tailwindcss from '@tailwindcss/vite';
import reactCompiler from 'babel-plugin-react-compiler';
import { execSync } from 'node:child_process';

const buildHash = (() => {
  try {
    return execSync('git rev-parse --short HEAD', { encoding: 'utf8' }).trim();
  } catch {
    return Date.now().toString(36);
  }
})();

/** @type {import('astro').AstroIntegration} */
const swBuildHashIntegration = {
  name: 'sw-build-hash',
  hooks: {
    'astro:build:done': async ({ dir }) => {
      const { readFileSync, writeFileSync } = await import('node:fs');
      const swPath = new URL('sw.js', dir);
      try {
        const content = readFileSync(swPath, 'utf8');
        writeFileSync(swPath, content.replace('__BUILD_HASH__', buildHash));
      } catch {
        // sw.js not present — skip
      }
    },
  },
};

export default defineConfig({
  output: 'static',
  integrations: [react({ babel: { plugins: [reactCompiler] } }), swBuildHashIntegration],
  vite: {
    plugins: [tailwindcss()],
    define: {
      'import.meta.env.VITE_API_URL': JSON.stringify(process.env.VITE_API_URL || ''),
    },
    build: {
      rollupOptions: {
        output: {
          manualChunks(id) {
            if (!id.includes('node_modules')) return;
            if (/node_modules\/(react|react-dom|react-router|react-router-dom)\//.test(id))
              return 'vendor-react';
            if (/node_modules\/framer-motion\//.test(id))
              return 'vendor-motion';
            if (/node_modules\/(i18next|react-i18next|i18next-browser-languagedetector)\//.test(id))
              return 'vendor-i18n';
          },
        },
      },
    },
  },
  fonts: process.env.CI || process.env.DOCKER ? [] : [
    {
      name: 'Outfit',
      cssVariable: '--font-outfit',
      provider: fontProviders.google(),
      weights: [400, 500, 600, 700, 800],
      styles: ['normal'],
      subsets: ['latin'],
      fallbacks: ['system-ui', 'sans-serif'],
    },
    {
      name: 'Work Sans',
      cssVariable: '--font-work-sans',
      provider: fontProviders.google(),
      weights: [300, 400, 500, 600, 700],
      styles: ['normal'],
      subsets: ['latin'],
      fallbacks: ['system-ui', 'sans-serif'],
    },
  ],
});
