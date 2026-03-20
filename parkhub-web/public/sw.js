// ParkHub Service Worker — Enhanced PWA with offline support
// Version is updated during build via astro integration; old caches are purged on activate.
const CACHE_VERSION = '__BUILD_HASH__';
const STATIC_CACHE = `parkhub-static-${CACHE_VERSION}`;
const API_CACHE = `parkhub-api-${CACHE_VERSION}`;
const OFFLINE_PAGE = '/offline.html';

const PRECACHE_URLS = ['/', '/favicon.svg', '/icons/icon.svg', OFFLINE_PAGE];

// API paths eligible for stale-while-revalidate caching
const CACHEABLE_API_PATHS = [
  '/api/v1/bookings',
  '/api/v1/lots',
  '/api/v1/profile',
  '/api/v1/theme',
  '/api/v1/vehicles',
  '/api/v1/credits',
  '/api/v1/announcements',
];

const MAX_API_CACHE_AGE_MS = 24 * 60 * 60 * 1000; // 24 hours
const SYNC_QUEUE_STORE = 'parkhub-sync-queue';

// ── Install ──────────────────────────────────────────────────────────────────
self.addEventListener('install', (event) => {
  event.waitUntil(
    caches.open(STATIC_CACHE).then((cache) => cache.addAll(PRECACHE_URLS))
  );
  self.skipWaiting();
});

// ── Activate ─────────────────────────────────────────────────────────────────
self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches.keys().then((keys) =>
      Promise.all(
        keys
          .filter((k) => k !== STATIC_CACHE && k !== API_CACHE)
          .map((k) => caches.delete(k))
      )
    )
  );
  self.clients.claim();
});

// ── Fetch ────────────────────────────────────────────────────────────────────
self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);

  // Only handle same-origin requests
  if (url.origin !== self.location.origin) return;

  // Queue non-GET API mutations when offline (background sync)
  if (request.method !== 'GET' && url.pathname.startsWith('/api/')) {
    event.respondWith(handleMutation(request));
    return;
  }

  // Skip non-GET requests
  if (request.method !== 'GET') return;

  // Stale-while-revalidate for cacheable API endpoints
  if (isCacheableApi(url.pathname)) {
    event.respondWith(staleWhileRevalidate(request, API_CACHE));
    return;
  }

  // Skip non-cacheable API and health endpoints
  if (url.pathname.startsWith('/api/') || url.pathname.startsWith('/health')) return;

  // Cache-first for static assets (js, css, fonts, images)
  if (isStaticAsset(url.pathname)) {
    event.respondWith(cacheFirst(request, STATIC_CACHE));
    return;
  }

  // Network-first for HTML navigation
  event.respondWith(networkFirstNavigation(request));
});

// ── Background Sync ──────────────────────────────────────────────────────────
self.addEventListener('sync', (event) => {
  if (event.tag === 'parkhub-mutation-sync') {
    event.waitUntil(replayMutationQueue());
  }
});

// Listen for manual sync trigger from client
self.addEventListener('message', (event) => {
  if (event.data?.type === 'REPLAY_SYNC_QUEUE') {
    replayMutationQueue().then(() => {
      event.source?.postMessage({ type: 'SYNC_COMPLETE' });
    });
  }
  if (event.data?.type === 'GET_CACHE_VERSION') {
    event.source?.postMessage({ type: 'CACHE_VERSION', version: CACHE_VERSION });
  }
});

// ── Strategies ───────────────────────────────────────────────────────────────

function isStaticAsset(pathname) {
  return (
    /\.(js|css|woff2?|png|svg|jpg|jpeg|webp|avif|ico)$/.test(pathname) ||
    pathname.startsWith('/icons/')
  );
}

function isCacheableApi(pathname) {
  return CACHEABLE_API_PATHS.some(
    (p) => pathname === p || pathname.startsWith(p + '/')
  );
}

/** Cache-first: return cached response, or fetch + cache */
function cacheFirst(request, cacheName) {
  return caches.match(request).then(
    (cached) =>
      cached ||
      fetch(request).then((response) => {
        if (response.ok) {
          const clone = response.clone();
          caches.open(cacheName).then((cache) => cache.put(request, clone));
        }
        return response;
      })
  );
}

