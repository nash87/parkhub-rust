import { useEffect, useRef, useCallback, useState } from 'react';

export type WsEventType =
  | 'booking_created'
  | 'booking_cancelled'
  | 'occupancy_changed'
  | 'announcement_published'
  | 'slot_status_change';

export interface WsEvent {
  event: WsEventType;
  data: Record<string, unknown>;
  timestamp: string;
}

export type WsEventHandler = (event: WsEvent) => void;

/** Per-lot occupancy snapshot, kept up-to-date by WebSocket events. */
export interface OccupancyMap {
  [lotId: string]: { available: number; total: number };
}

interface UseWebSocketOptions {
  url?: string;
  autoReconnect?: boolean;
  reconnectDelay?: number;
  maxReconnectDelay?: number;
  onEvent?: WsEventHandler;
  /** Auth token appended as `?token=...` query parameter. */
  token?: string;
}

export function useWebSocket(options: UseWebSocketOptions = {}) {
  const {
    autoReconnect = true,
    reconnectDelay = 1000,
    maxReconnectDelay = 30_000,
    onEvent,
    token,
  } = options;

  const [connected, setConnected] = useState(false);
  const [lastMessage, setLastMessage] = useState<WsEvent | null>(null);
  const [occupancy, setOccupancy] = useState<OccupancyMap>({});

  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const retryCount = useRef(0);
  const onEventRef = useRef(onEvent);
  const unmountedRef = useRef(false);

  onEventRef.current = onEvent;

  const getWsUrl = useCallback(() => {
    if (options.url) return options.url;
    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    let url = `${proto}//${window.location.host}/api/v1/ws`;
    if (token) {
      url += `?token=${encodeURIComponent(token)}`;
    }
    return url;
  }, [options.url, token]);

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
        setLastMessage(event);
        onEventRef.current?.(event);

        // Update occupancy map from occupancy_changed events
        if (event.event === 'occupancy_changed' && event.data.lot_id) {
          setOccupancy(prev => ({
            ...prev,
            [event.data.lot_id as string]: {
              available: event.data.available as number,
              total: event.data.total as number,
            },
          }));
        }
      } catch {
        // Ignore non-JSON messages (e.g., pong frames)
      }
    };

    ws.onclose = () => {
      setConnected(false);
      wsRef.current = null;
      if (autoReconnect && !unmountedRef.current) {
        const delay = Math.min(reconnectDelay * Math.pow(2, retryCount.current), maxReconnectDelay);
        retryCount.current += 1;
        reconnectTimer.current = setTimeout(connect, delay);
      }
    };

    ws.onerror = () => {};
  }, [getWsUrl, autoReconnect, reconnectDelay, maxReconnectDelay]);

  useEffect(() => {
    unmountedRef.current = false;
    connect();
    return () => {
      unmountedRef.current = true;
      if (reconnectTimer.current) clearTimeout(reconnectTimer.current);
      if (wsRef.current) { wsRef.current.close(); wsRef.current = null; }
    };
  }, [connect]);

  return { connected, lastMessage, occupancy };
}
