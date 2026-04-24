/**
 * useFleetEvents — subscribe to the backend SSE fleet stream (T-1946).
 *
 * Fleet screens (Einchecken / EV / Tausch) keep their 30 s polling fallback
 * via `useQuery({ staleTime: 30_000 })`. This hook upgrades them to
 * push-based updates so that a check-in / swap / guest action performed by
 * another client is visible in under 1 s instead of up to 30 s.
 *
 * ## Wire contract
 * Each SSE frame looks like:
 * ```text
 * event: checkin.completed
 * data: {"type":"checkin.completed","resource_id":"<uuid>","lot_id":"<uuid|null>","user_id":"<uuid|null>","timestamp":"2026-04-24T08:00:00Z"}
 * ```
 *
 * ## Usage
 * ```tsx
 * useFleetEvents({
 *   invalidate: {
 *     'checkin.started': [['einchecken-bookings']],
 *     'checkin.completed': [['einchecken-bookings'], ['checkin-status']],
 *   },
 * });
 * ```
 */

import { useEffect, useRef, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';

/** All event types emitted by the backend. */
export type FleetEventType =
  | 'checkin.started'
  | 'checkin.completed'
  | 'swap.requested'
  | 'swap.accepted'
  | 'swap.declined'
  | 'ev.session.started'
  | 'ev.session.stopped'
  | 'guest.created'
  | 'guest.cancelled';

/** Wire shape — matches `parkhub_common::FleetEvent` via `#[serde(rename="type")]`. */
export interface FleetEvent {
  type: FleetEventType;
  resource_id: string;
  lot_id: string | null;
  user_id: string | null;
  timestamp: string;
}

/** Map of event type → React-Query `queryKey` tuples to invalidate. */
export type FleetEventInvalidationMap = {
  [K in FleetEventType]?: ReadonlyArray<ReadonlyArray<unknown>>;
};

export interface UseFleetEventsOptions {
  /** Per-event-type queryKeys to invalidate when the event arrives. */
  invalidate: FleetEventInvalidationMap;
  /** Optional observer callback, called for each received event. */
  onEvent?: (event: FleetEvent) => void;
  /** Override the SSE URL (default: `/api/v1/events/fleet`). */
  url?: string;
  /**
   * Gate: when `false` the hook skips connecting. Used to defer until the
   * user is authenticated or to opt out for tests.
   * @default true
   */
  enabled?: boolean;
}

export interface UseFleetEventsResult {
  /** Whether the SSE connection is currently open. */
  connected: boolean;
}

const DEFAULT_URL = '/api/v1/events/fleet';

const ALL_EVENT_TYPES: FleetEventType[] = [
  'checkin.started',
  'checkin.completed',
  'swap.requested',
  'swap.accepted',
  'swap.declined',
  'ev.session.started',
  'ev.session.stopped',
  'guest.created',
  'guest.cancelled',
];

export function useFleetEvents(options: UseFleetEventsOptions): UseFleetEventsResult {
  const { invalidate, onEvent, url = DEFAULT_URL, enabled = true } = options;

  const qc = useQueryClient();
  const [connected, setConnected] = useState(false);

  // Stash references so the useEffect dependency array stays stable.
  const invalidateRef = useRef(invalidate);
  const onEventRef = useRef(onEvent);
  invalidateRef.current = invalidate;
  onEventRef.current = onEvent;

  useEffect(() => {
    if (!enabled) return;
    // Browsers: EventSource is always on `window`. Tests stub it on `globalThis`.
    const Ctor = (globalThis as { EventSource?: typeof EventSource }).EventSource;
    if (!Ctor) {
      // No EventSource polyfill — just skip (SSR, old browsers).
      return;
    }

    const es = new Ctor(url, { withCredentials: true });

    es.onopen = () => setConnected(true);
    es.onerror = () => setConnected(false);

    const handlers: Array<{ type: FleetEventType; h: (ev: MessageEvent) => void }> = [];
    for (const type of ALL_EVENT_TYPES) {
      const h = (ev: MessageEvent) => {
        let payload: FleetEvent | null = null;
        try {
          payload = JSON.parse(ev.data) as FleetEvent;
        } catch {
          return; // ignore malformed frames
        }
        if (!payload || payload.type !== type) return;
        onEventRef.current?.(payload);
        const keys = invalidateRef.current[type];
        if (keys) {
          for (const queryKey of keys) {
            qc.invalidateQueries({ queryKey: queryKey as unknown[] });
          }
        }
      };
      es.addEventListener(type, h);
      handlers.push({ type, h });
    }

    return () => {
      for (const { type, h } of handlers) {
        es.removeEventListener(type, h);
      }
      es.close();
      setConnected(false);
    };
  }, [url, enabled, qc]);

  return { connected };
}
