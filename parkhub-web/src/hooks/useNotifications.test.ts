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

  it('detects already-subscribed when permission is granted', async () => {
    const mockNotification = vi.fn() as any;
    mockNotification.permission = 'granted';
    mockNotification.requestPermission = vi.fn().mockResolvedValue('granted');
    Object.defineProperty(globalThis, 'Notification', { value: mockNotification, writable: true });

    const mockSubscription = { endpoint: 'https://push.example.com', toJSON: () => ({ endpoint: 'https://push.example.com', keys: { p256dh: 'key1', auth: 'key2' } }) };
    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: {
        ready: Promise.resolve({
          pushManager: {
            getSubscription: vi.fn().mockResolvedValue(mockSubscription),
            subscribe: vi.fn().mockResolvedValue(mockSubscription),
          },
        }),
      },
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'PushManager', { value: {}, writable: true });

    const { result } = renderHook(() => useNotifications());

    await vi.waitFor(() => {
      expect(result.current.subscribed).toBe(true);
    });
  });

  // Note: Testing subscribe when not supported requires modifying global browser APIs
  // which is restricted in JSDOM. The unsupported path is covered by the initial
  // "detects unsupported environment" test.

  it('subscribe sets error when permission denied', async () => {
    const mockNotification = vi.fn() as any;
    mockNotification.permission = 'default';
    mockNotification.requestPermission = vi.fn().mockResolvedValue('denied');
    Object.defineProperty(globalThis, 'Notification', { value: mockNotification, writable: true });

    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: {
        ready: Promise.resolve({
          pushManager: {
            getSubscription: vi.fn().mockResolvedValue(null),
            subscribe: vi.fn(),
          },
        }),
      },
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'PushManager', { value: {}, writable: true });

    const { result } = renderHook(() => useNotifications());
    await act(async () => {
      await result.current.subscribe();
    });

    expect(result.current.error).toBe('Notification permission denied');
    expect(result.current.loading).toBe(false);
  });

  it('subscribe sets error when VAPID key fetch fails', async () => {
    const mockNotification = vi.fn() as any;
    mockNotification.permission = 'default';
    mockNotification.requestPermission = vi.fn().mockResolvedValue('granted');
    Object.defineProperty(globalThis, 'Notification', { value: mockNotification, writable: true });

    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: {
        ready: Promise.resolve({
          pushManager: {
            getSubscription: vi.fn().mockResolvedValue(null),
            subscribe: vi.fn(),
          },
        }),
      },
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'PushManager', { value: {}, writable: true });

    global.fetch = vi.fn().mockResolvedValue({ ok: false } as Response);

    const { result } = renderHook(() => useNotifications());
    await act(async () => {
      await result.current.subscribe();
    });

    expect(result.current.error).toBe('Push notifications not configured on server');
    expect(result.current.loading).toBe(false);
  });

  it('subscribe sets error when VAPID key is missing', async () => {
    const mockNotification = vi.fn() as any;
    mockNotification.permission = 'default';
    mockNotification.requestPermission = vi.fn().mockResolvedValue('granted');
    Object.defineProperty(globalThis, 'Notification', { value: mockNotification, writable: true });

    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: {
        ready: Promise.resolve({
          pushManager: {
            getSubscription: vi.fn().mockResolvedValue(null),
            subscribe: vi.fn(),
          },
        }),
      },
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'PushManager', { value: {}, writable: true });

    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ data: { public_key: null } }),
    } as any);

    const { result } = renderHook(() => useNotifications());
    await act(async () => {
      await result.current.subscribe();
    });

    expect(result.current.error).toBe('VAPID key not available');
    expect(result.current.loading).toBe(false);
  });

  it('subscribe completes full flow successfully', async () => {
    const mockNotification = vi.fn() as any;
    mockNotification.permission = 'default';
    mockNotification.requestPermission = vi.fn().mockResolvedValue('granted');
    Object.defineProperty(globalThis, 'Notification', { value: mockNotification, writable: true });

    const mockSubscription = {
      endpoint: 'https://push.example.com/sub1',
      toJSON: () => ({ endpoint: 'https://push.example.com/sub1', keys: { p256dh: 'key-p256dh', auth: 'key-auth' } }),
    };

    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: {
        ready: Promise.resolve({
          pushManager: {
            getSubscription: vi.fn().mockResolvedValue(null),
            subscribe: vi.fn().mockResolvedValue(mockSubscription),
          },
        }),
      },
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'PushManager', { value: {}, writable: true });

    // Mock the dynamic import for getInMemoryToken
    vi.doMock('../api/client', () => ({
      getInMemoryToken: () => 'test-token-123',
    }));

    let callCount = 0;
    global.fetch = vi.fn().mockImplementation(() => {
      callCount++;
      if (callCount === 1) {
        // VAPID key response
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({ data: { public_key: 'BNcRd-base64url-key-here' } }),
        });
      }
      // Subscribe response
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true }) });
    });

    const { result } = renderHook(() => useNotifications());
    await act(async () => {
      await result.current.subscribe();
    });

    expect(result.current.subscribed).toBe(true);
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();

    vi.doUnmock('../api/client');
  });

  it('subscribe handles server registration failure', async () => {
    const mockNotification = vi.fn() as any;
    mockNotification.permission = 'default';
    mockNotification.requestPermission = vi.fn().mockResolvedValue('granted');
    Object.defineProperty(globalThis, 'Notification', { value: mockNotification, writable: true });

    const mockSubscription = {
      endpoint: 'https://push.example.com/sub1',
      toJSON: () => ({ endpoint: 'https://push.example.com/sub1', keys: { p256dh: 'key-p256dh', auth: 'key-auth' } }),
    };

    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: {
        ready: Promise.resolve({
          pushManager: {
            getSubscription: vi.fn().mockResolvedValue(null),
            subscribe: vi.fn().mockResolvedValue(mockSubscription),
          },
        }),
      },
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'PushManager', { value: {}, writable: true });

    vi.doMock('../api/client', () => ({
      getInMemoryToken: () => null,
    }));

    let callCount = 0;
    global.fetch = vi.fn().mockImplementation(() => {
      callCount++;
      if (callCount === 1) {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({ data: { public_key: 'BNcRd-base64url-key-here' } }),
        });
      }
      // Server registration returns not-ok
      return Promise.resolve({ ok: false, json: () => Promise.resolve({ error: 'Registration failed' }) });
    });

    const { result } = renderHook(() => useNotifications());
    await act(async () => {
      await result.current.subscribe();
    });

    expect(result.current.subscribed).toBe(false);
    expect(result.current.error).toBe('Failed to register subscription on server');
    expect(result.current.loading).toBe(false);

    vi.doUnmock('../api/client');
  });

  it('subscribe handles unexpected errors', async () => {
    const mockNotification = vi.fn() as any;
    mockNotification.permission = 'default';
    mockNotification.requestPermission = vi.fn().mockRejectedValue(new Error('Browser blocked'));
    Object.defineProperty(globalThis, 'Notification', { value: mockNotification, writable: true });

    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: {
        ready: Promise.resolve({
          pushManager: {
            getSubscription: vi.fn().mockResolvedValue(null),
            subscribe: vi.fn(),
          },
        }),
      },
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'PushManager', { value: {}, writable: true });

    const { result } = renderHook(() => useNotifications());
    await act(async () => {
      await result.current.subscribe();
    });

    expect(result.current.error).toBe('Browser blocked');
    expect(result.current.loading).toBe(false);
  });

  it('subscribe handles non-Error thrown objects', async () => {
    const mockNotification = vi.fn() as any;
    mockNotification.permission = 'default';
    mockNotification.requestPermission = vi.fn().mockRejectedValue('string error');
    Object.defineProperty(globalThis, 'Notification', { value: mockNotification, writable: true });

    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: {
        ready: Promise.resolve({
          pushManager: {
            getSubscription: vi.fn().mockResolvedValue(null),
            subscribe: vi.fn(),
          },
        }),
      },
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'PushManager', { value: {}, writable: true });

    const { result } = renderHook(() => useNotifications());
    await act(async () => {
      await result.current.subscribe();
    });

    expect(result.current.error).toBe('Unknown error');
    expect(result.current.loading).toBe(false);
  });

  it('unsubscribe completes full flow', async () => {
    const mockUnsubscribe = vi.fn().mockResolvedValue(true);
    const mockSubscription = {
      endpoint: 'https://push.example.com/sub1',
      unsubscribe: mockUnsubscribe,
    };
    const mockNotification = vi.fn() as any;
    mockNotification.permission = 'granted';
    mockNotification.requestPermission = vi.fn().mockResolvedValue('granted');
    Object.defineProperty(globalThis, 'Notification', { value: mockNotification, writable: true });

    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: {
        ready: Promise.resolve({
          pushManager: {
            getSubscription: vi.fn().mockResolvedValue(mockSubscription),
          },
        }),
      },
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'PushManager', { value: {}, writable: true });

    vi.doMock('../api/client', () => ({
      getInMemoryToken: () => 'tok-123',
    }));

    global.fetch = vi.fn().mockResolvedValue({ ok: true, json: () => Promise.resolve({ success: true }) });

    const { result } = renderHook(() => useNotifications());

    await act(async () => {
      await result.current.unsubscribe();
    });

    expect(mockUnsubscribe).toHaveBeenCalled();
    expect(result.current.subscribed).toBe(false);
    expect(result.current.loading).toBe(false);

    vi.doUnmock('../api/client');
  });

  it('unsubscribe works even when no existing subscription', async () => {
    const mockNotification = vi.fn() as any;
    mockNotification.permission = 'granted';
    Object.defineProperty(globalThis, 'Notification', { value: mockNotification, writable: true });

    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: {
        ready: Promise.resolve({
          pushManager: {
            getSubscription: vi.fn().mockResolvedValue(null), // no subscription
          },
        }),
      },
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'PushManager', { value: {}, writable: true });

    vi.doMock('../api/client', () => ({
      getInMemoryToken: () => null,
    }));

    global.fetch = vi.fn().mockResolvedValue({ ok: true });

    const { result } = renderHook(() => useNotifications());

    await act(async () => {
      await result.current.unsubscribe();
    });

    expect(result.current.subscribed).toBe(false);
    expect(result.current.loading).toBe(false);

    vi.doUnmock('../api/client');
  });

  it('unsubscribe handles errors', async () => {
    const mockNotification = vi.fn() as any;
    mockNotification.permission = 'default'; // Use 'default' so init effect doesn't call getSubscription
    Object.defineProperty(globalThis, 'Notification', { value: mockNotification, writable: true });

    // Create a rejecting promise but add a .catch to prevent unhandled rejection
    const rejectPromise = Promise.reject(new Error('SW not available'));
    rejectPromise.catch(() => {}); // prevent unhandled rejection in test env

    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: {
        ready: rejectPromise,
      },
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'PushManager', { value: {}, writable: true });

    const { result } = renderHook(() => useNotifications());

    await act(async () => {
      await result.current.unsubscribe();
    });

    expect(result.current.error).toBe('SW not available');
    expect(result.current.loading).toBe(false);
  });

  it('unsubscribe handles non-Error thrown objects', async () => {
    const mockNotification = vi.fn() as any;
    mockNotification.permission = 'default'; // Use 'default' so init effect doesn't call getSubscription
    Object.defineProperty(globalThis, 'Notification', { value: mockNotification, writable: true });

    // Create a rejecting promise but add a .catch to prevent unhandled rejection
    const rejectPromise = Promise.reject('plain string error');
    rejectPromise.catch(() => {}); // prevent unhandled rejection in test env

    Object.defineProperty(globalThis.navigator, 'serviceWorker', {
      value: {
        ready: rejectPromise,
      },
      writable: true,
      configurable: true,
    });
    Object.defineProperty(globalThis, 'PushManager', { value: {}, writable: true });

    const { result } = renderHook(() => useNotifications());

    await act(async () => {
      await result.current.unsubscribe();
    });

    expect(result.current.error).toBe('Unknown error');
    expect(result.current.loading).toBe(false);
  });
});
