import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// We can't directly import `request` since it's not exported — but we can test
// the behaviour through the public `api` object which calls `request` internally.
// We need to mock fetch and localStorage.

// Mock import.meta.env before importing the module
vi.stubGlobal('localStorage', {
  store: {} as Record<string, string>,
  getItem(key: string) { return this.store[key] ?? null; },
  setItem(key: string, val: string) { this.store[key] = val; },
  removeItem(key: string) { delete this.store[key]; },
  clear() { this.store = {}; },
});

// We need to import after mocks are set up
const { api } = await import('./client');

describe('API client', () => {
  const originalFetch = globalThis.fetch;
  const originalLocation = window.location;

  beforeEach(() => {
    localStorage.clear();
    // Mock window.location
    Object.defineProperty(window, 'location', {
      writable: true,
      value: { href: '/', reload: vi.fn() },
    });
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
    Object.defineProperty(window, 'location', {
      writable: true,
      value: originalLocation,
    });
    vi.restoreAllMocks();
  });

  it('sends JSON headers by default', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    await api.getLots();

    expect(globalThis.fetch).toHaveBeenCalledOnce();
    const [url, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/lots');
    expect(opts.headers['Content-Type']).toBe('application/json');
    expect(opts.headers['Accept']).toBe('application/json');
  });

  it('includes Authorization header when token is present', async () => {
    localStorage.setItem('parkhub_token', 'test-jwt-token');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve({ success: true, data: { id: '1' } }),
    });

    await api.me();

    const [, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(opts.headers['Authorization']).toBe('Bearer test-jwt-token');
  });

  it('omits Authorization header when no token', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    await api.getLots();

    const [, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(opts.headers['Authorization']).toBeUndefined();
  });

  it('sends POST with JSON body for login', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve({ success: true, data: { tokens: { access_token: 'abc' } } }),
    });

    await api.login('admin', 'demo');

    const [url, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/auth/login');
    expect(opts.method).toBe('POST');
    expect(JSON.parse(opts.body)).toEqual({ username: 'admin', password: 'demo' });
  });

  it('handles 401 by clearing token and redirecting to /login', async () => {
    localStorage.setItem('parkhub_token', 'expired-token');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 401,
      json: () => Promise.resolve({ error: { code: 'UNAUTHORIZED', message: 'Expired' } }),
    });

    const result = await api.me();

    expect(result.success).toBe(false);
    expect(result.error?.code).toBe('UNAUTHORIZED');
    expect(localStorage.getItem('parkhub_token')).toBeNull();
    expect(window.location.href).toBe('/login');
  });

  it('returns structured error for non-OK responses', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 404,
      statusText: 'Not Found',
      json: () => Promise.resolve({ error: { code: 'NOT_FOUND', message: 'Lot not found' } }),
    });

    const result = await api.getLot('nonexistent');

    expect(result.success).toBe(false);
    expect(result.data).toBeNull();
    expect(result.error?.code).toBe('NOT_FOUND');
    expect(result.error?.message).toBe('Lot not found');
  });

  it('falls back to HTTP status when response has no error body', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      statusText: 'Internal Server Error',
      json: () => Promise.resolve(null),
    });

    const result = await api.getLots();

    expect(result.success).toBe(false);
    expect(result.error?.code).toBe('HTTP_500');
    expect(result.error?.message).toBe('Internal Server Error');
  });

  it('handles network errors gracefully', async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new TypeError('Failed to fetch'));

    const result = await api.getLots();

    expect(result.success).toBe(false);
    expect(result.error?.code).toBe('NETWORK');
    expect(result.error?.message).toBe('Network error');
  });

  it('normalizes raw data responses (without success wrapper)', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve([{ id: '1', name: 'Lot A' }]),
    });

    const result = await api.getLots();

    expect(result.success).toBe(true);
    expect(result.data).toEqual([{ id: '1', name: 'Lot A' }]);
  });

  it('passes through pre-wrapped API responses', async () => {
    const apiResponse = { success: true, data: { id: '1', name: 'Lot A' } };
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve(apiResponse),
    });

    const result = await api.getLot('1');

    expect(result).toEqual(apiResponse);
  });

  it('handles json parse failure on error responses', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 502,
      statusText: 'Bad Gateway',
      json: () => Promise.reject(new Error('not json')),
    });

    const result = await api.getLots();

    expect(result.success).toBe(false);
    expect(result.error?.code).toBe('HTTP_502');
  });
});
