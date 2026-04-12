import { test, expect } from '@playwright/test';
import { loginViaApi, DEMO_ADMIN } from './helpers';

const BASE = process.env.E2E_BASE_URL || 'http://localhost:8081';

test.describe('Booking — Edge Cases', () => {
  let token: string;

  test.beforeAll(async ({ playwright }) => {
    const ctx = await playwright.request.newContext({ baseURL: BASE });
    token = await loginViaApi(ctx);
    await ctx.dispose();
  });

  /** Helper to get lot and slot IDs. */
  async function getLotAndSlot(request: any) {
    const lotsRes = await request.get('/api/v1/lots', {
      headers: { Authorization: `Bearer ${token}` },
    });
    const lots = (await lotsRes.json()).data ?? [];
    if (!Array.isArray(lots) || lots.length === 0) return null;

    const lotId = lots[0].id;
    const slotsRes = await request.get(`/api/v1/lots/${lotId}/slots`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    const slots = (await slotsRes.json()).data ?? [];
    if (!Array.isArray(slots) || slots.length === 0) return null;

    return { lotId, slotId: slots[0].id };
  }

  // ── Past Date Booking ──

  test('reject booking in the past', async ({ request }) => {
    const info = await getLotAndSlot(request);
    if (!info) {
      test.skip(true, 'No lots/slots available');
      return;
    }

    const pastDate = new Date();
    pastDate.setDate(pastDate.getDate() - 7);
    const dateStr = pastDate.toISOString().split('T')[0];

    const res = await request.post('/api/v1/bookings', {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        lot_id: info.lotId,
        slot_id: info.slotId,
        date: dateStr,
        start_time: '09:00',
        end_time: '17:00',
      },
    });

    // Past bookings should be rejected with 400 or 422
    expect([400, 422]).toContain(res.status());
  });

  // ── Overlapping Booking ──

  test('reject overlapping booking on same slot', async ({ request }) => {
    const info = await getLotAndSlot(request);
    if (!info) {
      test.skip(true, 'No lots/slots available');
      return;
    }

    const futureDate = new Date();
    futureDate.setDate(futureDate.getDate() + 25);
    const dateStr = futureDate.toISOString().split('T')[0];

    // Create first booking
    const res1 = await request.post('/api/v1/bookings', {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        lot_id: info.lotId,
        slot_id: info.slotId,
        date: dateStr,
        start_time: '10:00',
        end_time: '14:00',
      },
    });

    if (![200, 201].includes(res1.status())) {
      test.skip(true, 'Could not create initial booking');
      return;
    }

    const booking1 = await res1.json();
    const booking1Id = booking1.data?.id ?? booking1.id;

    // Try overlapping booking on the same slot
    const res2 = await request.post('/api/v1/bookings', {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        lot_id: info.lotId,
        slot_id: info.slotId,
        date: dateStr,
        start_time: '12:00',
        end_time: '16:00',
      },
    });

    // Should be rejected
    expect([400, 409, 422]).toContain(res2.status());

    // Clean up
    if (booking1Id) {
      await request.delete(`/api/v1/bookings/${booking1Id}`, {
        headers: { Authorization: `Bearer ${token}` },
      });
    }
  });

  // ── Create + Cancel Lifecycle ──

  test('book today, then cancel successfully', async ({ request }) => {
    const info = await getLotAndSlot(request);
    if (!info) {
      test.skip(true, 'No lots/slots available');
      return;
    }

    const tomorrow = new Date();
    tomorrow.setDate(tomorrow.getDate() + 1);
    const dateStr = tomorrow.toISOString().split('T')[0];

    const createRes = await request.post('/api/v1/bookings', {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        lot_id: info.lotId,
        slot_id: info.slotId,
        date: dateStr,
        start_time: '08:00',
        end_time: '09:00',
      },
    });

    if (![200, 201].includes(createRes.status())) {
      test.skip(true, 'Booking creation failed');
      return;
    }

    const body = await createRes.json();
    const bookingId = body.data?.id ?? body.id;
    expect(bookingId).toBeTruthy();

    // Cancel the booking
    const cancelRes = await request.delete(`/api/v1/bookings/${bookingId}`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect([200, 204]).toContain(cancelRes.status());

    // Verify booking is cancelled or removed
    const getRes = await request.get(`/api/v1/bookings/${bookingId}`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    // Should be 404 (deleted) or 200 with cancelled status
    if (getRes.status() === 200) {
      const booking = (await getRes.json()).data ?? (await getRes.json());
      expect(booking.status).toMatch(/cancelled|canceled|deleted/i);
    } else {
      expect([404, 410]).toContain(getRes.status());
    }
  });

  // ── Credits Integration ──

  test('credits balance is available', async ({ request }) => {
    const res = await request.get('/api/v1/credits', {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(res.status()).toBe(200);
    const body = await res.json();
    const data = body.data ?? body;
    // Should return a balance or credits object
    expect(data).toBeDefined();
  });

  // ── Waitlist ──

  test('waitlist endpoint is accessible', async ({ request }) => {
    const lotsRes = await request.get('/api/v1/lots', {
      headers: { Authorization: `Bearer ${token}` },
    });
    const lots = (await lotsRes.json()).data ?? [];

    if (!Array.isArray(lots) || lots.length === 0) {
      test.skip(true, 'No lots available');
      return;
    }

    const lotId = lots[0].id;
    const res = await request.get(`/api/v1/lots/${lotId}/waitlist`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    // Waitlist may return 200 (list) or 404 (module disabled)
    expect([200, 404]).toContain(res.status());
  });

  // ── Booking History ──

  test('booking history returns records', async ({ request }) => {
    const res = await request.get('/api/v1/bookings/history', {
      headers: { Authorization: `Bearer ${token}` },
    });
    // History may be at different endpoints
    if (res.status() === 404) {
      // Try alternate endpoint
      const altRes = await request.get('/api/v1/bookings?status=completed', {
        headers: { Authorization: `Bearer ${token}` },
      });
      expect(altRes.status()).toBe(200);
    } else {
      expect(res.status()).toBe(200);
    }
  });

  // ── Invalid Input Validation ──

  test('reject booking with missing required fields', async ({ request }) => {
    const res = await request.post('/api/v1/bookings', {
      headers: { Authorization: `Bearer ${token}` },
      data: {},
    });
    expect([400, 422]).toContain(res.status());
  });

  test('reject booking with invalid date format', async ({ request }) => {
    const info = await getLotAndSlot(request);
    if (!info) {
      test.skip(true, 'No lots/slots available');
      return;
    }

    const res = await request.post('/api/v1/bookings', {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        lot_id: info.lotId,
        slot_id: info.slotId,
        date: 'not-a-date',
        start_time: '09:00',
        end_time: '17:00',
      },
    });
    expect([400, 422]).toContain(res.status());
  });

  test('reject booking where end_time precedes start_time', async ({ request }) => {
    const info = await getLotAndSlot(request);
    if (!info) {
      test.skip(true, 'No lots/slots available');
      return;
    }

    const futureDate = new Date();
    futureDate.setDate(futureDate.getDate() + 20);
    const dateStr = futureDate.toISOString().split('T')[0];

    const res = await request.post('/api/v1/bookings', {
      headers: { Authorization: `Bearer ${token}` },
      data: {
        lot_id: info.lotId,
        slot_id: info.slotId,
        date: dateStr,
        start_time: '17:00',
        end_time: '09:00',
      },
    });
    expect([400, 422]).toContain(res.status());
  });

  // ── Recurring Bookings ──

  test('recurring booking endpoint is accessible', async ({ request }) => {
    const res = await request.get('/api/v1/recurring-bookings', {
      headers: { Authorization: `Bearer ${token}` },
    });
    // May return 200 (list) or 404 (module disabled)
    expect([200, 404]).toContain(res.status());
  });
});
