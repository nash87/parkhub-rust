/**
 * Tests for useFleetEvents (T-1946).
 *
 * Mocks the global `EventSource` so we can deterministically dispatch
 * fleet events and assert that the hook invalidates the right React-Query
 * cache keys.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { QueryClient } from '@tanstack/react-query';
import type { ReactNode } from 'react';
import React from 'react';
import { QueryClientProvider } from '@tanstack/react-query';
import { useFleetEvents, type FleetEvent, type FleetEventType } from './useFleetEvents';

class MockEventSource {
  static instances: MockEventSource[] = [];
  static CONNECTING = 0;
  static OPEN = 1;
  static CLOSED = 2;

  url: string;
  withCredentials: boolean;
  readyState = 0;
  onopen: ((ev: Event) => void) | null = null;
  onerror: ((ev: Event) => void) | null = null;
  onmessage: ((ev: MessageEvent) => void) | null = null;
  private listeners: Record<string, Array<(ev: MessageEvent) => void>> = {};
  close = vi.fn(() => {
    this.readyState = 2;
  });

  constructor(url: string, init?: EventSourceInit) {
    this.url = url;
    this.withCredentials = init?.withCredentials ?? false;
    MockEventSource.instances.push(this);
  }

  addEventListener(type: string, handler: (ev: MessageEvent) => void) {
    this.listeners[type] = this.listeners[type] ?? [];
    this.listeners[type].push(handler);
  }

  removeEventListener(type: string, handler: (ev: MessageEvent) => void) {
    this.listeners[type] = (this.listeners[type] ?? []).filter((h) => h !== handler);
  }

  simulateOpen() {
    this.readyState = 1;
    this.onopen?.(new Event('open'));
  }

  simulateError() {
    this.onerror?.(new Event('error'));
  }

  /** Fire an event of the given SSE `event:` name carrying a serialized FleetEvent. */
  fire(eventType: FleetEventType, payload: Omit<FleetEvent, 'type' | 'timestamp'> & Partial<Pick<FleetEvent, 'timestamp'>>) {
    const data: FleetEvent = {
      type: eventType,
      timestamp: payload.timestamp ?? new Date().toISOString(),
      resource_id: payload.resource_id,
      lot_id: payload.lot_id ?? null,
      user_id: payload.user_id ?? null,
    };
    const msg = new MessageEvent(eventType, { data: JSON.stringify(data) });
    (this.listeners[eventType] ?? []).forEach((h) => h(msg));
    // Also fire the generic 'message' listener for callers using onmessage.
    this.onmessage?.(msg);
  }
}

beforeEach(() => {
  MockEventSource.instances = [];
  vi.stubGlobal('EventSource', MockEventSource);
});
afterEach(() => {
  vi.restoreAllMocks();
});

function wrapper(qc: QueryClient) {
  return ({ children }: { children: ReactNode }) =>
    React.createElement(QueryClientProvider, { client: qc }, children);
}

describe('useFleetEvents', () => {
  it('opens EventSource at /api/v1/events/fleet with credentials', () => {
    const qc = new QueryClient();
    renderHook(() => useFleetEvents({ invalidate: {} }), { wrapper: wrapper(qc) });
    expect(MockEventSource.instances).toHaveLength(1);
    const es = MockEventSource.instances[0];
    expect(es!.url).toContain('/api/v1/events/fleet');
    expect(es!.withCredentials).toBe(true);
  });

  it('reports connected=true after open', () => {
    const qc = new QueryClient();
    const { result } = renderHook(() => useFleetEvents({ invalidate: {} }), {
      wrapper: wrapper(qc),
    });
    expect(result.current.connected).toBe(false);
    act(() => MockEventSource.instances[0]!.simulateOpen());
    expect(result.current.connected).toBe(true);
  });

  it('invalidates the right queryKey on matching event type', () => {
    const qc = new QueryClient();
    const invalidate = vi.spyOn(qc, 'invalidateQueries');
    renderHook(
      () =>
        useFleetEvents({
          invalidate: {
            'checkin.started': [['einchecken-bookings']],
            'checkin.completed': [['einchecken-bookings'], ['checkin-status']],
          },
        }),
      { wrapper: wrapper(qc) },
    );

    act(() => MockEventSource.instances[0]!.simulateOpen());
    act(() =>
      MockEventSource.instances[0]!.fire('checkin.completed', {
        resource_id: 'b-1',
        lot_id: 'lot-1',
        user_id: 'u-1',
      }),
    );

    expect(invalidate).toHaveBeenCalledWith({ queryKey: ['einchecken-bookings'] });
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ['checkin-status'] });
  });

  it('does not invalidate on unmatched event types', () => {
    const qc = new QueryClient();
    const invalidate = vi.spyOn(qc, 'invalidateQueries');
    renderHook(
      () =>
        useFleetEvents({
          invalidate: {
            'checkin.completed': [['einchecken-bookings']],
          },
        }),
      { wrapper: wrapper(qc) },
    );
    act(() => MockEventSource.instances[0]!.simulateOpen());
    act(() =>
      MockEventSource.instances[0]!.fire('guest.created', {
        resource_id: 'g-1',
        lot_id: 'lot-1',
        user_id: 'u-1',
      }),
    );
    expect(invalidate).not.toHaveBeenCalled();
  });

  it('invokes onEvent callback with every matching event', () => {
    const qc = new QueryClient();
    const onEvent = vi.fn();
    renderHook(
      () =>
        useFleetEvents({
          invalidate: { 'swap.accepted': [['tausch']] },
          onEvent,
        }),
      { wrapper: wrapper(qc) },
    );
    act(() => MockEventSource.instances[0]!.simulateOpen());
    act(() =>
      MockEventSource.instances[0]!.fire('swap.accepted', {
        resource_id: 'swap-1',
        lot_id: null,
        user_id: 'u-1',
      }),
    );
    expect(onEvent).toHaveBeenCalledTimes(1);
    const ev = onEvent.mock.calls[0]![0] as FleetEvent;
    expect(ev.type).toBe('swap.accepted');
    expect(ev.resource_id).toBe('swap-1');
  });

  it('cleans up EventSource on unmount', () => {
    const qc = new QueryClient();
    const { unmount } = renderHook(() => useFleetEvents({ invalidate: {} }), {
      wrapper: wrapper(qc),
    });
    const es = MockEventSource.instances[0];
    act(() => es!.simulateOpen());
    unmount();
    expect(es!.close).toHaveBeenCalled();
  });

  it('reports connected=false after error', () => {
    const qc = new QueryClient();
    const { result } = renderHook(() => useFleetEvents({ invalidate: {} }), {
      wrapper: wrapper(qc),
    });
    act(() => MockEventSource.instances[0]!.simulateOpen());
    expect(result.current.connected).toBe(true);
    act(() => MockEventSource.instances[0]!.simulateError());
    expect(result.current.connected).toBe(false);
  });

  it('supports disabled option to skip connection', () => {
    const qc = new QueryClient();
    renderHook(() => useFleetEvents({ invalidate: {}, enabled: false }), {
      wrapper: wrapper(qc),
    });
    expect(MockEventSource.instances).toHaveLength(0);
  });
});
