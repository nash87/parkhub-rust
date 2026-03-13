// ParkHub Service Worker — Cache-first for static assets, network-first for API
// Version is updated during build; old caches are purged on activate.
const CACHE_VERSION = '__BUILD_HASH__';
const CACHE_NAME = `parkhub-${CACHE_VERSION}`;
const STATIC_ASSETS = ['/', '/favicon.svg', '/icons/icon.svg'];
const MAX_CACHE_AGE_MS = 7 * 24 * 60 * 60 * 1000; // 7 days

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(CACHE_NAME).then((cache) => cache.addAll(STATIC_ASSETS))
  );
  self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(keys.filter((k) => k !== CACHE_NAME).map((k) => caches.delete(k)))
    )
  );
  self.clients.claim();
});

self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // Skip non-GET and API/auth requests
  if (request.method !== 'GET') return;
  if (url.pathname.startsWith('/api/')) return;
  if (url.pathname.startsWith('/health')) return;

  // Cache-first for static assets
  if (
    url.pathname.match(/\.(js|css|woff2?|png|svg|jpg|ico)$/) ||
    url.pathname.startsWith('/icons/')
  ) {
    event.respondWith(
      caches.match(request).then((cached) =>
        cached || fetch(request).then((response) => {
          if (response.ok) {
            const clone = response.clone();
            caches.open(CACHE_NAME).then((cache) => cache.put(request, clone));
          }
          return response;
        })
      )
    );
    return;
  }

  // Network-first for HTML (SPA navigation)
  event.respondWith(
    fetch(request)
      .then((response) => {
        if (response.ok && url.pathname === '/') {
          const clone = response.clone();
          caches.open(CACHE_NAME).then((cache) => cache.put(request, clone));
        }
        return response;
      })
      .catch(() => caches.match('/'))
  );
});
