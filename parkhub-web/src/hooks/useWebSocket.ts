import { useEffect, useRef, useCallback, useState } from 'react';

export type WsEventType = 'booking_created' | 'booking_cancelled' | 'occupancy_changed';

export interface WsEvent {
  event: WsEventType;
  data: Record<string, unknown>;
  timestamp: string;
}

export type WsEventHandler = (event: WsEvent) => void;

interface UseWebSocketOptions {
  url?: string;
  autoReconnect?: boolean;
  reconnectDelay?: number;
  onEvent?: WsEventHandler;
}

export function useWebSocket(options: UseWebSocketOptions = {}) {
  const {
    autoReconnect = true,
    reconnectDelay = 1000,
    onEvent,
  } = options;

  const [connected, setConnected] = useState(false);
  const [lastEvent, setLastEvent] = useState<WsEvent | null>(null);

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const retryCount = useRef(0);
  const onEventRef = useRef(onEvent);
  const unmountedRef = useRef(false);

  onEventRef.current = onEvent;

  const getWsUrl = useCallback(() => {
    if (options.url) return options.url;
    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${proto}//${window.location.host}/api/v1/ws`;
  }, [options.url]);

  const connect = useCallback(() => {
    if (unmountedRef.current) return;
    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }

    const ws = new WebSocket(getWsUrl());
    wsRef.current = ws;

    ws.onopen = () => {
      setConnected(true);
      retryCount.current = 0;
    };

    ws.onmessage = (msg) => {
      try {
        const event: WsEvent = JSON.parse(msg.data);
        setLastEvent(event);
        onEventRef.current?.(event);
      } catch {
        // Ignore non-JSON messages
      }
    };

    ws.onclose = () => {
      setConnected(false);
      wsRef.current = null;
      if (autoReconnect && !unmountedRef.current) {
        const delay = Math.min(reconnectDelay * Math.pow(2, retryCount.current), 30_000);
        retryCount.current += 1;
        reconnectTimer.current = setTimeout(connect, delay);
      }
    };

    ws.onerror = () => {};
  }, [getWsUrl, autoReconnect, reconnectDelay]);

  useEffect(() => {
    unmountedRef.current = false;
    connect();
    return () => {
      unmountedRef.current = true;
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current);
      if (wsRef.current) { wsRef.current.close(); wsRef.current = null; }
    };
  }, [connect]);

  return { connected, lastEvent };
}
