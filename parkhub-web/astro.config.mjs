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

/**
 * Trim the landing-page critical path: (1) mark the render-blocking stylesheet
 * and entry module with fetchpriority=high so they aren't starved by the many
 * small icon chunks Vite emits, (2) preload the tiny Welcome lazy chunk that
 * actually paints the LCP element. The full theme system keeps working — we
 * only rewrite resource hints, never inline a specific theme.
 * @type {import('astro').AstroIntegration}
 */
const perfBoostIntegration = {
  name: 'perf-boost',
  hooks: {
    'astro:build:done': async ({ dir }) => {
      const { readFileSync, writeFileSync, readdirSync } = await import('node:fs');
      const htmlPath = new URL('index.html', dir);
      const astroDir = new URL('_astro/', dir);
      let html, files;
      try {
        html = readFileSync(htmlPath, 'utf8');
        files = readdirSync(astroDir);
      } catch {
        return;
      }

      // 1. Tell the browser to fetch the render-blocking stylesheet with
      //    high priority so it doesn't get starved by script downloads.
      //    Async-loading was tested but re-paints from late-arriving CSS
      //    actually pushed LCP LATER (the subtitle text grows when Tailwind
      //    classes apply, triggering a new LCP event), so we stick with a
      //    prioritized render-blocking link.
      const primaryCssRe = /<link rel="stylesheet" href="(\/_astro\/index@_@astro\.[^"]+\.css)">/;
      html = html.replace(primaryCssRe, (_all, href) =>
        `<link rel="stylesheet" href="${href}" fetchpriority="high">`);

      // 1b. Hoist the entry `<script type="module">` to <head> with
      //     fetchpriority=high so it downloads in parallel with CSS instead
      //     of waiting for the browser to reach <body>. `async` preserves
      //     the default module-script behavior (run after DOMContentLoaded).
      const scriptRe = /<script type="module" src="(\/_astro\/index\.astro_astro_type_script_index_0_lang\.[^"]+\.js)"><\/script>/;
      const sm = html.match(scriptRe);
      if (sm) {
        const jsHref = sm[1];
        html = html.replace(scriptRe, '');
        const hoisted = `<script type="module" src="${jsHref}" fetchpriority="high"></script>`;
        html = html.replace('</head>', `${hoisted}</head>`);
      }

      // 2. Preload the lazy Welcome chunk. It's only ~10KB and renders the
      //    LCP text — browsers would otherwise discover it late via dynamic
      //    import inside AppRoutes. Preloading the bigger vendor chunks was
      //    tested and hurt FCP on mobile (bandwidth contention with CSS), so
      //    we let the entry script's static <link modulepreload> graph cover
      //    those.
      const criticalBases = ['Welcome'];
      const hints = [];
      for (const base of criticalBases) {
        const match = files.find(f => f.startsWith(`${base}.`) && f.endsWith('.js'));
        if (match) hints.push(`<link rel="modulepreload" href="/_astro/${match}">`);
      }
      if (hints.length && html.includes('</head>')) {
        html = html.replace('</head>', `${hints.join('')}</head>`);
      }

      writeFileSync(htmlPath, html);
    },
  },
};

export default defineConfig({
  output: 'static',
  integrations: [react({ babel: { plugins: [reactCompiler] } }), swBuildHashIntegration, perfBoostIntegration],
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
