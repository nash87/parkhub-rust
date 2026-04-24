const BASE_URL = import.meta.env?.VITE_API_URL || '';

// ──────────────────────────────────────────────────────────────────────
// Auto-generated imports from the Rust API source of truth (T-1941).
//
// Types re-exported from parkhub-web/src/generated/types/ are emitted by
// `cargo test --features gen-types -p parkhub-server --test ts_export` out
// of parkhub-common/src/{models,protocol}.rs. The `types-drift` CI job
// re-runs that command on every PR and fails if the committed TS drifts
// from the Rust source.
//
// Scope of this migration: closed enums + protocol-error shapes only. Most
// object types in client.ts still diverge structurally from the current
// Rust DB models (the server projects to different shapes in handlers),
// and migrating them requires matching API-handler-level DTOs on the Rust
// side first. Tracked as a follow-up to T-1941.
// ──────────────────────────────────────────────────────────────────────
import type { ApiError as GeneratedApiError } from '../generated/types/ApiError';
import type { SlotType as GeneratedSlotType } from '../generated/types/SlotType';
import type { SlotFeature as GeneratedSlotFeature } from '../generated/types/SlotFeature';
import type { LotStatus as GeneratedLotStatus } from '../generated/types/LotStatus';
import type { AbsenceType as GeneratedAbsenceType } from '../generated/types/AbsenceType';
import type { ProposalStatus as GeneratedProposalStatus } from '../generated/types/ProposalStatus';
import type { SwapRequestStatus as GeneratedSwapRequestStatus } from '../generated/types/SwapRequestStatus';
import type { AnnouncementSeverity as GeneratedAnnouncementSeverity } from '../generated/types/AnnouncementSeverity';
import type { VehicleType as GeneratedVehicleType } from '../generated/types/VehicleType';
import type { FuelType as GeneratedFuelType } from '../generated/types/FuelType';
import type { PaginatedResponse } from '../generated/types/PaginatedResponse';

export type { PaginatedResponse };

/**
 * Standard API response wrapper.
 *
 * Shape closely matches `parkhub_common::protocol::ApiResponse<T>` in
 * parkhub-common/src/protocol.rs. The Rust source also carries a `meta:
 * ResponseMeta | null` field (pagination envelope) which we model as
 * optional here so existing `request(...)` construction sites don't need
 * to pass `meta: null`. `error` mirrors the generated `ApiError`.
 */
export interface ApiResponse<T> {
  success: boolean;
  data: T | null;
  error?: GeneratedApiError;
  meta?: unknown;
}

/** Re-export generated closed-enum types under their short TS names. */
export type SlotType = GeneratedSlotType;
export type SlotFeature = GeneratedSlotFeature;
export type LotStatus = GeneratedLotStatus;
export type AbsenceType = GeneratedAbsenceType;
export type ProposalStatus = GeneratedProposalStatus;
export type SwapRequestStatus = GeneratedSwapRequestStatus;
export type AnnouncementSeverity = GeneratedAnnouncementSeverity;
export type VehicleType = GeneratedVehicleType;
export type FuelType = GeneratedFuelType;

export interface RequestOptions extends Omit<RequestInit, 'signal'> {
  signal?: AbortSignal;
  retries?: number;
}

// In-memory token storage (XSS-safe: not in localStorage).
// Used as fallback when httpOnly cookie is not available (API/mobile clients).
let _inMemoryToken: string | null = null;

export function setInMemoryToken(token: string | null) {
  _inMemoryToken = token;
}

export function getInMemoryToken(): string | null {
  return _inMemoryToken;
}

// GET request deduplication — concurrent identical GETs share one in-flight promise
const _inflightGets = new Map<string, Promise<ApiResponse<any>>>();

const MAX_RETRIES = 2;
const RETRY_BASE_MS = 300;

function isTransientError(status: number): boolean {
  return status === 502 || status === 503 || status === 504 || status === 429;
}

// Single-flight token refresh. All concurrent 401s share one refresh call
// so we don't stampede the refresh endpoint with dozens of parallel POSTs.
let _inflightRefresh: Promise<boolean> | null = null;

async function attemptTokenRefresh(): Promise<boolean> {
  if (_inflightRefresh) return _inflightRefresh;

  _inflightRefresh = (async () => {
    try {
      const token = _inMemoryToken;
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
        Accept: 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
      };
      const res = await fetch(`${BASE_URL}/api/v1/auth/refresh`, {
        method: 'POST',
        headers,
        credentials: 'include',
      });
      if (!res.ok) return false;
      const body = (await res.json().catch(() => null)) as
        | {
            success?: boolean;
            data?: {
              tokens?: { access_token?: string };
              access_token?: string;
            };
          }
        | null;
      const next =
        body?.data?.tokens?.access_token ?? body?.data?.access_token ?? null;
      if (next) {
        _inMemoryToken = next;
        return true;
      }
      return false;
    } catch {
      return false;
    } finally {
      _inflightRefresh = null;
    }
  })();

  return _inflightRefresh;
}

async function requestOnce<T>(path: string, opts: RequestInit): Promise<ApiResponse<T>> {
  const res = await fetch(`${BASE_URL}${path}`, opts);

  // 401 on /auth/login is a wrong-password, not a session expiration — don't
  // wipe auth state or dispatch the global unauth event mid-login form.
  // 401 on /auth/refresh means the refresh token is also expired; hard-fail
  // there so the outer loop doesn't try to refresh its own refresh.
  const isLoginPath = path.includes('/auth/login');
  const isRefreshPath = path.includes('/auth/refresh');
  if (res.status === 401 && !isLoginPath && !isRefreshPath) {
    return { success: false, data: null, error: { code: 'UNAUTHORIZED', message: 'Session expired' } };
  }

  const json = await res.json().catch(() => null);

  if (!res.ok) {
    return {
      success: false,
      data: null,
      error: json?.error || { code: `HTTP_${res.status}`, message: res.statusText },
    };
  }

  if (json && typeof json === 'object' && 'success' in json) {
    return json;
  }
  return { success: true, data: json as T };
}

