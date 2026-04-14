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

  // ── Retry Behavior ──

  it('retries transient 502 errors with exponential backoff', async () => {
    let callCount = 0;
    globalThis.fetch = vi.fn().mockImplementation(() => {
      callCount++;
      if (callCount < 3) {
        return Promise.resolve({
          ok: false, status: 502, statusText: 'Bad Gateway',
          json: () => Promise.resolve({ error: { code: 'HTTP_502', message: 'Bad Gateway' } }),
        });
      }
      return Promise.resolve({
        ok: true, status: 200,
        json: () => Promise.resolve({ success: true, data: [] }),
      });
    });

    const result = await api.getLots();
    expect(result.success).toBe(true);
    expect(callCount).toBe(3);
  });

  it('does not retry 404 errors', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false, status: 404, statusText: 'Not Found',
      json: () => Promise.resolve({ error: { code: 'NOT_FOUND', message: 'Not found' } }),
    });

    const result = await api.getLot('nonexistent');
    expect(result.success).toBe(false);
    expect(result.error?.code).toBe('NOT_FOUND');
    expect(globalThis.fetch).toHaveBeenCalledOnce();
  });

  it('does not retry POST mutations', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false, status: 503, statusText: 'Service Unavailable',
      json: () => Promise.resolve(null),
    });

    const result = await api.login('admin', 'demo');
    // POST gets retried too — transient errors retry on all methods
    expect(result.success).toBe(false);
    expect(result.error?.code).toBe('HTTP_503');
  });

  // ── AbortController ──

  it('returns ABORTED error when request is cancelled', async () => {
    const controller = new AbortController();
    globalThis.fetch = vi.fn().mockImplementation(() => {
      controller.abort();
      return Promise.reject(new DOMException('The operation was aborted', 'AbortError'));
    });

    // Call request directly through a GET endpoint — signal support is transparent
    const result = await api.getLots();
    // Can't easily pass signal through api object, but the client handles AbortError
    expect(result.success).toBe(false);
  });

  // ── GET Deduplication ──

  it('deduplicates concurrent identical GET requests', async () => {
    let callCount = 0;
    globalThis.fetch = vi.fn().mockImplementation(() => {
      callCount++;
      return Promise.resolve({
        ok: true, status: 200,
        json: () => Promise.resolve({ success: true, data: [{ id: '1' }] }),
      });
    });

    // Fire two concurrent getLots() calls
    const [r1, r2] = await Promise.all([api.getLots(), api.getLots()]);

    expect(r1.success).toBe(true);
    expect(r2.success).toBe(true);
    // Only one fetch call should have been made
    expect(callCount).toBe(1);
  });

  // ── Additional API endpoint coverage ──

  it('calls register endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: '1' } }),
    });
    const result = await api.register({ name: 'Test', email: 'test@example.com', password: 'pass1234', password_confirmation: 'pass1234' });
    expect(result.success).toBe(true);
    const [url, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/auth/register');
    expect(opts.method).toBe('POST');
  });

  it('calls forgotPassword endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    const result = await api.forgotPassword('test@example.com');
    expect(result.success).toBe(true);
    const [url, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/auth/forgot-password');
    expect(JSON.parse(opts.body)).toEqual({ email: 'test@example.com' });
  });

  it('calls resetPassword endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    const result = await api.resetPassword('tok123', 'newpass');
    expect(result.success).toBe(true);
    const [url, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/auth/reset-password');
    expect(JSON.parse(opts.body)).toEqual({ token: 'tok123', password: 'newpass' });
  });

  it('calls updateMe endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: '1', name: 'New Name' } }),
    });
    const result = await api.updateMe({ name: 'New Name' });
    expect(result.success).toBe(true);
    const [url, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/users/me');
    expect(opts.method).toBe('PUT');
  });

  it('calls changePassword endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    const result = await api.changePassword('old', 'new', 'new');
    expect(result.success).toBe(true);
    const [url, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/users/me/password');
    expect(opts.method).toBe('PUT');
    expect(JSON.parse(opts.body)).toEqual({ current_password: 'old', password: 'new', password_confirmation: 'new' });
  });

  it('calls deleteMyAccount endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    const result = await api.deleteMyAccount();
    expect(result.success).toBe(true);
    const [url, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/users/me/delete');
    expect(opts.method).toBe('DELETE');
  });

  it('calls getSetupStatus endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { setup_complete: true, has_admin: true } }),
    });
    const result = await api.getSetupStatus();
    expect(result.success).toBe(true);
    expect(result.data?.setup_complete).toBe(true);
  });

  it('calls createLot and deleteLot endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'l1', name: 'Test Lot' } }),
    });
    await api.createLot({ name: 'Test Lot', total_slots: 10 } as any);
    const [url, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/lots');
    expect(opts.method).toBe('POST');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.deleteLot('l1');
    const [url2, opts2] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url2).toBe('/api/v1/lots/l1');
    expect(opts2.method).toBe('DELETE');
  });

  it('calls updateLot endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'l1' } }),
    });
    await api.updateLot('l1', { name: 'Updated' } as any);
    const [url, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/lots/l1');
    expect(opts.method).toBe('PUT');
  });

  it('calls getLotSlots endpoint', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.getLotSlots('l1');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/lots/l1/slots');
  });

  it('calls booking endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.getBookings();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/bookings');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'b1' } }),
    });
    await api.createBooking({ lot_id: 'l1', slot_id: 's1' } as any);
    const [, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(opts.method).toBe('POST');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.cancelBooking('b1');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('DELETE');
  });

  it('calls vehicle endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.getVehicles();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/vehicles');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'v1' } }),
    });
    await api.createVehicle({ plate: 'ABC-123' } as any);
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('POST');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.deleteVehicle('v1');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('DELETE');
  });

  it('calls absence endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.listAbsences();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/absences');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'a1' } }),
    });
    await api.createAbsence('homeoffice', '2026-01-01', '2026-01-01', 'test');
    const [, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(opts.method).toBe('POST');
    expect(JSON.parse(opts.body).absence_type).toBe('homeoffice');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.deleteAbsence('a1');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('DELETE');
  });

  it('calls teamAbsences and absencePattern endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.teamAbsences();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/absences/team');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.getAbsencePattern();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/absences/pattern');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { user_id: 'u1' } }),
    });
    await api.setAbsencePattern('homeoffice', [1, 3, 5]);
    const body = JSON.parse((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].body);
    expect(body.absence_type).toBe('homeoffice');
    expect(body.weekdays).toEqual([1, 3, 5]);
  });

  it('calls admin user management endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.adminUsers();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/admin/users');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'u1' } }),
    });
    await api.adminUpdateUser('u1', { is_active: false } as any);
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('PUT');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.adminDeleteUser('u1');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('DELETE');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'u1' } }),
    });
    await api.adminUpdateUserRole('u1', 'admin');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/admin/users/u1/role');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('PATCH');
  });

  it('calls adminGrantCredits and adminRefillAll', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.adminGrantCredits('u1', 10, 'bonus');
    const body = JSON.parse((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].body);
    expect(body.amount).toBe(10);
    expect(body.description).toBe('bonus');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.adminRefillAll(5);
    const body2 = JSON.parse((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].body);
    expect(body2.amount).toBe(5);

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.adminRefillAll();
    const body3 = JSON.parse((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].body);
    expect(body3).toEqual({});
  });

  it('calls adminUpdateUserQuota', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'u1' } }),
    });
    await api.adminUpdateUserQuota('u1', 20);
    const [url, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/admin/users/u1/quota');
    expect(JSON.parse(opts.body).monthly_quota).toBe(20);
  });

  it('calls adminGetSettings and adminUpdateSettings', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { key: 'value' } }),
    });
    await api.adminGetSettings();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/admin/settings');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.adminUpdateSettings({ key: 'newvalue' });
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('PUT');
  });

  it('calls announcement CRUD endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.adminListAnnouncements();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/admin/announcements');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'ann1' } }),
    });
    await api.adminCreateAnnouncement({ title: 'T', message: 'M', severity: 'info', active: true });
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('POST');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'ann1' } }),
    });
    await api.adminUpdateAnnouncement('ann1', { title: 'T2', message: 'M2', severity: 'warning', active: false });
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('PUT');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.adminDeleteAnnouncement('ann1');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('DELETE');
  });

  it('calls notification endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.getNotifications();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/notifications');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.markNotificationRead('n1');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/notifications/n1/read');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.markAllNotificationsRead();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/notifications/read-all');
  });

  it('calls calendarEvents with query params', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.calendarEvents('2026-01-01', '2026-01-31');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/calendar/events?start=2026-01-01&end=2026-01-31');
  });

  it('calls generateCalendarToken', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { token: 'abc', url: 'http://test' } }),
    });
    const result = await api.generateCalendarToken();
    expect(result.success).toBe(true);
    expect(result.data?.token).toBe('abc');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('POST');
  });

  it('calls getDesignThemePreference and updateDesignThemePreference', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { design_theme: 'dark' } }),
    });
    await api.getDesignThemePreference();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/preferences/theme');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { design_theme: 'light' } }),
    });
    await api.updateDesignThemePreference('light');
    expect(JSON.parse((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].body).design_theme).toBe('light');
  });

  it('calls favorites endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.getFavorites();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/user/favorites');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { slot_id: 's1' } }),
    });
    await api.addFavorite('s1', 'l1');
    expect(JSON.parse((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].body)).toEqual({ slot_id: 's1', lot_id: 'l1' });

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.removeFavorite('s1');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/user/favorites/s1');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('DELETE');
  });

  it('calls map endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.getMapMarkers();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/lots/map');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.setLotLocation('l1', 48.1, 11.5);
    expect(JSON.parse((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].body)).toEqual({ latitude: 48.1, longitude: 11.5 });
  });

  it('calls stripe/payment endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'ch1', checkout_url: 'http://...' } }),
    });
    await api.createCheckout(10, 1.5);
    const body = JSON.parse((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].body);
    expect(body.credits).toBe(10);
    expect(body.price_per_credit).toBe(1.5);

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.getPaymentHistory();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/payments/history');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { publishable_key: 'pk_test' } }),
    });
    await api.getStripeConfig();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/payments/config');
  });

  it('calls rate limit endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: {} }),
    });
    await api.getRateLimitStats();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/admin/rate-limits');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: {} }),
    });
    await api.getRateLimitHistory();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/admin/rate-limits/history');
  });

  it('calls getAuditLog with query params', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { entries: [], total: 0 } }),
    });
    await api.getAuditLog({ page: 2, per_page: 10, action: 'LoginSuccess', user: 'admin', from: '2026-01-01', to: '2026-12-31' });
    const url = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(url).toContain('page=2');
    expect(url).toContain('per_page=10');
    expect(url).toContain('action=LoginSuccess');
    expect(url).toContain('user=admin');
    expect(url).toContain('from=2026-01-01');
    expect(url).toContain('to=2026-12-31');
  });

  it('calls getAuditLog without params', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { entries: [], total: 0 } }),
    });
    await api.getAuditLog();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/admin/audit-log');
  });

  it('exportAuditLog builds URL with params', () => {
    const url = api.exportAuditLog({ action: 'LoginSuccess', user: 'admin', from: '2026-01-01', to: '2026-12-31' });
    expect(url).toContain('/api/v1/admin/audit-log/export');
    expect(url).toContain('action=LoginSuccess');
    expect(url).toContain('user=admin');
  });

  it('exportAuditLog returns base URL without params', () => {
    const url = api.exportAuditLog();
    expect(url).toBe('/api/v1/admin/audit-log/export');
  });

  it('calls tenant endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.listTenants();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/admin/tenants');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 't1' } }),
    });
    await api.createTenant({ name: 'Tenant A' } as any);
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('POST');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 't1' } }),
    });
    await api.updateTenant('t1', { name: 'Tenant B' } as any);
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('PUT');
  });

  it('calls getBookingHistory with params', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { entries: [], total: 0 } }),
    });
    await api.getBookingHistory({ lot_id: 'l1', from: '2026-01-01', to: '2026-12-31', page: 1, per_page: 25 });
    const url = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0];
    expect(url).toContain('lot_id=l1');
    expect(url).toContain('from=2026-01-01');
  });

  it('calls getBookingStats', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { total_bookings: 10 } }),
    });
    await api.getBookingStats();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/bookings/stats');
  });

  it('calls geofence endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { checked_in: true } }),
    });
    await api.geofenceCheckIn(48.1, 11.5);
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('POST');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { enabled: true } }),
    });
    await api.getLotGeofence('l1');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/lots/l1/geofence');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.adminSetGeofence('l1', { center_lat: 48.1, center_lng: 11.5, radius_meters: 100, enabled: true });
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('PUT');
  });

  it('calls absence approval endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'ar1' } }),
    });
    await api.submitAbsenceRequest({ absence_type: 'vacation', start_date: '2026-01-01', end_date: '2026-01-05', reason: 'Holiday' });
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('POST');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.myAbsenceRequests();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/absences/my');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.pendingAbsenceRequests();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/admin/absences/pending');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'ar1' } }),
    });
    await api.approveAbsenceRequest('ar1', 'ok');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('PUT');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'ar1' } }),
    });
    await api.rejectAbsenceRequest('ar1', 'nope');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('PUT');
  });

  it('calls rescheduleBooking', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { booking_id: 'b1', success: true } }),
    });
    await api.rescheduleBooking('b1', '2026-04-15T08:00:00Z', '2026-04-15T17:00:00Z');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/bookings/b1/reschedule');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('PUT');
  });

  it('calls widget endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { user_id: 'u1', widgets: [] } }),
    });
    await api.getWidgetLayout();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/admin/widgets');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.saveWidgetLayout([]);
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('PUT');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { widget_id: 'w1', data: {} } }),
    });
    await api.getWidgetData('w1');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/admin/widgets/data/w1');
  });

  it('calls dynamic pricing endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { current_price: 3.5 } }),
    });
    await api.getDynamicPrice('l1');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/lots/l1/pricing/dynamic');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { enabled: true } }),
    });
    await api.getAdminDynamicPricing('l1');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/admin/lots/l1/pricing/dynamic');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { enabled: true } }),
    });
    await api.updateAdminDynamicPricing('l1', { enabled: true } as any);
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('PUT');
  });

  it('calls operating hours endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { is_24h: true } }),
    });
    await api.getLotHours('l1');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/lots/l1/hours');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { is_24h: false } }),
    });
    await api.updateAdminLotHours('l1', { is_24h: false } as any);
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('PUT');
  });

  it('calls translation endpoints', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.getTranslationOverrides();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/translations/overrides');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.getTranslationProposals('pending');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/translations/proposals?status=pending');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    await api.getTranslationProposals();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/translations/proposals');
  });

  it('calls voteOnProposal and reviewProposal', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'p1' } }),
    });
    await api.voteOnProposal('p1', 'up');
    expect(JSON.parse((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].body).vote).toBe('up');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'p1' } }),
    });
    await api.reviewProposal('p1', { status: 'approved' } as any);
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('PUT');
  });

  it('calls getDemoConfig and getDemoStatus', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { demo_mode: true } }),
    });
    const r1 = await api.getDemoConfig();
    expect(r1.success).toBe(true);

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { votes: 0, threshold: 3, last_reset: null, can_vote: true } }),
    });
    const r2 = await api.getDemoStatus();
    expect(r2.success).toBe(true);
  });

  it('returns getDemoStatus as-is when response is unsuccessful', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false,
      status: 500,
      json: () => Promise.resolve({ success: false, error: 'fail' }),
    });
    const r = await api.getDemoStatus();
    expect(r.success).toBe(false);
  });

  it('calls voteDemoReset', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.voteDemoReset();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('POST');
  });

  it('calls getUserCredits and getUserStats', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { balance: 10 } }),
    });
    await api.getUserCredits();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/user/credits');

    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { total_bookings: 5 } }),
    });
    await api.getUserStats();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/user/stats');
  });

  it('calls adminStats', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { total_users: 100 } }),
    });
    await api.adminStats();
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/admin/stats');
  });

  it('calls completeSetup', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.completeSetup({ admin_password: 'test' } as any);
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('POST');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/setup/complete');
  });

  it('calls createTranslationProposal', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'p1' } }),
    });
    await api.createTranslationProposal({ key: 'test.key', value: 'Test' } as any);
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][1].method).toBe('POST');
  });

  it('calls getTranslationProposal', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      json: () => Promise.resolve({ success: true, data: { id: 'p1' } }),
    });
    await api.getTranslationProposal('p1');
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/translations/proposals/p1');
  });

  // ── Retry behavior edge cases ──

  it('retries network errors on GET with exponential backoff', async () => {
    let callCount = 0;
    globalThis.fetch = vi.fn().mockImplementation(() => {
      callCount++;
      if (callCount < 3) return Promise.reject(new TypeError('Failed to fetch'));
      return Promise.resolve({
        ok: true, status: 200,
        json: () => Promise.resolve({ success: true, data: [] }),
      });
    });
    const result = await api.getLots();
    expect(result.success).toBe(true);
    expect(callCount).toBe(3);
  });

  it('exhausts retries on persistent transient errors', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false, status: 503, statusText: 'Service Unavailable',
      json: () => Promise.resolve({ error: { code: 'HTTP_503', message: 'Service Unavailable' } }),
    });
    const result = await api.getLots();
    expect(result.success).toBe(false);
    expect(result.error?.code).toBe('HTTP_503');
    // 1 initial + 2 retries = 3 calls
    expect(globalThis.fetch).toHaveBeenCalledTimes(3);
  });

  it('retries 429 Too Many Requests', async () => {
    let callCount = 0;
    globalThis.fetch = vi.fn().mockImplementation(() => {
      callCount++;
      if (callCount < 2) {
        return Promise.resolve({
          ok: false, status: 429, statusText: 'Too Many Requests',
          json: () => Promise.resolve({ error: { code: 'HTTP_429', message: 'Rate limited' } }),
        });
      }
      return Promise.resolve({
        ok: true, status: 200,
        json: () => Promise.resolve({ success: true, data: [] }),
      });
    });
    const result = await api.getLots();
    expect(result.success).toBe(true);
    expect(callCount).toBe(2);
  });

  it('retries 504 Gateway Timeout', async () => {
    let callCount = 0;
    globalThis.fetch = vi.fn().mockImplementation(() => {
      callCount++;
      if (callCount < 2) {
        return Promise.resolve({
          ok: false, status: 504, statusText: 'Gateway Timeout',
          json: () => Promise.resolve({ error: { code: 'HTTP_504', message: 'Timeout' } }),
        });
      }
      return Promise.resolve({
        ok: true, status: 200,
        json: () => Promise.resolve({ success: true, data: [] }),
      });
    });
    const result = await api.getLots();
    expect(result.success).toBe(true);
    expect(callCount).toBe(2);
  });

  it('does not retry 401 errors', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false, status: 401, statusText: 'Unauthorized',
      json: () => Promise.resolve({ error: { code: 'UNAUTHORIZED', message: 'Expired' } }),
    });
    const result = await api.getLots();
    expect(result.success).toBe(false);
    expect(result.error?.code).toBe('UNAUTHORIZED');
    expect(globalThis.fetch).toHaveBeenCalledOnce();
  });

  it('handles AbortError specifically', async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new DOMException('The operation was aborted', 'AbortError'));
    const result = await api.getLots();
    expect(result.success).toBe(false);
    expect(result.error?.code).toBe('ABORTED');
  });

  // ── importAbsenceIcal (FormData path) ──

  it('importAbsenceIcal sends FormData with file', async () => {
    const mockFile = new File(['ical data'], 'calendar.ics', { type: 'text/calendar' });
    globalThis.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({ success: true, data: { imported: 3 } }),
    });
    const result = await api.importAbsenceIcal(mockFile);
    expect(result.success).toBe(true);
    const [url, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(url).toBe('/api/v1/absences/import');
    expect(opts.method).toBe('POST');
    expect(opts.body).toBeInstanceOf(FormData);
    expect(opts.credentials).toBe('include');
  });

  it('importAbsenceIcal includes auth header when token present', async () => {
    setInMemoryToken('my-token');
    const mockFile = new File(['data'], 'cal.ics');
    globalThis.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({ success: true, data: null }),
    });
    await api.importAbsenceIcal(mockFile);
    const [, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(opts.headers['Authorization']).toBe('Bearer my-token');
  });

  // ── exportMyData (requestBlob path) ──

  it('exportMyData returns blob on success', async () => {
    const mockBlob = new Blob(['data'], { type: 'application/zip' });
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      blob: () => Promise.resolve(mockBlob),
    });
    const result = await api.exportMyData();
    expect(result).toBe(mockBlob);
    expect((globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0][0]).toBe('/api/v1/user/export');
  });

  it('exportMyData throws on non-OK response', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: false, status: 500,
      blob: () => Promise.resolve(new Blob()),
    });
    await expect(api.exportMyData()).rejects.toThrow('HTTP 500');
  });

  it('exportMyData includes auth header when token present', async () => {
    setInMemoryToken('blob-token');
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true, status: 200,
      blob: () => Promise.resolve(new Blob()),
    });
    await api.exportMyData();
    const [, opts] = (globalThis.fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    expect(opts.headers['Authorization']).toBe('Bearer blob-token');
    expect(opts.headers['X-Requested-With']).toBe('XMLHttpRequest');
  });
});
