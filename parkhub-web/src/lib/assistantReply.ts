/**
 * Real-data reply builder for the Assistant.
 *
 * Replaces the original pure-pattern `defaultReply` with a matcher that
 * queries the running parkhub-server for the signals a user is likely to
 * ask about — credits, upcoming bookings, vehicles, stats — and
 * synthesises a reply sentence from the numbers. The rule-based framing
 * from the claude.ai/design handoff stays intact: no LLM, no network
 * beyond the local server, and every reply is deterministic.
 *
 * Exports:
 *  - `buildLiveReply`   — async reply builder, used by <Assistant> when
 *                         plugged in via the `reply` prop.
 *  - `defaultReply`     — kept as a synchronous fallback for stories /
 *                         tests that don't have an API context.
 */
// Types only — the actual `api` runtime is imported lazily inside
// buildLiveReply() so this module stays a leaf dependency of <Assistant>
// and doesn't pull the ~80KB API client into the Layout critical-path
// chunk. The dynamic import resolves once (cached by the bundler) so
// subsequent Assistant queries don't re-download.
import type { Booking, UserCredits, UserStats, Vehicle } from '../api/client';

type BuildOpts = {
  /** Optional locale code for number formatting (defaults to the browser). */
  locale?: string;
};

const NF = (locale?: string) => new Intl.NumberFormat(locale);
const RF = (locale?: string) =>
  new Intl.RelativeTimeFormat(locale ?? 'en', { numeric: 'auto' });

function matchIntent(q: string): Intent {
  const lc = q.toLowerCase();
  if (/\b(credit|credits|coin|coins|balance|quota)\b/.test(lc)) return 'credits';
  if (/\b(next|upcoming|tomorrow)\b/.test(lc) || /today.*book/.test(lc)) return 'next_booking';
  if (/\b(yesterday|last\s+(?:booking|week)|history)\b/.test(lc) || /where.*park/.test(lc)) return 'recent';
  if (/\b(stats|monthly|average|homeoffice|home\s*office)\b/.test(lc) || /this\s+month/.test(lc)) return 'stats';
  if (/\b(vehicles?|cars?|plate|plates)\b/.test(lc)) return 'vehicles';
  return 'unknown';
}

type Intent = 'credits' | 'next_booking' | 'recent' | 'stats' | 'vehicles' | 'unknown';

function fmtDateRange(b: Booking, locale?: string) {
  const start = new Date(b.start_time);
  const end = new Date(b.end_time);
  const dd = new Intl.DateTimeFormat(locale, {
    weekday: 'short', month: 'short', day: 'numeric',
    hour: '2-digit', minute: '2-digit',
  });
  const hh = new Intl.DateTimeFormat(locale, { hour: '2-digit', minute: '2-digit' });
  return `${dd.format(start)} → ${hh.format(end)}`;
}

function fmtDayRelative(ts: string, locale?: string): string {
  const t = new Date(ts).getTime();
  const now = Date.now();
  const dayMs = 86_400_000;
  const diffDays = Math.round((t - now) / dayMs);
  const rtf = RF(locale);
  if (Math.abs(diffDays) < 14) return rtf.format(diffDays, 'day');
  return new Intl.DateTimeFormat(locale, { dateStyle: 'medium' }).format(new Date(ts));
}