async function request<T>(path: string, opts: RequestOptions = {}): Promise<ApiResponse<T>> {
  const { retries = MAX_RETRIES, signal, ...rest } = opts;
  const token = _inMemoryToken;
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    Accept: 'application/json',
    'X-Requested-With': 'XMLHttpRequest',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...(rest.headers as Record<string, string> || {}),
  };

  const fetchOpts: RequestInit = {
    ...rest,
    headers,
    credentials: 'include',
    ...(signal ? { signal } : {}),
  };

  const method = (rest.method || 'GET').toUpperCase();
  const isGet = method === 'GET';

  // Deduplicate concurrent identical GET requests
  if (isGet) {
    const existing = _inflightGets.get(path);
    if (existing) return existing as Promise<ApiResponse<T>>;
  }

  const isAuthFlowPath = path.includes('/auth/login') || path.includes('/auth/refresh');

  const execute = async (): Promise<ApiResponse<T>> => {
    let refreshedOnce = false;
    for (let attempt = 0; attempt <= retries; attempt++) {
      try {
        const result = await requestOnce<T>(path, fetchOpts);

        // Transparent token refresh on 401. Only attempt once per request
        // and only for non-auth paths. On success, rebuild fetchOpts with
        // the new token and retry the original call.
        if (
          !result.success &&
          result.error?.code === 'UNAUTHORIZED' &&
          !isAuthFlowPath &&
          !refreshedOnce
        ) {
          refreshedOnce = true;
          const refreshed = await attemptTokenRefresh();
          if (refreshed) {
            const refreshedToken = _inMemoryToken;
            if (refreshedToken) {
              (fetchOpts.headers as Record<string, string>).Authorization = `Bearer ${refreshedToken}`;
            }
            continue;
          }
          // Refresh failed — signal unauth normally.
          _inMemoryToken = null;
          window.dispatchEvent(new Event('auth:unauthorized'));
          return result;
        }

        if (!result.success && result.error) {
          const status = parseInt(result.error.code.replace('HTTP_', ''), 10);
          if (isTransientError(status) && attempt < retries) {
            await new Promise(r => setTimeout(r, RETRY_BASE_MS * 2 ** attempt));
            continue;
          }
        }

        return result;
      } catch (e) {
        if (e instanceof DOMException && e.name === 'AbortError') {
          return { success: false, data: null, error: { code: 'ABORTED', message: 'Request aborted' } };
        }
        if (attempt < retries) {
          await new Promise(r => setTimeout(r, RETRY_BASE_MS * 2 ** attempt));
          continue;
        }
        return { success: false, data: null, error: { code: 'NETWORK', message: 'Network error' } };
      }
    }
    return { success: false, data: null, error: { code: 'NETWORK', message: 'Network error' } };
  };

  const promise = execute();

  if (isGet) {
    _inflightGets.set(path, promise);
    promise.finally(() => _inflightGets.delete(path));
  }

  return promise;
}

async function requestBlob(path: string, signal?: AbortSignal): Promise<Blob> {
  const token = _inMemoryToken;
  const headers: Record<string, string> = {
    'X-Requested-With': 'XMLHttpRequest',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
  };
  const res = await fetch(`${BASE_URL}${path}`, { headers, credentials: 'include', ...(signal ? { signal } : {}) });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.blob();
}

