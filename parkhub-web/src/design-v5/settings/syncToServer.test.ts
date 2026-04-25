import { describe, expect, it, vi, beforeEach } from 'vitest';
import { DEFAULT_SETTINGS } from './settings';
import { syncSettingsToServer } from './syncToServer';
import { api } from '../../api/client';

vi.mock('../../api/client', () => ({
  api: {
    updateSettings: vi.fn(),
  },
}));

describe('syncSettingsToServer', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('resolves when the API reports success', async () => {
    vi.mocked(api.updateSettings).mockResolvedValue({
      success: true,
      data: DEFAULT_SETTINGS as unknown as Record<string, unknown>,
    });
    await expect(syncSettingsToServer(DEFAULT_SETTINGS)).resolves.toBeDefined();
  });

  it('throws when the API resolves with success=false', async () => {
    vi.mocked(api.updateSettings).mockResolvedValue({
      success: false,
      data: null,
      error: { code: 'PAYLOAD_TOO_LARGE', message: 'too big' },
    });
    await expect(syncSettingsToServer(DEFAULT_SETTINGS)).rejects.toThrow(
      /PAYLOAD_TOO_LARGE/,
    );
  });

  it('propagates network rejections from updateSettings', async () => {
    vi.mocked(api.updateSettings).mockRejectedValue(new Error('network down'));
    await expect(syncSettingsToServer(DEFAULT_SETTINGS)).rejects.toThrow(
      /network down/,
    );
  });
});
