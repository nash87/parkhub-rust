import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// We can't directly import `request` since it's not exported — but we can test
// the behaviour through the public `api` object which calls `request` internally.
// We need to mock fetch.

// We need to import after mocks are set up
const { api, setInMemoryToken, getInMemoryToken } = await import('./client');

describe('API client', () => {
  const originalFetch = globalThis.fetch;
  const originalLocation = window.location;

  beforeEach(() => {
    setInMemoryToken(null);
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

  it('sends JSON headers and credentials by default', async () => {
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
    expect(opts.headers['X-Requested-With']).toBe('XMLHttpRequest');
    expect(opts.credentials).toBe('include');
  });

  it('includes Authorization header when in-memory token is present', async () => {
    setInMemoryToken('test-jwt-token');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: () => Promise.resolve({ success: true, data: { id: '1' } }),
    });

    await api.me();

    const [, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(opts.headers['Authorization']).toBe('Bearer test-jwt-token');
  });

  it('omits Authorization header when no in-memory token', async () => {
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

  it('handles 401 by clearing in-memory token and dispatching auth:unauthorized', async () => {
    setInMemoryToken('expired-token');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 401,
      json: () => Promise.resolve({ error: { code: 'UNAUTHORIZED', message: 'Expired' } }),
    });

    const handler = vi.fn();
    window.addEventListener('auth:unauthorized', handler);

    const result = await api.me();

    expect(result.success).toBe(false);
    expect(result.error?.code).toBe('UNAUTHORIZED');
    expect(getInMemoryToken()).toBeNull();
    expect(handler).toHaveBeenCalledTimes(1);

    window.removeEventListener('auth:unauthorized', handler);
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

  // ── 2FA API ──

  it('calls 2FA setup endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { secret: 'ABC', otpauth_uri: 'otpauth://...', qr_code_base64: 'iVBOR...' } }),
    });
    const result = await api.setup2FA();
    expect(result.success).toBe(true);
    expect(result.data?.secret).toBe('ABC');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/auth/2fa/setup');
  });

  it('calls 2FA verify endpoint with code', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { enabled: true } }),
    });
    const result = await api.verify2FA('123456');
    expect(result.success).toBe(true);
    const [, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(JSON.parse(opts.body)).toEqual({ code: '123456' });
  });

  it('calls 2FA disable endpoint with password', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { enabled: false } }),
    });
    const result = await api.disable2FA('mypassword');
    expect(result.success).toBe(true);
    const [, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(JSON.parse(opts.body)).toEqual({ current_password: 'mypassword' });
  });

  it('calls get 2FA status endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { enabled: false } }),
    });
    const result = await api.get2FAStatus();
    expect(result.success).toBe(true);
    expect(result.data?.enabled).toBe(false);
  });

  // ── Login History ──

  it('calls login history endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [{ timestamp: '2026-03-22T00:00:00Z', ip_address: '10.0.0.1', user_agent: 'Chrome', success: true }] }),
    });
    const result = await api.getLoginHistory();
    expect(result.success).toBe(true);
    expect(result.data?.length).toBe(1);
  });

  // ── Sessions ──

  it('calls sessions list endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    const result = await api.getSessions();
    expect(result.success).toBe(true);
  });

  it('calls revoke session endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.revokeSession('abc123...');
    const [url, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/auth/sessions/abc123...');
    expect(opts.method).toBe('DELETE');
  });

  // ── Notification Preferences ──

  it('calls get notification preferences', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { email_booking_confirm: true, email_booking_reminder: true, email_swap_request: true, push_enabled: true } }),
    });
    const result = await api.getNotificationPreferences();
    expect(result.success).toBe(true);
    expect(result.data?.email_booking_confirm).toBe(true);
  });

  it('calls update notification preferences', async () => {
    const prefs = { email_booking_confirm: false, email_booking_reminder: true, email_swap_request: false, push_enabled: true };
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: prefs }),
    });
    const result = await api.updateNotificationPreferences(prefs);
    expect(result.success).toBe(true);
    const [, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(JSON.parse(opts.body).email_booking_confirm).toBe(false);
  });

  // ── Bulk Admin ──

  it('calls bulk update endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { total: 2, succeeded: 2, failed: 0, errors: [] } }),
    });
    const result = await api.adminBulkUpdate(['id1', 'id2'], 'activate');
    expect(result.success).toBe(true);
    expect(result.data?.succeeded).toBe(2);
  });

  it('calls bulk delete endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { total: 1, succeeded: 1, failed: 0, errors: [] } }),
    });
    const result = await api.adminBulkDelete(['id1']);
    expect(result.success).toBe(true);
    const [, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(JSON.parse(opts.body).user_ids).toEqual(['id1']);
  });

  // ── Logout ──

  it('calls logout endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    const result = await api.logout();
    expect(result.success).toBe(true);
    const [url, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/auth/logout');
    expect(opts.method).toBe('POST');
    expect(opts.credentials).toBe('include');
  });

  // ── In-memory token management ──

  it('setInMemoryToken/getInMemoryToken roundtrip', () => {
    expect(getInMemoryToken()).toBeNull();
    setInMemoryToken('test-123');
    expect(getInMemoryToken()).toBe('test-123');
    setInMemoryToken(null);
    expect(getInMemoryToken()).toBeNull();
  });
});
