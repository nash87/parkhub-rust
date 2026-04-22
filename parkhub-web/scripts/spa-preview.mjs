import { createReadStream, existsSync } from 'node:fs';
import { stat } from 'node:fs/promises';
import http from 'node:http';
import https from 'node:https';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const distDir = path.resolve(__dirname, '..', 'dist');
const host = process.env.HOST || '127.0.0.1';
const port = Number(process.env.PORT || '4321');
const apiOrigin = new URL(process.env.API_ORIGIN || 'http://127.0.0.1:8081');

const MIME_TYPES = new Map([
  ['.css', 'text/css; charset=utf-8'],
  ['.html', 'text/html; charset=utf-8'],
  ['.ico', 'image/x-icon'],
  ['.js', 'text/javascript; charset=utf-8'],
  ['.json', 'application/json; charset=utf-8'],
  ['.map', 'application/json; charset=utf-8'],
  ['.mjs', 'text/javascript; charset=utf-8'],
  ['.png', 'image/png'],
  ['.svg', 'image/svg+xml'],
  ['.txt', 'text/plain; charset=utf-8'],
  ['.webmanifest', 'application/manifest+json; charset=utf-8'],
  ['.webp', 'image/webp'],
  ['.woff', 'font/woff'],
  ['.woff2', 'font/woff2'],
]);

const DEFAULT_HEADERS = {
  'x-content-type-options': 'nosniff',
  'referrer-policy': 'strict-origin-when-cross-origin',
};

const HTML_HEADERS = {
  ...DEFAULT_HEADERS,
  'content-security-policy': "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; connect-src 'self' ws: wss:; base-uri 'self'; form-action 'self';",
};

function send(res, statusCode, body, contentType = 'text/plain; charset=utf-8', headers = DEFAULT_HEADERS) {
  res.writeHead(statusCode, { ...headers, 'content-type': contentType });
  res.end(body);
}

function isSpaRoute(pathname) {
  return !pathname.startsWith('/api/') && !pathname.includes('.');
}

function isProxyRoute(pathname) {
  return pathname === '/health'
    || pathname.startsWith('/health/')
    || pathname.startsWith('/api/')
    || pathname.startsWith('/api-docs')
    || pathname.startsWith('/swagger-ui')
    || pathname.startsWith('/ws');
}

function proxyRequest(req, res, url) {
  const transport = apiOrigin.protocol === 'https:' ? https : http;
  const upstreamReq = transport.request({
    protocol: apiOrigin.protocol,
    hostname: apiOrigin.hostname,
    port: apiOrigin.port,
    method: req.method,
    path: `${url.pathname}${url.search}`,
    headers: {
      ...req.headers,
      host: apiOrigin.host,
    },
  }, (upstreamRes) => {
    res.writeHead(upstreamRes.statusCode || 502, {
      ...DEFAULT_HEADERS,
      ...upstreamRes.headers,
    });
    upstreamRes.pipe(res);
  });

  upstreamReq.on('error', (error) => {
    send(res, 502, error instanceof Error ? error.message : 'Bad Gateway');
  });

  req.pipe(upstreamReq);
}

async function resolveFile(pathname) {
  const decoded = decodeURIComponent(pathname);
  const normalized = path.normalize(decoded).replace(/^(\.\.[/\\])+/, '');
  const requestedPath = path.join(distDir, normalized);
  const indexPath = path.join(distDir, 'index.html');

  if (existsSync(requestedPath)) {
    const info = await stat(requestedPath);
    if (info.isFile()) return requestedPath;
  }

  if (pathname === '/' || isSpaRoute(pathname)) {
    return indexPath;
  }

  return null;
}

http.createServer(async (req, res) => {
  try {
    const url = new URL(req.url || '/', `http://${req.headers.host || `${host}:${port}`}`);
    if (isProxyRoute(url.pathname)) {
      proxyRequest(req, res, url);
      return;
    }

    const filePath = await resolveFile(url.pathname);
    if (!filePath) {
      send(res, 404, 'Not Found');
      return;
    }

    const ext = path.extname(filePath);
    const contentType = MIME_TYPES.get(ext) || 'application/octet-stream';
    const headers = ext === '.html' ? HTML_HEADERS : DEFAULT_HEADERS;
    res.writeHead(200, { ...headers, 'content-type': contentType });
    createReadStream(filePath).pipe(res);
  } catch (error) {
    send(res, 500, error instanceof Error ? error.message : 'Internal Server Error');
  }
}).listen(port, host, () => {
  console.log(`spa-preview ready on http://${host}:${port}`);
});
