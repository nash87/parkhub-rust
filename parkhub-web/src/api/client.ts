/**
 * ParkHub API Client
 */

const API_BASE = import.meta.env.VITE_API_URL || '';

interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
  };
}

class ApiClient {
  private token: string | null = null;

  setToken(token: string | null) {
    this.token = token;
    if (token) {
      localStorage.setItem('parkhub_token', token);
    } else {
      localStorage.removeItem('parkhub_token');
    }
  }

  getToken(): string | null {
    if (!this.token) {
      this.token = localStorage.getItem('parkhub_token');
    }
    return this.token;
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<ApiResponse<T>> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...options.headers as Record<string, string>,
    };

    const token = this.getToken();
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }

    try {
      const response = await fetch(`${API_BASE}${endpoint}`, {
        ...options,
        headers,
      });

      const data = await response.json();
      return data;
    } catch (error) {
      return {
        success: false,
        error: {
          code: 'NETWORK_ERROR',
          message: error instanceof Error ? error.message : 'Network error',
        },
      };
    }
  }

  // Auth
  async login(username: string, password: string) {
    return this.request<{ user: User; tokens: AuthTokens }>('/api/v1/auth/login', {
      method: 'POST',
      body: JSON.stringify({ username, password }),
    });
  }

  async register(data: RegisterData) {
    return this.request<{ user: User; tokens: AuthTokens }>('/api/v1/auth/register', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async refreshToken(refreshToken: string) {
    return this.request<AuthTokens>('/api/v1/auth/refresh', {
      method: 'POST',
      body: JSON.stringify({ refresh_token: refreshToken }),
    });
  }

  async forgotPassword(email: string) {
    return this.request<void>('/api/v1/auth/forgot-password', {
      method: 'POST',
      body: JSON.stringify({ email }),
    });
  }

  async resetPassword(token: string, password: string) {
    return this.request<void>('/api/v1/auth/reset-password', {
      method: 'POST',
      body: JSON.stringify({ token, password }),
    });
  }

  // Users
  async getCurrentUser() {
    return this.request<User>('/api/v1/users/me');
  }

  async updateProfile(data: { name?: string; phone?: string; picture?: string }) {
    return this.request<User>('/api/v1/users/me', {
      method: 'PUT',
      body: JSON.stringify(data),
    });
  }

  // Lots
  async getLots() {
    return this.request<ParkingLot[]>('/api/v1/lots');
  }

  async getLot(id: string) {
    return this.request<ParkingLot>(`/api/v1/lots/${id}`);
  }

  async createLot(data: CreateLotData) {
    return this.request<ParkingLot>('/api/v1/lots', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async getLotSlots(lotId: string) {
    return this.request<ParkingSlot[]>(`/api/v1/lots/${lotId}/slots`);
  }

  // Bookings
  async getBookings() {
    return this.request<Booking[]>('/api/v1/bookings');
  }

  async createBooking(data: CreateBookingData) {
    return this.request<Booking>('/api/v1/bookings', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async cancelBooking(id: string) {
    return this.request<void>(`/api/v1/bookings/${id}`, {
      method: 'DELETE',
    });
  }

  // Vehicles
  async getVehicles() {
    return this.request<Vehicle[]>('/api/v1/vehicles');
  }

  async createVehicle(data: CreateVehicleData) {
    return this.request<Vehicle>('/api/v1/vehicles', {
      method: 'POST',
      body: JSON.stringify(data),
    });
  }

  async deleteVehicle(id: string) {
    return this.request<void>(`/api/v1/vehicles/${id}`, {
      method: 'DELETE',
    });
  }

  // Lot detailed (mock)
  async getLotDetailed(_id: string): Promise<ApiResponse<ParkingLotDetailed>> {
    return {
      success: true,
      data: {
        id: 'lot-1',
        name: 'Firmenparkplatz',
        address: 'Hauptstraße 1',
        total_slots: 13,
        available_slots: 8,
        layout: {
          roadLabel: 'Fahrweg',
          rows: [
            {
              id: 'row-a',
              side: 'top',
              label: 'Reihe A',
              slots: Array.from({ length: 6 }, (_, i) => ({
                id: `slot-a-${i}`,
                number: String(45 + i),
                status: (i === 2 ? 'occupied' : i === 4 ? 'reserved' : 'available') as SlotConfig['status'],
                vehiclePlate: i === 2 ? 'M-AB 1234' : undefined,
              })),
            },
            {
              id: 'row-b',
              side: 'bottom',
              label: 'Reihe B',
              slots: Array.from({ length: 7 }, (_, i) => ({
                id: `slot-b-${i}`,
                number: String(51 + i),
                status: (i === 1 ? 'occupied' : i === 5 ? 'occupied' : i === 3 ? 'disabled' : 'available') as SlotConfig['status'],
                vehiclePlate: i === 1 ? 'S-XY 5678' : i === 5 ? 'HH-CD 9012' : undefined,
              })),
            },
          ],
        },
      },
    };
  }

  // Health
  async health() {
    return this.request<{ status: string }>('/health');
  }

  // GDPR / User data
  async exportUserData() {
    return this.request<Record<string, unknown>>('/api/v1/users/me/export');
  }

  async deleteAccount() {
    return this.request<void>('/api/v1/users/me/delete', {
      method: 'DELETE',
    });
  }

  // Admin — Users
  async adminGetUsers() {
    return this.request<AdminUser[]>('/api/v1/admin/users');
  }

  async adminUpdateUserRole(id: string, role: AdminUser['role']) {
    return this.request<AdminUser>(`/api/v1/admin/users/${id}/role`, {
      method: 'PATCH',
      body: JSON.stringify({ role }),
    });
  }

  async adminUpdateUserStatus(id: string, status: AdminUser['status']) {
    return this.request<AdminUser>(`/api/v1/admin/users/${id}/status`, {
      method: 'PATCH',
      body: JSON.stringify({ status }),
    });
  }

  async adminDeleteUser(id: string) {
    return this.request<void>(`/api/v1/admin/users/${id}`, {
      method: 'DELETE',
    });
  }

  // Admin — Bookings
  async adminGetBookings() {
    return this.request<AdminBooking[]>('/api/v1/admin/bookings');
  }
}

export const api = new ApiClient();

// Types
export interface User {
  id: string;
  username: string;
  email: string;
  name: string;
  role: 'user' | 'admin' | 'superadmin';
  created_at: string;
  phone?: string;
  picture?: string;
}

export interface AuthTokens {
  access_token: string;
  refresh_token: string;
  token_type: string;
  /** ISO 8601 datetime string when the access token expires */
  expires_at: string;
}

export interface RegisterData {
  email: string;
  password: string;
  name: string;
}

export interface ParkingLot {
  id: string;
  name: string;
  address: string;
  total_slots: number;
  available_slots: number;
}

export interface ParkingSlot {
  id: string;
  lot_id: string;
  number: string;
  status: 'available' | 'occupied' | 'reserved' | 'disabled';
  floor?: number;
  section?: string;
}

export interface Booking {
  id: string;
  user_id: string;
  slot_id: string;
  lot_id: string;
  slot_number: string;
  lot_name: string;
  vehicle_plate?: string;
  start_time: string;
  end_time: string;
  status: 'active' | 'completed' | 'cancelled';
  created_at: string;
}

export interface CreateBookingData {
  slot_id: string;
  start_time: string;
  duration_minutes: number;
  vehicle_id?: string;
  license_plate?: string;
}

export interface Vehicle {
  id: string;
  user_id: string;
  license_plate: string;
  make?: string;
  model?: string;
  color?: string;
  is_default: boolean;
}

export interface CreateVehicleData {
  license_plate: string;
  make?: string;
  model?: string;
  color?: string;
}

export interface CreateLotData {
  id?: string;
  name: string;
  address: string;
  latitude?: number;
  longitude?: number;
  total_slots: number;
  available_slots: number;
  floors?: unknown[];
  amenities?: string[];
  pricing?: unknown;
  operating_hours?: unknown;
  images?: string[];
  status?: string;
}

// Parking lot layout configuration
export interface LotLayout {
  rows: LotRow[];
  roadLabel?: string;
}

export interface LotRow {
  id: string;
  side: 'top' | 'bottom';
  slots: SlotConfig[];
  label?: string;
}

export interface SlotConfig {
  id: string;
  number: string;
  status: 'available' | 'occupied' | 'reserved' | 'disabled' | 'blocked';
  vehiclePlate?: string;
  bookedBy?: string;
}

export interface ParkingLotDetailed extends ParkingLot {
  layout?: LotLayout;
}

export interface AdminUser {
  id: string;
  username: string;
  email: string;
  name: string;
  role: 'user' | 'admin' | 'superadmin';
  status: 'active' | 'disabled';
  created_at: string;
}

export interface AdminBooking {
  id: string;
  user_id: string;
  user_name: string;
  user_email: string;
  lot_id: string;
  lot_name: string;
  slot_id: string;
  slot_number: string;
  vehicle_plate?: string;
  start_time: string;
  end_time: string;
  status: 'active' | 'completed' | 'cancelled';
  booking_type?: 'one-time' | 'recurring' | 'guest';
  created_at: string;
}
