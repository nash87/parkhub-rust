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

  // Users
  async getCurrentUser() {
    return this.request<User>('/api/v1/users/me');
  }

  // Lots
  async getLots() {
    return this.request<ParkingLot[]>('/api/v1/lots');
  }

  async getLot(id: string) {
    return this.request<ParkingLot>(`/api/v1/lots/${id}`);
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

  // Health
  async health() {
    return this.request<{ status: string }>('/health');
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
}

export interface AuthTokens {
  access_token: string;
  refresh_token: string;
  token_type: string;
  expires_in: number;
}

export interface RegisterData {
  username: string;
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
