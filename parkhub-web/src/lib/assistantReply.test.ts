import { describe, it, expect, vi, beforeEach } from 'vitest';
import { buildLiveReply, defaultReply } from './assistantReply';

vi.mock('../api/client', () => ({
  api: {
    getUserCredits: vi.fn(),
    getUserStats: vi.fn(),
    getBookings: vi.fn(),
    getVehicles: vi.fn(),
  },
}));

import { api } from '../api/client';

const mockedApi = api as unknown as {
  getUserCredits: ReturnType<typeof vi.fn>;
  getUserStats: ReturnType<typeof vi.fn>;
  getBookings: ReturnType<typeof vi.fn>;
  getVehicles: ReturnType<typeof vi.fn>;
};

describe('buildLiveReply', () => {
  beforeEach(() => vi.clearAllMocks());

  it('returns credit balance in a reply sentence', async () => {
    mockedApi.getUserCredits.mockResolvedValue({
      success: true,
      data: {
        enabled: true, balance: 12, monthly_quota: 30,
        last_refilled: new Date().toISOString(), transactions: [],
      },
    });
    const r = await buildLiveReply('how many credits left?');
    expect(r).toContain('12');
    expect(r).toContain('30');
  });

  it('surfaces the next upcoming booking', async () => {
    const future = new Date(Date.now() + 2 * 86_400_000);
    const end = new Date(future.getTime() + 4 * 3600_000);
    mockedApi.getBookings.mockResolvedValue({
      success: true,
      data: [{
        id: 'b1', user_id: 'u', lot_id: 'l', slot_id: 's',
        lot_name: 'HQ Garage', slot_number: 'L2-08',
        start_time: future.toISOString(), end_time: end.toISOString(),
        status: 'confirmed',
      }],
    });
    const r = await buildLiveReply('what is my next booking?');
    expect(r).toContain('HQ Garage');
    expect(r).toContain('L2-08');
  });

  it('summarises monthly stats', async () => {
    mockedApi.getUserStats.mockResolvedValue({
      success: true,
      data: {
        total_bookings: 120, bookings_this_month: 8,
        homeoffice_days_this_month: 3, avg_duration_minutes: 540,
      },
    });
    const r = await buildLiveReply('stats this month please');
    expect(r).toContain('8');
    expect(r).toContain('120');
  });

  it('lists vehicles by plate', async () => {
    mockedApi.getVehicles.mockResolvedValue({
      success: true,
      data: [
        { id: '1', plate: 'ABC-123', make: 'Tesla', model: 'Model 3', is_default: true },
        { id: '2', plate: 'DEF-456', is_default: false },
      ],
    });
    const r = await buildLiveReply('list my cars');
    expect(r).toContain('ABC-123');
    expect(r).toContain('Tesla Model 3');
    expect(r).toContain('DEF-456');
  });

  it('falls back to an on-prem helper tip on unknown intents', async () => {
    const r = await buildLiveReply('what is the meaning of life?');
    expect(r.toLowerCase()).toContain('local data');
  });

  it('surfaces API failure as a polite error', async () => {
    mockedApi.getUserCredits.mockRejectedValue(new Error('boom'));
    const r = await buildLiveReply('credits');
    expect(r.toLowerCase()).toContain('boom');
  });
});

describe('defaultReply', () => {
  it('routes a credit question to the credits fallback', () => {
    expect(defaultReply('what is my credit balance')).toContain('credit');
  });
  it('responds with a prompt-tip on unknown', () => {
    expect(defaultReply('something else').toLowerCase()).toContain('local data');
  });
});
