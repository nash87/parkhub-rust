import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';

vi.mock('../api/client', () => ({
  getInMemoryToken: vi.fn(() => 'test-token'),
}));

import { useNotifications } from './useNotifications';

describe('useNotifications', () => {
  let mockPushManager: any;
  let mockRegistration: any;

  beforeEach(() => {
    vi.clearAllMocks();

    mockPushManager = {
      getSubscription: vi.fn().mockResolvedValue(null),
      subscribe: vi.fn().mockResolvedValue({
        toJSON: () => ({ endpoint: 'https://push.example.com', keys: { p256dh: 'key1', auth: 'key2' } }),
        unsubscribe: vi.fn().mockResolvedValue(true),
      }),
    };

    mockRegistration = { pushManager: mockPushManager };

    Object.defineProperty(globalThis, 'Notification', {
      value: { permission: 'default', requestPermission: vi.fn().mockResolvedValue('granted') },
      writable: true,
      configurable: true,
    });

    Object.defineProperty(navigator, 'serviceWorker', {
      value: { ready: Promise.resolve(mockRegistration) },
      writable: true,
      configurable: true,
    });

    Object.defineProperty(globalThis, 'PushManager', { value: class {}, writable: true, configurable: true });

    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/vapid-key')) return Promise.resolve({ ok: true, json: () => Promise.resolve({ data: { public_key: 'AAAA' } }) } as Response);
      if (url.includes('/subscribe')) return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true }) } as Response);
      if (url.includes('/unsubscribe')) return Promise.resolve({ ok: true } as Response);
      return Promise.resolve({ ok: true, json: () => Promise.resolve({}) } as Response);
    }) as any;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('detects supported environment', async () => {
    const { result } = renderHook(() => useNotifications());
    await waitFor(() => expect(result.current.supported).toBe(true));
    expect(result.current.permission).toBe('default');
  });

  it('subscribe succeeds', async () => {
    const { result } = renderHook(() => useNotifications());
    await act(async () => { await result.current.subscribe(); });
    await waitFor(() => {
      // If there's an error, the subscribe path hit a snag
      if (result.current.error) {
        throw new Error(`Subscribe failed with error: ${result.current.error}`);
      }
      expect(result.current.subscribed).toBe(true);
    });
    expect(result.current.loading).toBe(false);
  });

  it('subscribe denied permission', async () => {
    (Notification as any).requestPermission = vi.fn().mockResolvedValue('denied');
    const { result } = renderHook(() => useNotifications());
    await act(async () => { await result.current.subscribe(); });
    await waitFor(() => expect(result.current.error).toBe('Notification permission denied'));
    expect(result.current.loading).toBe(false);
  });

  it('subscribe vapid key fetch fails', async () => {
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/vapid-key')) return Promise.resolve({ ok: false } as Response);
      return Promise.resolve({ ok: true } as Response);
    }) as any;
    const { result } = renderHook(() => useNotifications());
    await act(async () => { await result.current.subscribe(); });
    await waitFor(() => expect(result.current.error).toBe('Push notifications not configured on server'));
  });

  it('subscribe empty vapid key', async () => {
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/vapid-key')) return Promise.resolve({ ok: true, json: () => Promise.resolve({ data: { public_key: null } }) } as Response);
      return Promise.resolve({ ok: true } as Response);
    }) as any;
    const { result } = renderHook(() => useNotifications());
    await act(async () => { await result.current.subscribe(); });
    await waitFor(() => expect(result.current.error).toBe('VAPID key not available'));
  });

  it('subscribe server registration fails', async () => {
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/vapid-key')) return Promise.resolve({ ok: true, json: () => Promise.resolve({ data: { public_key: 'BEl62iUYgUivxIkv69yViEuiBIa' } }) } as Response);
      if (url.includes('/subscribe')) return Promise.resolve({ ok: false } as Response);
      return Promise.resolve({ ok: true } as Response);
    }) as any;
    const { result } = renderHook(() => useNotifications());
    await act(async () => { await result.current.subscribe(); });
    await waitFor(() => expect(result.current.error).toBe('Failed to register subscription on server'));
  });

  it('subscribe catches exceptions', async () => {
    globalThis.fetch = vi.fn(() => Promise.reject(new Error('Network failure'))) as any;
    const { result } = renderHook(() => useNotifications());
    await act(async () => { await result.current.subscribe(); });
    await waitFor(() => expect(result.current.error).toBe('Network failure'));
  });

  it('unsubscribe succeeds', async () => {
    const mockSub = { unsubscribe: vi.fn().mockResolvedValue(true) };
    mockPushManager.getSubscription.mockResolvedValue(mockSub);
    const { result } = renderHook(() => useNotifications());
    await act(async () => { await result.current.unsubscribe(); });
    await waitFor(() => expect(result.current.subscribed).toBe(false));
    expect(result.current.loading).toBe(false);
  });

  it('unsubscribe with no existing subscription', async () => {
    mockPushManager.getSubscription.mockResolvedValue(null);
    const { result } = renderHook(() => useNotifications());
    await act(async () => { await result.current.unsubscribe(); });
    await waitFor(() => expect(result.current.subscribed).toBe(false));
  });

  it('unsubscribe catches exceptions', async () => {
    mockPushManager.getSubscription.mockRejectedValue(new Error('Unsub fail'));
    const { result } = renderHook(() => useNotifications());
    await act(async () => { await result.current.unsubscribe(); });
    await waitFor(() => expect(result.current.error).toBe('Unsub fail'));
  });

  it('detects already subscribed on mount when granted', async () => {
    (Notification as any).permission = 'granted';
    mockPushManager.getSubscription.mockResolvedValue({ endpoint: 'https://push.example.com' });
    const { result } = renderHook(() => useNotifications());
    await waitFor(() => expect(result.current.subscribed).toBe(true));
  });

  it('subscribe catches non-Error exceptions', async () => {
    (Notification as any).requestPermission = vi.fn().mockRejectedValue('string error');
    const { result } = renderHook(() => useNotifications());
    await act(async () => { await result.current.subscribe(); });
    await waitFor(() => expect(result.current.error).toBe('Unknown error'));
  });

  it('unsubscribe catches non-Error exceptions', async () => {
    mockPushManager.getSubscription.mockRejectedValue('string error');
    const { result } = renderHook(() => useNotifications());
    await act(async () => { await result.current.unsubscribe(); });
    await waitFor(() => expect(result.current.error).toBe('Unknown error'));
  });

  it('subscribe is a no-op when not supported', async () => {
    // Remove PushManager so supported is false
    delete (globalThis as any).PushManager;
    const { result } = renderHook(() => useNotifications());
    await waitFor(() => expect(result.current.supported).toBe(false));
    await act(async () => { await result.current.subscribe(); });
    expect(result.current.loading).toBe(false);
    expect(result.current.subscribed).toBe(false);
  });
});
