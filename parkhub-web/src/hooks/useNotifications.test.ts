import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useNotifications } from './useNotifications';

// Mock import.meta.env
vi.stubGlobal('import', { meta: { env: {} } });

describe('useNotifications', () => {
  const originalNotification = globalThis.Notification;
  const originalServiceWorker = globalThis.navigator?.serviceWorker;

  beforeEach(() => {
    vi.restoreAllMocks();
  });

  afterEach(() => {
    // Restore originals
    if (originalNotification) {
      Object.defineProperty(globalThis, 'Notification', { value: originalNotification, writable: true });
    }
    if (originalServiceWorker) {
      Object.defineProperty(globalThis.navigator, 'serviceWorker', { value: originalServiceWorker, writable: true, configurable: true });
    }
  });

  it('detects unsupported environment', () => {
    // Remove Notification API
    Object.defineProperty(globalThis, 'Notification', { value: undefined, writable: true });

    const { result } = renderHook(() => useNotifications());

    expect(result.current.supported).toBe(false);
    expect(result.current.subscribed).toBe(false);
  });

  it('detects supported environment with default permission', () => {
    const mockNotification = vi.fn() as any;
    mockNotification.permission = 'default';
    mockNotification.requestPermission = vi.fn();
    Object.defineProperty(globalThis, 'Notification', { value: mockNotification, writable: true });

    // Mock serviceWorker and PushManager
    const mockPushManager = {};
    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: {
        ready: Promise.resolve({
          pushManager: {
            getSubscription: vi.fn().mockResolvedValue(null),
          },
        }),
      },
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'PushManager', { value: mockPushManager, writable: true });

    const { result } = renderHook(() => useNotifications());

    expect(result.current.supported).toBe(true);
    expect(result.current.permission).toBe('default');
  });

  it('reports not subscribed initially', () => {
    const mockNotification = vi.fn() as any;
    mockNotification.permission = 'default';
    Object.defineProperty(globalThis, 'Notification', { value: mockNotification, writable: true });

    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: {
        ready: Promise.resolve({
          pushManager: {
            getSubscription: vi.fn().mockResolvedValue(null),
          },
        }),
      },
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'PushManager', { value: {}, writable: true });

    const { result } = renderHook(() => useNotifications());

    expect(result.current.subscribed).toBe(false);
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
  });

  it('provides subscribe and unsubscribe functions', () => {
    const mockNotification = vi.fn() as any;
    mockNotification.permission = 'default';
    Object.defineProperty(globalThis, 'Notification', { value: mockNotification, writable: true });

    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: {
        ready: Promise.resolve({
          pushManager: {
            getSubscription: vi.fn().mockResolvedValue(null),
          },
        }),
      },
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'PushManager', { value: {}, writable: true });

    const { result } = renderHook(() => useNotifications());

    expect(typeof result.current.subscribe).toBe('function');
    expect(typeof result.current.unsubscribe).toBe('function');
  });
});
