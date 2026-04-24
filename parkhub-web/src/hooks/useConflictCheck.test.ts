import { describe, it, expect } from 'vitest';
import { findOverlappingBooking, type BookingWindow } from './useConflictCheck';

function w(id: string, lot: string, start: string, end: string, status = 'active'): BookingWindow {
  return { id, lot_name: lot, start_time: start, end_time: end, status };
}

describe('findOverlappingBooking', () => {
  it('returns null when no bookings overlap', () => {
    const existing = [w('b1', 'Alpha', '2026-04-24T10:00:00Z', '2026-04-24T12:00:00Z')];
    const result = findOverlappingBooking(existing, new Date('2026-04-24T13:00:00Z'), new Date('2026-04-24T14:00:00Z'));
    expect(result).toBeNull();
  });

  it('returns overlapping booking when start is inside an existing booking window', () => {
    const existing = [w('b1', 'Alpha', '2026-04-24T10:00:00Z', '2026-04-24T12:00:00Z')];
    const result = findOverlappingBooking(existing, new Date('2026-04-24T11:00:00Z'), new Date('2026-04-24T13:00:00Z'));
    expect(result?.id).toBe('b1');
  });

  it('returns overlapping booking when end is inside an existing window', () => {
    const existing = [w('b1', 'Alpha', '2026-04-24T10:00:00Z', '2026-04-24T12:00:00Z')];
    const result = findOverlappingBooking(existing, new Date('2026-04-24T09:00:00Z'), new Date('2026-04-24T10:30:00Z'));
    expect(result?.id).toBe('b1');
  });

  it('returns overlapping booking when new window fully contains existing', () => {
    const existing = [w('b1', 'Alpha', '2026-04-24T10:00:00Z', '2026-04-24T12:00:00Z')];
    const result = findOverlappingBooking(existing, new Date('2026-04-24T09:00:00Z'), new Date('2026-04-24T14:00:00Z'));
    expect(result?.id).toBe('b1');
  });

  it('ignores cancelled / completed bookings', () => {
    const existing = [
      w('b1', 'Alpha', '2026-04-24T10:00:00Z', '2026-04-24T12:00:00Z', 'cancelled'),
      w('b2', 'Beta', '2026-04-24T10:00:00Z', '2026-04-24T12:00:00Z', 'completed'),
    ];
    const result = findOverlappingBooking(existing, new Date('2026-04-24T11:00:00Z'), new Date('2026-04-24T13:00:00Z'));
    expect(result).toBeNull();
  });

  it('treats adjacent (touching) windows as non-overlapping', () => {
    const existing = [w('b1', 'Alpha', '2026-04-24T10:00:00Z', '2026-04-24T12:00:00Z')];
    const result = findOverlappingBooking(existing, new Date('2026-04-24T12:00:00Z'), new Date('2026-04-24T13:00:00Z'));
    expect(result).toBeNull();
  });
});
