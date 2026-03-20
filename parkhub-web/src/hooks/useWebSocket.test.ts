import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useWebSocket, type WsEvent } from './useWebSocket';

class MockWebSocket {
  static instances: MockWebSocket[] = [];
  url: string;
  onopen: (() => void) | null = null;
  onmessage: ((msg: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  readyState = 0;
  close = vi.fn(() => { this.readyState = 3; });
  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }
  simulateOpen() { this.readyState = 1; this.onopen?.(); }
  simulateMessage(event: WsEvent) { this.onmessage?.({ data: JSON.stringify(event) }); }
  simulateClose() { this.readyState = 3; this.onclose?.(); }
}

beforeEach(() => { MockWebSocket.instances = []; vi.stubGlobal('WebSocket', MockWebSocket); vi.useFakeTimers(); });
afterEach(() => { vi.useRealTimers(); vi.restoreAllMocks(); });

describe('useWebSocket', () => {
  it('connects to default ws URL', () => {
    renderHook(() => useWebSocket());
    expect(MockWebSocket.instances).toHaveLength(1);
    expect(MockWebSocket.instances[0].url).toContain('/api/v1/ws');
  });

  it('reports connected state after open', () => {
    const { result } = renderHook(() => useWebSocket({ autoReconnect: false }));
    expect(result.current.connected).toBe(false);
    act(() => MockWebSocket.instances[0].simulateOpen());
    expect(result.current.connected).toBe(true);
  });

  it('receives events and calls onEvent callback', () => {
    const onEvent = vi.fn();
    const { result } = renderHook(() => useWebSocket({ onEvent, autoReconnect: false }));
    const ws = MockWebSocket.instances[0];
    act(() => ws.simulateOpen());
    const event: WsEvent = { event: 'booking_created', data: { booking_id: 'abc' }, timestamp: '2026-03-21T10:00:00Z' };
    act(() => ws.simulateMessage(event));
    expect(onEvent).toHaveBeenCalledWith(event);
    expect(result.current.lastEvent).toEqual(event);
  });

  it('auto-reconnects with exponential backoff', () => {
    renderHook(() => useWebSocket({ reconnectDelay: 100 }));
    const ws1 = MockWebSocket.instances[MockWebSocket.instances.length - 1];
    act(() => ws1.simulateOpen());
    act(() => ws1.simulateClose());
    const countAfterClose = MockWebSocket.instances.length;
    // After 100ms (100 * 2^0) a reconnect should happen
    act(() => vi.advanceTimersByTime(100));
    expect(MockWebSocket.instances.length).toBeGreaterThan(countAfterClose);
  });

  it('does not reconnect when autoReconnect is false', () => {
    renderHook(() => useWebSocket({ autoReconnect: false }));
    act(() => MockWebSocket.instances[0].simulateOpen());
    act(() => MockWebSocket.instances[0].simulateClose());
    act(() => vi.advanceTimersByTime(60_000));
    expect(MockWebSocket.instances).toHaveLength(1);
  });

  it('cleans up on unmount', () => {
    const { unmount } = renderHook(() => useWebSocket({ autoReconnect: false }));
    act(() => MockWebSocket.instances[0].simulateOpen());
    unmount();
    expect(MockWebSocket.instances[0].close).toHaveBeenCalled();
  });

  it('resets retry count on successful connection', () => {
    renderHook(() => useWebSocket({ reconnectDelay: 100 }));
    act(() => MockWebSocket.instances[0].simulateOpen());
    act(() => MockWebSocket.instances[0].simulateClose());
    act(() => vi.advanceTimersByTime(100));
    expect(MockWebSocket.instances).toHaveLength(2);
    act(() => MockWebSocket.instances[1].simulateOpen());
    act(() => MockWebSocket.instances[1].simulateClose());
    act(() => vi.advanceTimersByTime(100));
    expect(MockWebSocket.instances).toHaveLength(3);
  });

  it('ignores non-JSON messages', () => {
    const onEvent = vi.fn();
    renderHook(() => useWebSocket({ onEvent, autoReconnect: false }));
    act(() => MockWebSocket.instances[0].simulateOpen());
    act(() => MockWebSocket.instances[0].onmessage?.({ data: 'not json' }));
    expect(onEvent).not.toHaveBeenCalled();
  });

  it('uses custom URL when provided', () => {
    renderHook(() => useWebSocket({ url: 'ws://custom:8080/ws' }));
    expect(MockWebSocket.instances[0].url).toBe('ws://custom:8080/ws');
  });
});
