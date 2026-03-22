const BASE_URL = import.meta.env?.VITE_API_URL || '';

export interface ApiResponse<T> {
  success: boolean;
  data: T | null;
  error?: { code: string; message: string };
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

async function request<T>(path: string, opts: RequestInit = {}): Promise<ApiResponse<T>> {
  const token = _inMemoryToken;
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    Accept: 'application/json',
    // CSRF protection: required by backend for cookie-based auth
    'X-Requested-With': 'XMLHttpRequest',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...(opts.headers as Record<string, string> || {}),
  };

  try {
    const res = await fetch(`${BASE_URL}${path}`, {
      ...opts,
      headers,
      credentials: 'include',  // Send httpOnly cookies automatically
    });

    if (res.status === 401) {
      _inMemoryToken = null;
      window.location.href = '/login';
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

    // Normalize: API may return { success, data } or raw data
    if (json && typeof json === 'object' && 'success' in json) {
      return json;
    }
    return { success: true, data: json as T };
  } catch (e) {
    return { success: false, data: null, error: { code: 'NETWORK', message: 'Network error' } };
  }
}

async function requestBlob(path: string): Promise<Blob> {
  const token = _inMemoryToken;
  const headers: Record<string, string> = {
    'X-Requested-With': 'XMLHttpRequest',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
  };
  const res = await fetch(`${BASE_URL}${path}`, { headers, credentials: 'include' });
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

  // ── Admin ──
  adminStats: () => request<AdminStats>('/api/v1/admin/stats'),
  adminUsers: () => request<User[]>('/api/v1/admin/users'),
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

  // ── Calendar ──
  calendarEvents: (start: string, end: string) =>
    request<CalendarEvent[]>(`/api/v1/calendar/events?start=${start}&end=${end}`),

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
};

// ── Types ──
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

export type SlotType = 'standard' | 'compact' | 'large' | 'handicap' | 'electric' | 'motorcycle' | 'reserved' | 'vip';
export type SlotFeature = 'near_exit' | 'near_elevator' | 'near_stairs' | 'covered' | 'security_camera' | 'well_lit' | 'wide_lane' | 'charging_station';

export interface ParkingSlot {
  id: string;
  lot_id: string;
  slot_number: string;
  status: string;
  slot_type?: SlotType;
  features?: SlotFeature[];
  zone_id?: string;
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

export interface UserStats {
  total_bookings: number;
  bookings_this_month: number;
  homeoffice_days_this_month: number;
  avg_duration_minutes: number;
  favorite_slot?: string;
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

export type LotStatus = 'open' | 'closed' | 'full' | 'maintenance';

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

export type ProposalStatus = 'pending' | 'approved' | 'rejected';

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
}

// ── Bulk Result ──
export interface BulkResult {
  total: number;
  succeeded: number;
  failed: number;
  errors: string[];
}