// ── Auth ──
export const api = {
  login: (username: string, password: string) =>
    request<{ tokens: { access_token: string } }>('/api/v1/auth/login', {
      method: 'POST', body: JSON.stringify({ username, password }),
    }),

  register: (data: { name: string; email: string; password: string; password_confirmation: string }) =>
    request('/api/v1/auth/register', { method: 'POST', body: JSON.stringify(data) }),

  forgotPassword: (email: string) =>
    request('/api/v1/auth/forgot-password', { method: 'POST', body: JSON.stringify({ email }) }),

  resetPassword: (token: string, password: string) =>
    request('/api/v1/auth/reset-password', { method: 'POST', body: JSON.stringify({ token, password }) }),

  logout: () => request('/api/v1/auth/logout', { method: 'POST' }),

  me: () => request<User>('/api/v1/users/me'),

  updateMe: (data: Partial<User>) =>
    request<User>('/api/v1/users/me', { method: 'PUT', body: JSON.stringify(data) }),

  setAccessibilityNeeds: (accessibility_needs: string) =>
    request<User>('/api/v1/users/me/accessibility-needs', {
      method: 'PUT',
      body: JSON.stringify({ accessibility_needs }),
    }),

  getBookingRecommendations: () =>
    request<Array<{
      slot_id: string;
      slot_number: number;
      lot_id: string;
      lot_name: string;
      floor_name: string;
      score: number;
      reasons: string[];
      reason_badges: string[];
    }>>('/api/v1/bookings/recommendations'),

  changePassword: (current_password: string, password: string, password_confirmation: string) =>
    request('/api/v1/users/me/password', {
      method: 'PUT', body: JSON.stringify({ current_password, password, password_confirmation }),
    }),

  exportMyData: () => requestBlob('/api/v1/user/export'),

  deleteMyAccount: () =>
    request('/api/v1/users/me/delete', { method: 'DELETE' }),

  // ── Setup ──
  getSetupStatus: () => request<SetupStatus>('/api/v1/setup/status'),
  completeSetup: (data: SetupPayload) => request('/api/v1/setup/complete', { method: 'POST', body: JSON.stringify(data) }),

  // ── Lots ──
  getLots: () => request<ParkingLot[]>('/api/v1/lots'),
  getLot: (id: string) => request<ParkingLot>(`/api/v1/lots/${id}`),
  getLotSlots: (lotId: string) => request<ParkingSlot[]>(`/api/v1/lots/${lotId}/slots`),
  createLot: (data: CreateLotRequest) =>
    request<ParkingLot>('/api/v1/lots', { method: 'POST', body: JSON.stringify(data) }),
  updateLot: (id: string, data: UpdateLotRequest) =>
    request<ParkingLot>(`/api/v1/lots/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  deleteLot: (id: string) =>
    request<void>(`/api/v1/lots/${id}`, { method: 'DELETE' }),

  // ── Dynamic Pricing ──
  getDynamicPrice: (lotId: string) =>
    request<DynamicPriceResult>(`/api/v1/lots/${lotId}/pricing/dynamic`),
  getAdminDynamicPricing: (lotId: string) =>
    request<DynamicPricingRules>(`/api/v1/admin/lots/${lotId}/pricing/dynamic`),
  updateAdminDynamicPricing: (lotId: string, data: Partial<DynamicPricingRules>) =>
    request<DynamicPricingRules>(`/api/v1/admin/lots/${lotId}/pricing/dynamic`, {
      method: 'PUT', body: JSON.stringify(data),
    }),

  // ── Operating Hours ──
  getLotHours: (lotId: string) =>
    request<OperatingHoursResponse>(`/api/v1/lots/${lotId}/hours`),
  updateAdminLotHours: (lotId: string, data: OperatingHoursData) =>
    request<OperatingHoursResponse>(`/api/v1/admin/lots/${lotId}/hours`, {
      method: 'PUT', body: JSON.stringify(data),
    }),

  // ── Bookings ──
  getBookings: () => request<Booking[]>('/api/v1/bookings'),
  createBooking: (data: CreateBookingPayload) => request<Booking>('/api/v1/bookings', { method: 'POST', body: JSON.stringify(data) }),
  cancelBooking: (id: string) => request<void>(`/api/v1/bookings/${id}`, { method: 'DELETE' }),

  // ── Vehicles ──
  getVehicles: () => request<Vehicle[]>('/api/v1/vehicles'),
  createVehicle: (data: CreateVehiclePayload) => request<Vehicle>('/api/v1/vehicles', { method: 'POST', body: JSON.stringify(data) }),
  deleteVehicle: (id: string) => request<void>(`/api/v1/vehicles/${id}`, { method: 'DELETE' }),

  // ── Absences ──
  listAbsences: () => request<AbsenceEntry[]>('/api/v1/absences'),
  createAbsence: (type: string, start: string, end: string, note?: string) =>
    request<AbsenceEntry>('/api/v1/absences', { method: 'POST', body: JSON.stringify({ absence_type: type, start_date: start, end_date: end, note }) }),
  deleteAbsence: (id: string) => request<void>(`/api/v1/absences/${id}`, { method: 'DELETE' }),
  teamAbsences: () => request<TeamAbsenceEntry[]>('/api/v1/absences/team'),
  getAbsencePattern: () => request<AbsencePattern[]>('/api/v1/absences/pattern'),
  setAbsencePattern: (type: string, weekdays: number[]) =>
    request<AbsencePattern>('/api/v1/absences/pattern', { method: 'POST', body: JSON.stringify({ absence_type: type, weekdays }) }),
  importAbsenceIcal: async (file: File) => {
    const fd = new FormData();
    fd.append('file', file);
    const token = _inMemoryToken;
    const res = await fetch(`${BASE_URL}/api/v1/absences/import`, {
      method: 'POST', body: fd,
      credentials: 'include',
      headers: {
        'X-Requested-With': 'XMLHttpRequest',
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
      },
    });
    return res.json();
  },

  // ── Credits ──
  getUserCredits: () => request<UserCredits>('/api/v1/user/credits'),
  getUserStats: () => request<UserStats>('/api/v1/user/stats'),

  // ── CO2 summary (T-1715) ──
  getCo2Summary: (from?: string, to?: string, lotId?: string) => {
    const params = new URLSearchParams();
    if (from) params.set('from', from);
    if (to) params.set('to', to);
    if (lotId) params.set('lot_id', lotId);
    const q = params.toString();
    return request<Co2Summary>(`/api/v1/bookings/co2-summary${q ? `?${q}` : ''}`);
  },

  // ── Admin ──
  adminStats: () => request<AdminStats>('/api/v1/admin/stats'),
  adminUsers: () => request<PaginatedResponse<User>>('/api/v1/admin/users'),
  adminUpdateUser: (id: string, data: UpdateUserPayload) => request<User>(`/api/v1/admin/users/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  adminDeleteUser: (id: string) => request<void>(`/api/v1/admin/users/${id}`, { method: 'DELETE' }),
  adminUpdateUserRole: (id: string, role: string) =>
    request<User>(`/api/v1/admin/users/${id}/role`, { method: 'PATCH', body: JSON.stringify({ role }) }),
  adminGrantCredits: (userId: string, amount: number, description?: string) =>
    request('/api/v1/admin/users/' + userId + '/credits', { method: 'POST', body: JSON.stringify({ amount, description }) }),
  adminRefillAll: (amount?: number) =>
    request('/api/v1/admin/credits/refill-all', { method: 'POST', body: JSON.stringify(amount ? { amount } : {}) }),
  adminUpdateUserQuota: (userId: string, monthlyQuota: number) =>
    request<User>(`/api/v1/admin/users/${userId}/quota`, { method: 'PUT', body: JSON.stringify({ monthly_quota: monthlyQuota }) }),
  adminGetSettings: () => request<Record<string, string>>('/api/v1/admin/settings'),
  adminUpdateSettings: (data: Record<string, string>) =>
    request('/api/v1/admin/settings', { method: 'PUT', body: JSON.stringify(data) }),

  // ── Admin Cost-Center Billing (parkhub-server/src/api/billing.rs) ──
  // Admin-only aggregates for tenant-wide finance reporting. The v5 Billing
  // screen surfaces these instead of the personal `/payments/history` feed.
  adminBillingByCostCenter: () =>
    request<AdminCostCenterSummary[]>('/api/v1/admin/billing/by-cost-center'),
  adminBillingByDepartment: () =>
    request<AdminDepartmentSummary[]>('/api/v1/admin/billing/by-department'),

  // ── Admin Modules (T-1720 v2 — runtime enable/disable) ──
  patchModule: (name: string, runtime_enabled: boolean) =>
    request<ModuleInfo>(`/api/v1/admin/modules/${encodeURIComponent(name)}`, {
      method: 'PATCH',
      body: JSON.stringify({ runtime_enabled }),
    }),

  // ── Admin Module Config (T-1720 v3 — per-module JSON Schema editor) ──
  getModuleConfig: (name: string) =>
    request<ModuleConfigResponse>(
      `/api/v1/admin/modules/${encodeURIComponent(name)}/config`,
    ),
  patchModuleConfig: (name: string, values: Record<string, unknown>) =>
    request<ModuleConfigResponse>(
      `/api/v1/admin/modules/${encodeURIComponent(name)}/config`,
      {
        method: 'PATCH',
        body: JSON.stringify(values),
      },
    ),

  // ── Admin Announcements ──
  adminListAnnouncements: () => request<Announcement[]>('/api/v1/admin/announcements'),
  adminCreateAnnouncement: (data: { title: string; message: string; severity: string; active: boolean; expires_at?: string }) =>
    request<Announcement>('/api/v1/admin/announcements', { method: 'POST', body: JSON.stringify(data) }),
  adminUpdateAnnouncement: (id: string, data: { title: string; message: string; severity: string; active: boolean; expires_at?: string }) =>
    request<Announcement>(`/api/v1/admin/announcements/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  adminDeleteAnnouncement: (id: string) =>
    request<void>(`/api/v1/admin/announcements/${id}`, { method: 'DELETE' }),

  // ── Notifications ──
  getNotifications: () => request<Notification[]>('/api/v1/notifications'),
  markNotificationRead: (id: string) => request<void>(`/api/v1/notifications/${id}/read`, { method: 'POST' }),
  markAllNotificationsRead: () => request<void>('/api/v1/notifications/read-all', { method: 'POST' }),

  // ── Notification Center (enriched feed used by the bell dropdown) ──
  getNotificationUnreadCount: () =>
    request<{ count: number }>('/api/v1/notifications/unread-count'),
  getNotificationCenter: (filter: 'all' | 'unread' | 'read' = 'all', perPage = 50) =>
    request<NotificationCenterPage>(
      `/api/v1/notifications/center?filter=${filter}&per_page=${perPage}`,
    ),
  markAllNotificationCenterRead: () =>
    request<void>('/api/v1/notifications/center/read-all', { method: 'PUT' }),
  markNotificationCenterRead: (id: string) =>
    request<void>(`/api/v1/notifications/${id}/read`, { method: 'PUT' }),
  deleteNotificationCenter: (id: string) =>
    request<void>(`/api/v1/notifications/center/${id}`, { method: 'DELETE' }),

  // ── Calendar ──
  // Backend (both rust `CalendarQuery` and PHP `BookingCalendarController::calendarEvents`)
  // reads `from`/`to`. Sending `start`/`end` silently skips the filter → unbounded result set.
  calendarEvents: (from: string, to: string) => {
    const params = new URLSearchParams();
    params.set('from', from);
    params.set('to', to);
    return request<CalendarEvent[]>(`/api/v1/calendar/events?${params.toString()}`);
  },
  generateCalendarToken: () =>
    request<{ token: string; url: string }>('/api/v1/calendar/token', { method: 'POST' }),

  // ── Demo ──
  getDemoConfig: () => request<{ demo_mode: boolean }>('/api/v1/demo/config'),
  getDemoStatus: async (): Promise<ApiResponse<DemoStatus>> => {
    const res = await request<DemoStatusRaw>('/api/v1/demo/status');
    if (res.success && res.data) {
      return { ...res, data: normalizeDemoStatus(res.data) };
    }
    return res as ApiResponse<DemoStatus>;
  },
  voteDemoReset: () => request('/api/v1/demo/vote', { method: 'POST' }),

  // ── Translations ──
  getTranslationOverrides: () =>
    request<TranslationOverride[]>('/api/v1/translations/overrides'),

  getTranslationProposals: (status?: ProposalStatus) =>
    request<TranslationProposal[]>(`/api/v1/translations/proposals${status ? `?status=${status}` : ''}`),

  getTranslationProposal: (id: string) =>
    request<TranslationProposal>(`/api/v1/translations/proposals/${id}`),

  createTranslationProposal: (data: CreateProposalRequest) =>
    request<TranslationProposal>('/api/v1/translations/proposals', {
      method: 'POST', body: JSON.stringify(data),
    }),

  voteOnProposal: (id: string, vote: 'up' | 'down') =>
    request<TranslationProposal>(`/api/v1/translations/proposals/${id}/vote`, {
      method: 'POST', body: JSON.stringify({ vote }),
    }),

  reviewProposal: (id: string, data: ReviewProposalRequest) =>
    request<TranslationProposal>(`/api/v1/translations/proposals/${id}/review`, {
      method: 'PUT', body: JSON.stringify(data),
    }),

  // ── Favorites ──
  getFavorites: () => request<Favorite[]>('/api/v1/user/favorites'),
  addFavorite: (slot_id: string, lot_id: string) =>
    request<Favorite>('/api/v1/user/favorites', { method: 'POST', body: JSON.stringify({ slot_id, lot_id }) }),
  removeFavorite: (slotId: string) =>
    request<void>(`/api/v1/user/favorites/${slotId}`, { method: 'DELETE' }),

  // ── 2FA ──
  setup2FA: () => request<TwoFactorSetup>('/api/v1/auth/2fa/setup', { method: 'POST' }),
  verify2FA: (code: string) => request<{ enabled: boolean }>('/api/v1/auth/2fa/verify', { method: 'POST', body: JSON.stringify({ code }) }),
  disable2FA: (current_password: string) => request<{ enabled: boolean }>('/api/v1/auth/2fa/disable', { method: 'POST', body: JSON.stringify({ current_password }) }),
  get2FAStatus: () => request<{ enabled: boolean }>('/api/v1/auth/2fa/status'),

  // ── Login History ──
  getLoginHistory: () => request<LoginHistoryEntry[]>('/api/v1/auth/login-history'),

  // ── Sessions ──
  getSessions: () => request<SessionInfo[]>('/api/v1/auth/sessions'),
  revokeSession: (id: string) => request<void>(`/api/v1/auth/sessions/${id}`, { method: 'DELETE' }),

  // ── Notification Preferences ──
  getNotificationPreferences: () => request<NotificationPreferences>('/api/v1/preferences/notifications'),
  updateNotificationPreferences: (prefs: NotificationPreferences) =>
    request<NotificationPreferences>('/api/v1/preferences/notifications', { method: 'PUT', body: JSON.stringify(prefs) }),

  // ── Design Theme Preferences ──
  getDesignThemePreference: () => request<{ design_theme: string }>('/api/v1/preferences/theme'),
  updateDesignThemePreference: (design_theme: string) =>
    request<{ design_theme: string }>('/api/v1/preferences/theme', { method: 'PUT', body: JSON.stringify({ design_theme }) }),

  // ── Bulk Admin ──
  adminBulkUpdate: (user_ids: string[], action: string, role?: string) =>
    request<BulkResult>('/api/v1/admin/users/bulk-update', { method: 'POST', body: JSON.stringify({ user_ids, action, role }) }),
  adminBulkDelete: (user_ids: string[]) =>
    request<BulkResult>('/api/v1/admin/users/bulk-delete', { method: 'POST', body: JSON.stringify({ user_ids }) }),

  // ── Map ──
  getMapMarkers: () => request<LotMarker[]>('/api/v1/lots/map'),
  setLotLocation: (lotId: string, latitude: number, longitude: number) =>
    request<void>(`/api/v1/admin/lots/${lotId}/location`, {
      method: 'PUT', body: JSON.stringify({ latitude, longitude }),
    }),

  // ── Stripe ──
  createCheckout: (credits: number, pricePerCredit?: number) =>
    request<CheckoutResponse>('/api/v1/payments/create-checkout', {
      method: 'POST', body: JSON.stringify({ credits, price_per_credit: pricePerCredit }),
    }),
  getPaymentHistory: () => request<PaymentHistoryEntry[]>('/api/v1/payments/history'),
  getStripeConfig: () => request<StripeConfigResponse>('/api/v1/payments/config'),

  // ── Rate Limits ──
  getRateLimitStats: () => request<RateLimitStats>('/api/v1/admin/rate-limits'),
  getRateLimitHistory: () => request<RateLimitHistory>('/api/v1/admin/rate-limits/history'),

  // ── Audit Log ──
  getAuditLog: (params?: { page?: number; per_page?: number; action?: string; user?: string; from?: string; to?: string }) => {
    const qs = new URLSearchParams();
    if (params?.page) qs.set('page', String(params.page));
    if (params?.per_page) qs.set('per_page', String(params.per_page));
    if (params?.action) qs.set('action', params.action);
    if (params?.user) qs.set('user', params.user);
    if (params?.from) qs.set('from', params.from);
    if (params?.to) qs.set('to', params.to);
    const q = qs.toString();
    return request<PaginatedAuditLog>(`/api/v1/admin/audit-log${q ? `?${q}` : ''}`);
  },
  exportAuditLog: (params?: { action?: string; user?: string; from?: string; to?: string }) => {
    const qs = new URLSearchParams();
    if (params?.action) qs.set('action', params.action);
    if (params?.user) qs.set('user', params.user);
    if (params?.from) qs.set('from', params.from);
    if (params?.to) qs.set('to', params.to);
    const q = qs.toString();
    return `/api/v1/admin/audit-log/export${q ? `?${q}` : ''}`;
  },

  // ── Tenants ──
  listTenants: () => request<TenantInfo[]>('/api/v1/admin/tenants'),
  createTenant: (data: CreateTenantRequest) =>
    request<TenantInfo>('/api/v1/admin/tenants', { method: 'POST', body: JSON.stringify(data) }),
  updateTenant: (id: string, data: CreateTenantRequest) =>
    request<TenantInfo>(`/api/v1/admin/tenants/${id}`, { method: 'PUT', body: JSON.stringify(data) }),

  // ── Parking History ──
  getBookingHistory: (params?: { lot_id?: string; from?: string; to?: string; page?: number; per_page?: number }) => {
    const qs = new URLSearchParams();
    if (params?.lot_id) qs.set('lot_id', params.lot_id);
    if (params?.from) qs.set('from', params.from);
    if (params?.to) qs.set('to', params.to);
    if (params?.page) qs.set('page', String(params.page));
    if (params?.per_page) qs.set('per_page', String(params.per_page));
    const q = qs.toString();
    return request<BookingHistoryResponse>(`/api/v1/bookings/history${q ? `?${q}` : ''}`);
  },
  getBookingStats: () => request<PersonalParkingStats>('/api/v1/bookings/stats'),

  // ── Geofencing ──
  geofenceCheckIn: (latitude: number, longitude: number) =>
    request<GeofenceCheckInResponse>('/api/v1/geofence/check-in', {
      method: 'POST', body: JSON.stringify({ latitude, longitude }),
    }),
  getLotGeofence: (lotId: string) =>
    request<GeofenceConfig>(`/api/v1/lots/${lotId}/geofence`),
  adminSetGeofence: (lotId: string, data: { center_lat: number; center_lng: number; radius_meters: number; enabled?: boolean }) =>
    request<GeofenceConfig>(`/api/v1/admin/lots/${lotId}/geofence`, {
      method: 'PUT', body: JSON.stringify(data),
    }),

  // ── Absence Approval ──
  submitAbsenceRequest: (data: { absence_type: string; start_date: string; end_date: string; reason: string }) =>
    request<AbsenceApprovalRequest>('/api/v1/absences/requests', { method: 'POST', body: JSON.stringify(data) }),
  myAbsenceRequests: () => request<AbsenceApprovalRequest[]>('/api/v1/absences/my'),
  pendingAbsenceRequests: () => request<AbsenceApprovalRequest[]>('/api/v1/admin/absences/pending'),
  approveAbsenceRequest: (id: string, comment?: string) =>
    request<AbsenceApprovalRequest>(`/api/v1/admin/absences/${id}/approve`, {
      method: 'PUT', body: JSON.stringify({ comment }),
    }),
  rejectAbsenceRequest: (id: string, reason: string) =>
    request<AbsenceApprovalRequest>(`/api/v1/admin/absences/${id}/reject`, {
      method: 'PUT', body: JSON.stringify({ reason }),
    }),

  // ── Calendar Drag Reschedule ──
  rescheduleBooking: (id: string, newStart: string, newEnd: string) =>
    request<RescheduleResponse>(`/api/v1/bookings/${id}/reschedule`, {
      method: 'PUT', body: JSON.stringify({ new_start: newStart, new_end: newEnd }),
    }),

  // ── Admin Widgets ──
  getWidgetLayout: () => request<WidgetLayoutResponse>('/api/v1/admin/widgets'),
  saveWidgetLayout: (widgets: WidgetEntryData[]) =>
    request<WidgetLayoutResponse>('/api/v1/admin/widgets', {
      method: 'PUT', body: JSON.stringify({ widgets }),
    }),
  getWidgetData: (widgetId: string) =>
    request<WidgetDataResponse>(`/api/v1/admin/widgets/data/${widgetId}`),

  // ── Team / Fleet (v5 Wave 3) ──
  getTeam: () => request<TeamMember[]>('/api/v1/team'),
  getAdminStatsExtended: () => request<AdminStatsExtended>('/api/v1/admin/stats'),

  // ── EV Chargers (v5 Wave 3) ──
  getLotChargers: (lotId: string) => request<EvCharger[]>(`/api/v1/lots/${lotId}/chargers`),
  getChargerSessions: () => request<ChargingSession[]>('/api/v1/chargers/sessions'),
  startCharging: (chargerId: string) =>
    request<void>(`/api/v1/chargers/${chargerId}/start`, { method: 'POST', body: JSON.stringify({}) }),
  stopCharging: (chargerId: string) =>
    request<void>(`/api/v1/chargers/${chargerId}/stop`, { method: 'POST' }),

  // ── Swap Requests (v5 Wave 3) ──
  // Rust backend uses `PUT /api/v1/swap-requests/{id}` with `{action}` body,
  // not separate POST /accept and /decline endpoints like PHP. The client
  // method signatures stay identical so the screens remain byte-identical
  // with the PHP mirror — only the URL shape below diverges.
  getSwapRequests: () => request<SwapRequest[]>('/api/v1/swap-requests'),
  acceptSwap: (id: string) =>
    request<void>(`/api/v1/swap-requests/${id}`, {
      method: 'PUT',
      body: JSON.stringify({ action: 'accept' }),
    }),
  declineSwap: (id: string) =>
    request<void>(`/api/v1/swap-requests/${id}`, {
      method: 'PUT',
      body: JSON.stringify({ action: 'decline' }),
    }),
  createSwapRequest: (sourceBookingId: string, targetBookingId: string, message: string | null) =>
    request<SwapRequest>(`/api/v1/bookings/${sourceBookingId}/swap-request`, {
      method: 'POST',
      body: JSON.stringify({ target_booking_id: targetBookingId, message }),
    }),

  // ── Check-in (v5 Wave 3) ──
  // Rust backend exposes `POST /api/v1/bookings/{id}/checkin` (no dash);
  // the separate `GET …/check-in` status probe and `POST …/check-out` are
  // PHP-only surfaces that still resolve via the 404 fallback path in the
  // screen. Method signatures stay identical for parity.
  getCheckInStatus: (bookingId: string) =>
    request<CheckInStatus>(`/api/v1/bookings/${bookingId}/check-in`),
  checkIn: (bookingId: string) =>
    request<void>(`/api/v1/bookings/${bookingId}/checkin`, { method: 'POST' }),
  checkOut: (bookingId: string) =>
    request<void>(`/api/v1/bookings/${bookingId}/check-out`, { method: 'POST' }),

  // ── Guest Passes (v5 Wave 3) ──
  // `DELETE …/bookings/guest/{id}` is PHP-only today; rust ships only the
  // admin PATCH cancel. We keep the user-side signature for parity and let
  // the 404 surface as a toast until the rust handler lands.
  getGuestBookings: () => request<GuestBooking[]>('/api/v1/bookings/guest'),
  createGuestBooking: (data: CreateGuestBookingPayload) =>
    request<GuestBooking>('/api/v1/bookings/guest', { method: 'POST', body: JSON.stringify(data) }),
  cancelGuestBooking: (id: string) =>
    request<void>(`/api/v1/bookings/guest/${id}`, { method: 'DELETE' }),

  // ── API Keys (v5 Wave 4) ──
  getApiKeys: () => request<ApiKey[]>('/api/v1/admin/api-keys'),
  createApiKey: (label: string) =>
    request<CreatedApiKey>('/api/v1/admin/api-keys', {
      method: 'POST', body: JSON.stringify({ label }),
    }),
  rotateApiKey: (id: string) =>
    request<CreatedApiKey>(`/api/v1/admin/api-keys/${id}/rotate`, { method: 'POST' }),
  revokeApiKey: (id: string) =>
    request<void>(`/api/v1/admin/api-keys/${id}`, { method: 'DELETE' }),

  // ── Integrations (v5 Wave 4) ──
  getIntegrations: () => request<Integration[]>('/api/v1/admin/integrations'),
  connectIntegration: (id: string) =>
    request<Integration>(`/api/v1/admin/integrations/${id}/connect`, { method: 'POST' }),
  disconnectIntegration: (id: string) =>
    request<Integration>(`/api/v1/admin/integrations/${id}/disconnect`, { method: 'POST' }),

  // ── Policies (v5 Wave 4) ──
  getPolicies: () => request<Policy[]>('/api/v1/admin/policies'),
  updatePolicy: (id: string, body: string) =>
    request<Policy>(`/api/v1/admin/policies/${id}`, {
      method: 'PUT', body: JSON.stringify({ body }),
    }),

  // ── Lobby Display (v5 Wave 4) ──
  getLobbyConfig: () => request<LobbyConfig>('/api/v1/admin/lobby'),
  updateLobbyConfig: (data: Partial<LobbyConfig>) =>
    request<LobbyConfig>('/api/v1/admin/lobby', {
      method: 'PUT', body: JSON.stringify(data),
    }),
};

// ── API Keys ──
export interface ApiKey {
  id: string;
  label: string;
  masked_key: string;
  last_used_at: string | null;
  created_at: string;
}

export interface CreatedApiKey extends ApiKey {
  /** Full key — returned exactly once on create/rotate. */
  token: string;
}

// ── Integrations ──
export interface Integration {
  id: string;
  name: string;
  provider: string;
  description: string;
  connected: boolean;
  connected_at: string | null;
  account_label: string | null;
}

// ── Policies ──
export interface Policy {
  id: string;
  title: string;
  slug: string;
  body: string;
  updated_at: string;
}

// ── Lobby Display ──
export type LobbyScreenKey = 'queue' | 'map' | 'announcements' | 'welcome';

export interface LobbyConfig {
  active_screen: LobbyScreenKey;
  rotate_interval_seconds: number;
  show_clock: boolean;
  show_weather: boolean;
}

// ── Types ──

/** Module registry entry returned by /api/v1/modules/info and /api/v1/admin/modules/{name}. */
export interface ModuleInfo {
  name: string;
  category: string;
  description: string;
  enabled: boolean;
  runtime_toggleable: boolean;
  runtime_enabled?: boolean;
  config_keys: string[];
  /** JSON Schema for the module's PATCH /config endpoint, when the module ships one (T-1720 v3). */
  config_schema?: unknown;
  ui_route: string | null;
  depends_on: string[];
  version: string;
}

/** Response body of `GET/PATCH /api/v1/admin/modules/{name}/config` (T-1720 v3). */
export interface ModuleConfigResponse {
  schema: {
    type: 'object';
    properties: Record<string, unknown>;
    required?: string[];
    title?: string;
    description?: string;
  };
  values: Record<string, unknown>;
}

export interface User {
  id: string;
  username: string;
  email: string;
  name: string;
  picture?: string;
  phone?: string;
  role: 'user' | 'premium' | 'admin' | 'superadmin';
  preferences: Record<string, any>;
  is_active: boolean;
  department?: string;
  credits_balance: number;
  credits_monthly_quota: number;
  created_at?: string;
  last_login?: string;
}

export interface SetupStatus {
  setup_complete: boolean;
  has_admin: boolean;
  needs_password_change?: boolean;
}

export interface ParkingLot {
  id: string;
  name: string;
  address?: string;
  total_slots: number;
  available_slots: number;
  status: string;
  hourly_rate?: number;
  daily_max?: number;
  monthly_pass?: number;
  currency?: string;
  operating_hours?: OperatingHoursData;
}

export type MarkerColor = 'green' | 'yellow' | 'red' | 'gray';

export interface LotMarker {
  id: string;
  name: string;
  address: string;
  latitude: number;
  longitude: number;
  available_slots: number;
  total_slots: number;
  status: string;
  color: MarkerColor;
}

export interface DynamicPricingRules {
  enabled: boolean;
  base_price: number;
  surge_multiplier: number;
  discount_multiplier: number;
  surge_threshold: number;
  discount_threshold: number;
}

export interface DynamicPriceResult {
  current_price: number;
  base_price: number;
  applied_multiplier: number;
  occupancy_percent: number;
  dynamic_pricing_active: boolean;
  tier: 'surge' | 'discount' | 'normal';
  currency: string;
}

export interface DayHoursData {
  open: string;
  close: string;
  closed: boolean;
}

export interface OperatingHoursData {
  is_24h: boolean;
  monday?: DayHoursData;
  tuesday?: DayHoursData;
  wednesday?: DayHoursData;
  thursday?: DayHoursData;
  friday?: DayHoursData;
  saturday?: DayHoursData;
  sunday?: DayHoursData;
}

export interface OperatingHoursResponse extends OperatingHoursData {
  is_open_now: boolean;
}

// `SlotType` and `SlotFeature` are re-exported from the generated types
// at the top of this file (T-1941). The hand-written definitions that
// used to live here were byte-for-byte identical to the generated ones;
// removing them prevents drift.

export interface ParkingSlot {
  id: string;
  lot_id: string;
  slot_number: string;
  status: string;
  slot_type?: SlotType;
  features?: SlotFeature[];
  zone_id?: string;
  is_accessible?: boolean;
}

export interface Favorite {
  user_id: string;
  slot_id: string;
  lot_id: string;
  created_at: string;
}

export interface Booking {
  id: string;
  user_id: string;
  lot_id: string;
  slot_id: string;
  lot_name: string;
  slot_number: string;
  vehicle_plate?: string;
  start_time: string;
  end_time: string;
  status: 'confirmed' | 'active' | 'completed' | 'cancelled';
  booking_type?: string;
  dauer_interval?: string;
  notes?: string;
  base_price?: number;
  tax_amount?: number;
  total_price?: number;
  currency?: string;
}

export interface Vehicle {
  id: string;
  plate: string;
  make?: string;
  model?: string;
  color?: string;
  is_default: boolean;
  photo_url?: string;
}

export interface AbsenceEntry {
  id: string;
  user_id: string;
  absence_type: string;
  start_date: string;
  end_date: string;
  note?: string;
  source: string;
  created_at: string;
}

export interface TeamAbsenceEntry {
  user_name: string;
  absence_type: string;
  start_date: string;
  end_date: string;
}

export interface AbsencePattern {
  user_id: string;
  absence_type: string;
  weekdays: number[];
}

export interface CreditTransaction {
  id: string;
  amount: number;
  type: 'grant' | 'deduction' | 'refund' | 'monthly_refill';
  description?: string;
  created_at: string;
}

export interface UserCredits {
  enabled: boolean;
  balance: number;
  monthly_quota: number;
  last_refilled?: string;
  transactions: CreditTransaction[];
}

export interface CheckoutResponse {
  id: string;
  checkout_url: string;
  amount: number;
  credits: number;
  currency: string;
}

export interface PaymentHistoryEntry {
  id: string;
  amount: number;
  credits: number;
  currency: string;
  status: 'pending' | 'completed' | 'expired' | 'failed';
  created_at: string;
  completed_at?: string;
}

export interface StripeConfigResponse {
  publishable_key?: string;
  configured: boolean;
}

/**
 * Admin cost-center billing aggregate — matches `CostCenterSummary` in
 * `parkhub-server/src/api/billing.rs`. Amount is in whole currency units
 * (f64 euros), not cents like PaymentHistoryEntry.
 */
export interface AdminCostCenterSummary {
  cost_center: string;
  department: string;
  user_count: number;
  total_bookings: number;
  total_credits_used: number;
  total_amount: number;
  currency: string;
}

/**
 * Admin department billing aggregate — matches `DepartmentSummary` in
 * `parkhub-server/src/api/billing.rs`.
 */
export interface AdminDepartmentSummary {
  department: string;
  user_count: number;
  total_bookings: number;
  total_credits_used: number;
  total_amount: number;
  currency: string;
}

export interface UserStats {
  total_bookings: number;
  bookings_this_month: number;
  homeoffice_days_this_month: number;
  avg_duration_minutes: number;
  favorite_slot?: string;
}

/** CO2 summary response from `/api/v1/bookings/co2-summary` (T-1715). */
export interface Co2Summary {
  from: string;
  to: string;
  bookings_counted: number;
  total_km: number;
  emitted_g: number;
  counterfactual_g: number;
  saved_g: number;
  carpool_saved_g: number;
  /** Server-rounded to 2 decimals; display directly. */
  saved_kg: number;
}

export interface AdminStats {
  total_users: number;
  total_lots: number;
  total_bookings: number;
  active_bookings: number;
}

/** Raw shape from the Rust API (nested objects) */
interface DemoStatusRaw {
  enabled?: boolean;
  timer?: { remaining: number; duration: number };
  timer_seconds?: number;
  votes?: { current: number; threshold: number; has_voted: boolean } | number;
  vote_threshold?: number;
  viewers: number;
  has_voted?: boolean;
  reset?: boolean;
  last_reset_at?: string;
  next_scheduled_reset?: string;
  reset_in_progress?: boolean;
}

/** Normalized shape used by components */
export interface DemoStatus {
  timer_seconds: number;
  votes: number;
  vote_threshold: number;
  viewers: number;
  has_voted: boolean;
  reset?: boolean;
  last_reset_at?: string;
  next_scheduled_reset?: string;
  reset_in_progress?: boolean;
}

/** Normalize Rust (nested) or PHP (flat) demo status into a consistent shape */
function normalizeDemoStatus(raw: DemoStatusRaw): DemoStatus {
  return {
    timer_seconds: raw.timer?.remaining ?? raw.timer_seconds ?? 0,
    votes: typeof raw.votes === 'object' ? raw.votes.current : (raw.votes ?? 0),
    vote_threshold: typeof raw.votes === 'object' ? raw.votes.threshold : (raw.vote_threshold ?? 3),
    has_voted: typeof raw.votes === 'object' ? raw.votes.has_voted : (raw.has_voted ?? false),
    viewers: raw.viewers ?? 0,
    reset: raw.reset,
    last_reset_at: raw.last_reset_at,
    next_scheduled_reset: raw.next_scheduled_reset,
    reset_in_progress: raw.reset_in_progress ?? false,
  };
}

export interface Notification {
  id: string;
  title: string;
  message: string;
  notification_type: string;
  read: boolean;
  created_at: string;
}

export interface CenterNotification {
  id: string;
  notification_type: string;
  title: string;
  message: string;
  read: boolean;
  action_url: string | null;
  icon: string;
  severity: string;
  type_label: string;
  created_at: string;
  date_group: string;
}

export interface NotificationCenterPage {
  items: CenterNotification[];
  total: number;
  page: number;
  per_page: number;
  unread_count: number;
}

export interface CalendarEvent {
  id: string;
  title: string;
  start: string;
  end: string;
  type: 'booking' | 'absence';
  status: string;
  lot_name?: string;
}

export interface Announcement {
  id: string;
  title: string;
  message: string;
  severity: string;
  active: boolean;
  expires_at?: string;
  created_at: string;
}

// `LotStatus` is re-exported from the generated types at the top of this
// file (T-1941).

export interface CreateLotRequest {
  name: string;
  address?: string;
  latitude?: number;
  longitude?: number;
  total_slots: number;
  hourly_rate?: number;
  daily_max?: number;
  monthly_pass?: number;
  currency?: string;
  status?: LotStatus;
}

export interface SetupPayload {
  password: string;
  password_confirmation: string;
  company_name?: string;
  use_case?: string;
}

export interface CreateBookingPayload {
  lot_id: string;
  slot_id: string;
  start_time: string;
  end_time: string;
  vehicle_id?: string;
}

export interface CreateVehiclePayload {
  plate: string;
  make?: string;
  model?: string;
  color?: string;
}

export interface UpdateUserPayload {
  name?: string;
  email?: string;
  role?: string;
  is_active?: boolean;
  department?: string;
}

export interface UpdateLotRequest {
  name?: string;
  address?: string;
  latitude?: number;
  longitude?: number;
  total_slots?: number;
  hourly_rate?: number;
  daily_max?: number;
  monthly_pass?: number;
  currency?: string;
  status?: LotStatus;
}

// ── Translation Management ──

// `ProposalStatus` is re-exported from the generated types at the top of
// this file (T-1941).

export interface TranslationProposal {
  id: string;
  language: string;
  key: string;
  current_value: string;
  proposed_value: string;
  context?: string;
  proposed_by: string;
  proposed_by_name: string;
  status: ProposalStatus;
  votes_for: number;
  votes_against: number;
  user_vote?: 'up' | 'down' | null;
  reviewer_id?: string;
  reviewer_name?: string;
  review_comment?: string;
  created_at: string;
  updated_at: string;
}

export interface TranslationOverride {
  language: string;
  key: string;
  value: string;
  updated_at: string;
}

export interface CreateProposalRequest {
  language: string;
  key: string;
  proposed_value: string;
  context?: string;
}

export interface ReviewProposalRequest {
  status: 'approved' | 'rejected';
  comment?: string;
}

// ── 2FA Types ──
export interface TwoFactorSetup {
  secret: string;
  otpauth_uri: string;
  qr_code_base64: string;
}

// ── Login History ──
export interface LoginHistoryEntry {
  timestamp: string;
  ip_address: string;
  user_agent: string;
  success: boolean;
}

// ── Session Info ──
export interface SessionInfo {
  id: string;
  username: string;
  role: string;
  created_at: string;
  expires_at: string;
  is_current: boolean;
}

// ── Notification Preferences ──
export interface NotificationPreferences {
  email_booking_confirm: boolean;
  email_booking_reminder: boolean;
  email_swap_request: boolean;
  push_enabled: boolean;
  sms_booking_confirm: boolean;
  sms_booking_reminder: boolean;
  sms_booking_cancelled: boolean;
  whatsapp_booking_confirm: boolean;
  whatsapp_booking_reminder: boolean;
  whatsapp_booking_cancelled: boolean;
  phone_number?: string;
}

// ── Bulk Result ──
export interface BulkResult {
  total: number;
  succeeded: number;
  failed: number;
  errors: string[];
}

// ── Rate Limits ──
export interface RateLimitGroup {
  group: string;
  limit_per_minute: number;
  description: string;
  current_count: number;
  reset_seconds: number;
  blocked_last_hour: number;
}

export interface RateLimitStats {
  groups: RateLimitGroup[];
  total_blocked_last_hour: number;
}

export interface RateLimitHistoryBin {
  hour: string;
  count: number;
}

export interface RateLimitHistory {
  bins: RateLimitHistoryBin[];
}

// ── Tenants ──
export interface TenantBranding {
  primary_color?: string;
  logo_url?: string;
  company_name?: string;
}

export interface TenantInfo {
  id: string;
  name: string;
  domain?: string;
  branding?: TenantBranding;
  created_at: string;
  updated_at: string;
  user_count: number;
  lot_count: number;
}

export interface CreateTenantRequest {
  name: string;
  domain?: string;
  branding?: TenantBranding;
}

// ── Audit Log ──
export interface AuditLogEntry {
  id: string;
  timestamp: string;
  event_type: string;
  user_id?: string;
  username?: string;
  target_type?: string;
  target_id?: string;
  ip_address?: string;
  details?: string;
}

export interface PaginatedAuditLog {
  entries: AuditLogEntry[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

// ── Parking History ──
export interface BookingHistoryResponse {
  items: Booking[];
  page: number;
  per_page: number;
  total: number;
  total_pages: number;
}

export interface MonthlyTrend {
  month: string;
  bookings: number;
}

export interface PersonalParkingStats {
  total_bookings: number;
  favorite_lot: string | null;
  avg_duration_minutes: number;
  busiest_day: string | null;
  credits_spent: number;
  monthly_trend: MonthlyTrend[];
}

// ── Geofencing ──
export interface GeofenceCheckInResponse {
  checked_in: boolean;
  booking_id: string | null;
  lot_name: string | null;
  message: string;
}

export interface GeofenceConfig {
  lot_id: string;
  center_lat: number;
  center_lng: number;
  radius_meters: number;
  enabled: boolean;
}

// ── Calendar Drag Reschedule ──
export interface RescheduleResponse {
  booking_id: string;
  old_start: string;
  old_end: string;
  new_start: string;
  new_end: string;
  slot_id: string;
  lot_id: string;
  success: boolean;
  message: string;
}

// ── Absence Approval ──
export interface AbsenceApprovalRequest {
  id: string;
  user_id: string;
  user_name: string;
  absence_type: string;
  start_date: string;
  end_date: string;
  reason: string;
  status: 'pending' | 'approved' | 'rejected';
  reviewer_id?: string;
  reviewer_comment?: string;
  created_at: string;
  reviewed_at?: string;
}

// ── Admin Widgets ──
export interface WidgetEntryData {
  id: string;
  widget_type: string;
  position: { x: number; y: number; w: number; h: number };
  visible: boolean;
}

export interface WidgetLayoutResponse {
  user_id: string;
  widgets: WidgetEntryData[];
}

export interface WidgetDataResponse {
  widget_id: string;
  widget_type: string;
  title: string;
  data: Record<string, unknown>;
}

// ── v5 Wave 3: Fleet ──

export interface TeamMember {
  id: string;
  username: string;
  name: string;
  role: string;
}

export interface UserBookingStats {
  total: number;
  this_month: number;
  ev_count: number;
  morning_count: number;
  swaps_accepted: number;
  no_shows: number;
  avg_duration_hours: number;
}

export interface DayOccupancy {
  avg_percentage: number;
  peak_hour: number;
  peak_percentage: number;
  bookings: number;
}

/**
 * Runtime-extended shape of `GET /api/v1/admin/stats`. The base
 * {@link AdminStats} interface only declares the guaranteed fields, while
 * the Wave 3 Fleet screens (Rangliste, Vorhersagen) consume optional
 * aggregates that the backend returns opportunistically.
 */
export interface AdminStatsExtended extends AdminStats {
  ev_bookings?: number;
  morning_bookings?: number;
  swap_requests_accepted?: number;
  no_shows?: number;
  bookings_by_user?: Record<string, UserBookingStats>;
  occupancy_by_day?: Record<string, DayOccupancy>;
  occupancy_by_hour?: Record<string, number>;
}

export type ChargerConnector = 'type2' | 'ccs' | 'chademo' | 'tesla';
export type ChargerStatus = 'available' | 'in_use' | 'offline' | 'maintenance';

export interface EvCharger {
  id: string;
  lot_id: string;
  label: string;
  connector_type: ChargerConnector;
  power_kw: number;
  status: ChargerStatus;
  location_hint: string | null;
}

export interface ChargingSession {
  id: string;
  charger_id: string;
  user_id: string;
  start_time: string;
  end_time: string | null;
  kwh_consumed: number;
  status: 'active' | 'completed' | 'cancelled';
}

export interface SwapBookingSummary {
  lot_name: string;
  slot_number: string;
  start_time: string;
  end_time: string;
}

export interface SwapRequest {
  id: string;
  requester_id: string;
  source_booking_id: string;
  target_booking_id: string;
  source_booking: SwapBookingSummary;
  target_booking: SwapBookingSummary;
  message: string | null;
  status: 'pending' | 'accepted' | 'declined';
  created_at: string;
}

export interface CheckInStatus {
  checked_in: boolean;
  checked_in_at: string | null;
  checked_out_at: string | null;
}

/**
 * Guest booking payload returned by `GET /api/v1/bookings/guest`.
 *
 * The `status` union mirrors the Rust backend's `BookingStatus` enum
 * (`parkhub-common::models::BookingStatus`) which is serialised as the full
 * snake_case set. In particular, newly created passes land in `confirmed`
 * (see `parkhub-server::api::guest::create_guest_booking`) and only flip to
 * `active` once the booking window opens, so clients MUST accept the full
 * set to render and act on fresh records.
 */
export interface GuestBooking {
  id: string;
  lot_id: string;
  lot_name: string;
  slot_id: string;
  slot_number: string;
  guest_name: string;
  guest_email: string | null;
  guest_code: string;
  start_time: string;
  end_time: string;
  status:
    | 'pending'
    | 'confirmed'
    | 'active'
    | 'completed'
    | 'expired'
    | 'cancelled'
    | 'no_show';
  created_at: string;
}

export interface CreateGuestBookingPayload {
  lot_id: string;
  slot_id: string;
  start_time: string;
  end_time: string;
  guest_name: string;
  guest_email: string | null;
}