/** Stale-while-revalidate: return cached immediately, fetch in background to update cache */
function staleWhileRevalidate(request, cacheName) {
  return caches.open(cacheName).then((cache) =>
    cache.match(request).then((cached) => {
      const fetchPromise = fetch(request)
        .then((response) => {
          if (response.ok) {
            // Store with timestamp header for cache expiry
            const headers = new Headers(response.headers);
            headers.set('X-SW-Cached-At', Date.now().toString());
            const timedResponse = new Response(response.clone().body, {
              status: response.status,
              statusText: response.statusText,
              headers,
            });
            cache.put(request, timedResponse);
          }
          return response;
        })
        .catch((err) => {
          // Network failed — if we have cached data, it was already returned
          if (cached) return cached;
          throw err;
        });

      // Return cached response immediately if fresh enough, otherwise wait for network
      if (cached) {
        const cachedAt = parseInt(cached.headers.get('X-SW-Cached-At') || '0', 10);
        if (Date.now() - cachedAt < MAX_API_CACHE_AGE_MS) {
          // Still fresh — return cached, update in background
          return cached;
        }
      }

      // No cache or stale beyond limit — wait for network
      return fetchPromise;
    })
  );
}

/** Network-first for navigation: try network, fall back to cache, then offline page */
function networkFirstNavigation(request) {
  return fetch(request)
    .then((response) => {
      if (response.ok) {
        const clone = response.clone();
        caches.open(STATIC_CACHE).then((cache) => cache.put(request, clone));
      }
      return response;
    })
    .catch(() =>
      caches.match(request).then(
        (cached) => cached || caches.match(OFFLINE_PAGE) || offlineFallbackResponse()
      )
    );
}

/** Inline fallback if offline.html wasn't cached */
function offlineFallbackResponse() {
  return new Response(
    '<html><body><h1>Offline</h1><p>ParkHub is not available offline.</p></body></html>',
    { status: 503, headers: { 'Content-Type': 'text/html' } }
  );
}

// ── Mutation Queue (Background Sync) ─────────────────────────────────────────

async function handleMutation(request) {
  try {
    const response = await fetch(request.clone());
    return response;
  } catch {
    // Network unavailable — queue the mutation
    await queueMutation(request);
    return new Response(
      JSON.stringify({
        queued: true,
        message: 'You are offline. This action has been queued and will be sent when you reconnect.',
      }),
      {
        status: 202,
        headers: { 'Content-Type': 'application/json' },
      }
    );
  }
}

async function queueMutation(request) {
  const body = await request.clone().text();
  const entry = {
    url: request.url,
    method: request.method,
    headers: Object.fromEntries(request.headers.entries()),
    body,
    timestamp: Date.now(),
  };

  const cache = await caches.open(SYNC_QUEUE_STORE);
  const queueResponse = await cache.match('queue');
  const queue = queueResponse ? await queueResponse.json() : [];
  queue.push(entry);
  await cache.put('queue', new Response(JSON.stringify(queue)));

  // Register for background sync if available
  if (self.registration.sync) {
    try {
      await self.registration.sync.register('parkhub-mutation-sync');
    } catch {
      // Background sync not supported — will replay on next online event
    }
  }

  // Notify all clients about the queued mutation
  const clients = await self.clients.matchAll();
  clients.forEach((client) =>
    client.postMessage({ type: 'MUTATION_QUEUED', queueLength: queue.length })
  );
}

async function replayMutationQueue() {
  const cache = await caches.open(SYNC_QUEUE_STORE);
  const queueResponse = await cache.match('queue');
  if (!queueResponse) return;

  const queue = await queueResponse.json();
  if (!queue.length) return;

  const remaining = [];
  for (const entry of queue) {
    try {
      await fetch(entry.url, {
        method: entry.method,
        headers: entry.headers,
        body: entry.body || undefined,
      });
    } catch {
      // Still offline — keep in queue
      remaining.push(entry);
    }
  }

  if (remaining.length) {
    await cache.put('queue', new Response(JSON.stringify(remaining)));
  } else {
    await cache.delete('queue');
  }

  // Notify clients about sync result
  const clients = await self.clients.matchAll();
  const synced = queue.length - remaining.length;
  clients.forEach((client) =>
    client.postMessage({
      type: 'SYNC_RESULT',
      synced,
      remaining: remaining.length,
    })
  );
}
