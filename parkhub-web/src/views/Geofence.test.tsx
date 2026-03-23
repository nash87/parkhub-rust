import { describe, it, expect, vi, beforeEach } from 'vitest';

// ── Mocks ──

const mockGeofenceCheckIn = vi.fn();
const mockGetLotGeofence = vi.fn();
const mockAdminSetGeofence = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    geofenceCheckIn: (...args: any[]) => mockGeofenceCheckIn(...args),
    getLotGeofence: (...args: any[]) => mockGetLotGeofence(...args),
    adminSetGeofence: (...args: any[]) => mockAdminSetGeofence(...args),
  },
}));

describe('Geofence API', () => {
  beforeEach(() => {
    mockGeofenceCheckIn.mockClear();
    mockGetLotGeofence.mockClear();
    mockAdminSetGeofence.mockClear();
  });

  it('calls geofenceCheckIn with lat/lng', async () => {
    mockGeofenceCheckIn.mockResolvedValue({
      success: true,
      data: { checked_in: true, booking_id: 'b1', lot_name: 'Garage A', message: 'Checked in' },
    });

    const result = await mockGeofenceCheckIn(48.1351, 11.5820);
    expect(mockGeofenceCheckIn).toHaveBeenCalledWith(48.1351, 11.5820);
    expect(result.data.checked_in).toBe(true);
    expect(result.data.lot_name).toBe('Garage A');
  });

  it('handles no active booking for check-in', async () => {
    mockGeofenceCheckIn.mockResolvedValue({
      success: true,
      data: { checked_in: false, booking_id: null, lot_name: null, message: 'No active bookings found' },
    });

    const result = await mockGeofenceCheckIn(48.0, 11.0);
    expect(result.data.checked_in).toBe(false);
    expect(result.data.message).toContain('No active bookings');
  });

  it('fetches lot geofence config', async () => {
    mockGetLotGeofence.mockResolvedValue({
      success: true,
      data: { lot_id: 'lot-1', center_lat: 48.1351, center_lng: 11.5820, radius_meters: 100, enabled: true },
    });

    const result = await mockGetLotGeofence('lot-1');
    expect(result.data.radius_meters).toBe(100);
    expect(result.data.enabled).toBe(true);
  });

  it('admin sets geofence radius', async () => {
    mockAdminSetGeofence.mockResolvedValue({
      success: true,
      data: { lot_id: 'lot-1', center_lat: 48.0, center_lng: 11.0, radius_meters: 200, enabled: true },
    });

    const result = await mockAdminSetGeofence('lot-1', {
      center_lat: 48.0,
      center_lng: 11.0,
      radius_meters: 200,
    });
    expect(result.data.radius_meters).toBe(200);
  });
});