export async function buildLiveReply(q: string, opts: BuildOpts = {}): Promise<string> {
  const intent = matchIntent(q);
  const nf = NF(opts.locale);
  // Lazy-import keeps this module out of the Layout-critical chunk.
  const { api } = await import('../api/client');

  try {
    switch (intent) {
      case 'credits': {
        const res = await api.getUserCredits();
        if (!res.success || !res.data) return "I couldn't reach the credits endpoint just now.";
        const c: UserCredits = res.data;
        if (!c.enabled) return 'Credits are disabled for this workspace.';
        const used = Math.max(0, c.monthly_quota - c.balance);
        return `You have ${nf.format(c.balance)} credits left (of ${nf.format(c.monthly_quota)} this month, ${nf.format(used)} used). Last refilled ${c.last_refilled ? fmtDayRelative(c.last_refilled, opts.locale) : 'no record'}.`;
      }

      case 'next_booking': {
        const res = await api.getBookings();
        if (!res.success || !res.data) return "I couldn't reach the bookings endpoint just now.";
        const upcoming = [...res.data]
          .filter(b => new Date(b.end_time).getTime() > Date.now())
          .sort((a, b) => new Date(a.start_time).getTime() - new Date(b.start_time).getTime());
        if (upcoming.length === 0) return 'No upcoming bookings on your account.';
        const next = upcoming[0];
        const hrs = Math.round((new Date(next.end_time).getTime() - new Date(next.start_time).getTime()) / 36e5);
        return `Next booking: ${fmtDateRange(next, opts.locale)} at ${next.lot_name ?? 'the lot'}${next.slot_number ? ` slot ${next.slot_number}` : ''} (${hrs}h).`;
      }

      case 'recent': {
        const res = await api.getBookings();
        if (!res.success || !res.data) return "I couldn't reach the bookings endpoint just now.";
        const past = [...res.data]
          .filter(b => new Date(b.end_time).getTime() <= Date.now())
          .sort((a, b) => new Date(b.end_time).getTime() - new Date(a.end_time).getTime());
        if (past.length === 0) return 'No past bookings yet.';
        const last = past[0];
        return `Last booking was ${fmtDayRelative(last.end_time, opts.locale)} — ${last.lot_name ?? 'a lot'}${last.slot_number ? ` slot ${last.slot_number}` : ''} for ${Math.round((new Date(last.end_time).getTime() - new Date(last.start_time).getTime()) / 36e5)}h.`;
      }

      case 'stats': {
        const res = await api.getUserStats();
        if (!res.success || !res.data) return "I couldn't reach the stats endpoint just now.";
        const s: UserStats = res.data;
        const h = Math.round(s.avg_duration_minutes / 6) / 10;
        return `This month: ${nf.format(s.bookings_this_month)} bookings, ${nf.format(s.homeoffice_days_this_month)} home-office days. All-time: ${nf.format(s.total_bookings)} bookings, avg ${nf.format(h)}h each${s.favorite_slot ? `, favourite slot ${s.favorite_slot}` : ''}.`;
      }

      case 'vehicles': {
        const res = await api.getVehicles();
        if (!res.success || !res.data) return "I couldn't reach the vehicles endpoint just now.";
        const v: Vehicle[] = res.data;
        if (v.length === 0) return 'No vehicles on file. Add one in Vehicles to pre-fill it on bookings.';
        const names = v.map(x => {
          const label = [x.make, x.model].filter(Boolean).join(' ');
          return label ? `${x.plate} (${label})` : x.plate;
        }).join(', ');
        return `${nf.format(v.length)} vehicle${v.length === 1 ? '' : 's'}: ${names}.`;
      }

      default:
        return 'I only answer from your local data. Try asking about credits, your next booking, where you parked last, monthly stats, or your vehicles.';
    }
  } catch (e) {
    return `Couldn't answer right now — ${e instanceof Error ? e.message : 'unknown error'}.`;
  }
}

// Synchronous fallback kept for test contexts without an API mock.
export function defaultReply(q: string): string {
  const intent = matchIntent(q);
  switch (intent) {
    case 'credits': return 'Ask me again once the server is reachable and I can fetch your credit balance.';
    case 'next_booking': return 'Ask me again once the server is reachable and I can show your next booking.';
    case 'recent': return 'Ask me again once the server is reachable and I can surface your last booking.';
    case 'stats': return 'Ask me again once the server is reachable and I can pull your monthly stats.';
    case 'vehicles': return 'Ask me again once the server is reachable and I can list your vehicles.';
    default:
      return 'I only answer from your local data. Try asking about credits, your next booking, where you parked last, monthly stats, or your vehicles.';
  }
}
