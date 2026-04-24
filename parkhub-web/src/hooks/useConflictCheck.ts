/**
 * Pure booking-overlap detector used by the Buchen / Book flow (Tier-2 item 8).
 *
 * Returns the first active/confirmed booking whose window overlaps
 * the requested [start, end) interval, or null if none overlaps.
 *
 * Windows that only touch at an endpoint (existing.end === new.start,
 * or existing.start === new.end) are treated as non-overlapping.
 */
export interface BookingWindow {
  id: string;
  lot_name: string;
  start_time: string;
  end_time: string;
  status: string;
}

const ACTIVE_STATUSES = new Set(['active', 'confirmed', 'checked_in']);

export function findOverlappingBooking(
  bookings: readonly BookingWindow[],
  newStart: Date,
  newEnd: Date,
): BookingWindow | null {
  const ns = newStart.getTime();
  const ne = newEnd.getTime();
  for (const b of bookings) {
    if (!ACTIVE_STATUSES.has(b.status)) continue;
    const bs = new Date(b.start_time).getTime();
    const be = new Date(b.end_time).getTime();
    if (Number.isNaN(bs) || Number.isNaN(be)) continue;
    // Overlap iff new.start < existing.end AND existing.start < new.end
    if (ns < be && bs < ne) return b;
  }
  return null;
}
