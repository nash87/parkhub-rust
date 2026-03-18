const BASE_URL = (import.meta as any).env?.VITE_API_URL || '';

export interface ApiResponse<T> {
  success: boolean;
  data: T | null;
  error?: { code: string; message: string };
}

async function request<T>(path: string, opts: RequestInit = {}): Promise<ApiResponse<T>> {
  const token = localStorage.getItem('parkhub_token');
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    Accept: 'application/json',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...(opts.headers as Record<string, string> || {}),
  };

  try {
    const res = await fetch(`${BASE_URL}${path}`, { ...opts, headers });

    if (res.status === 401) {
      localStorage.removeItem('parkhub_token');
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

// ── Auth ──
export const api = {
  login: (username: string, password: string) =>
    request<{ tokens: { access_token: string } }>('/api/v1/auth/login', {
      method: 'POST', body: JSON.stringify({ username, password }),
    }),

  register: (data: { username: string; email: string; password: string; name: string }) =>
    request('/api/v1/auth/register', { method: 'POST', body: JSON.stringify(data) }),

  me: () => request<User>('/api/v1/me'),

  updateMe: (data: Partial<User>) =>
    request<User>('/api/v1/me', { method: 'PUT', body: JSON.stringify(data) }),

  changePassword: (current_password: string, password: string, password_confirmation: string) =>
    request('/api/v1/users/me/password', {
      method: 'PUT', body: JSON.stringify({ current_password, password, password_confirmation }),
    }),

  // ── Setup ──
  getSetupStatus: () => request<SetupStatus>('/api/v1/setup/status'),
  completeSetup: (data: any) => request('/api/v1/setup/complete', { method: 'POST', body: JSON.stringify(data) }),

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

  // ── Bookings ──
  getBookings: () => request<Booking[]>('/api/v1/bookings'),
  createBooking: (data: any) => request<Booking>('/api/v1/bookings', { method: 'POST', body: JSON.stringify(data) }),
  cancelBooking: (id: string) => request<void>(`/api/v1/bookings/${id}`, { method: 'DELETE' }),

  // ── Vehicles ──
  getVehicles: () => request<Vehicle[]>('/api/v1/vehicles'),
  createVehicle: (data: any) => request<Vehicle>('/api/v1/vehicles', { method: 'POST', body: JSON.stringify(data) }),
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
    const token = localStorage.getItem('parkhub_token');
    const res = await fetch(`${BASE_URL}/api/v1/absences/import`, {
      method: 'POST', body: fd,
      headers: { ...(token ? { Authorization: `Bearer ${token}` } : {}) },
    });
    return res.json();
  },

  // ── Credits ──
  getUserCredits: () => request<UserCredits>('/api/v1/user/credits'),
  getUserStats: () => request<UserStats>('/api/v1/user/stats'),

  // ── Admin ──
  adminStats: () => request<AdminStats>('/api/v1/admin/stats'),
  adminUsers: () => request<User[]>('/api/v1/admin/users'),
  adminUpdateUser: (id: string, data: any) => request<User>(`/api/v1/admin/users/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
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
  getDemoStatus: () => request<DemoStatus>('/api/v1/demo/status'),
  voteDemoReset: () => request('/api/v1/demo/vote', { method: 'POST' }),
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
