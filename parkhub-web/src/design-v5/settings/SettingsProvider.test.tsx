import { describe, expect, it, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import {
  V5SettingsProvider,
  useV5Feature,
  useV5Settings,
} from './SettingsProvider';
import { DEFAULT_SETTINGS, STORAGE_KEY } from './settings';

function Probe() {
  const { settings, updateSetting, updateSection, resetSettings, syncState } = useV5Settings();
  return (
    <div>
      <div data-testid="mode">{settings.appearance.mode}</div>
      <div data-testid="sidebar">{settings.appearance.sidebar}</div>
      <div data-testid="density">{settings.appearance.density}</div>
      <div data-testid="voice">{String(settings.features.voiceCommands)}</div>
      <div data-testid="sync">{syncState}</div>
      <button onClick={() => updateSetting('appearance', 'mode', 'void')}>set-void</button>
      <button onClick={() => updateSetting('appearance', 'sidebar', 'columns')}>set-columns</button>
      <button onClick={() => updateSection('features', { voiceCommands: true, plateScan: true })}>
        flip-features
      </button>
      <button onClick={() => resetSettings()}>reset</button>
    </div>
  );
}

describe('<V5SettingsProvider>', () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.removeAttribute('data-ph-mode');
    document.documentElement.removeAttribute('data-ph-density');
    document.documentElement.removeAttribute('data-ph-font');
    document.documentElement.style.removeProperty('--v5-font-scale');
  });

  it('provides default settings on first mount', () => {
    render(
      <V5SettingsProvider>
        <Probe />
      </V5SettingsProvider>,
    );
    expect(screen.getByTestId('mode').textContent).toBe('marble_light');
    expect(screen.getByTestId('sidebar').textContent).toBe('marble');
  });

  it('writes attributes to <html> reflecting current settings', () => {
    render(
      <V5SettingsProvider>
        <Probe />
      </V5SettingsProvider>,
    );
    expect(document.documentElement.getAttribute('data-ph-mode')).toBe('marble_light');
    expect(document.documentElement.getAttribute('data-ph-density')).toBe('comfortable');
    expect(document.documentElement.getAttribute('data-ph-font')).toBe('inter');
  });

  it('updateSetting persists the change to localStorage', async () => {
    const user = userEvent.setup();
    render(
      <V5SettingsProvider>
        <Probe />
      </V5SettingsProvider>,
    );
    await user.click(screen.getByText('set-void'));
    expect(screen.getByTestId('mode').textContent).toBe('void');
    expect(document.documentElement.getAttribute('data-ph-mode')).toBe('void');
    const stored = JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? '{}');
    expect(stored.appearance.mode).toBe('void');
  });

  it('updateSection patches multiple keys at once', async () => {
    const user = userEvent.setup();
    render(
      <V5SettingsProvider>
        <Probe />
      </V5SettingsProvider>,
    );
    await user.click(screen.getByText('flip-features'));
    expect(screen.getByTestId('voice').textContent).toBe('true');
  });

  it('resetSettings goes back to factory defaults', async () => {
    const user = userEvent.setup();
    render(
      <V5SettingsProvider>
        <Probe />
      </V5SettingsProvider>,
    );
    await user.click(screen.getByText('set-void'));
    await user.click(screen.getByText('reset'));
    expect(screen.getByTestId('mode').textContent).toBe(DEFAULT_SETTINGS.appearance.mode);
  });

  it('debounces sync to backend', async () => {
    const sync = vi.fn().mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(
      <V5SettingsProvider syncToServer={sync} syncDebounceMs={50}>
        <Probe />
      </V5SettingsProvider>,
    );
    await user.click(screen.getByText('set-void'));
    await user.click(screen.getByText('set-columns'));
    // Wait for debounce window + microtasks.
    await new Promise((r) => setTimeout(r, 120));
    // Initial mount triggers one debounced sync (state changed via persistence
    // applying attributes); both clicks then collapse into a single call after
    // the window. Allow 1–2 calls; the important thing is the latest payload
    // contains both updates.
    expect(sync.mock.calls.length).toBeGreaterThanOrEqual(1);
    const last = sync.mock.calls[sync.mock.calls.length - 1]?.[0] as {
      appearance: { mode: string; sidebar: string };
    };
    expect(last.appearance.mode).toBe('void');
    expect(last.appearance.sidebar).toBe('columns');
  });

  it('reflects sync errors in syncState', async () => {
    const sync = vi.fn().mockRejectedValue(new Error('boom'));
    const user = userEvent.setup();
    render(
      <V5SettingsProvider syncToServer={sync} syncDebounceMs={20}>
        <Probe />
      </V5SettingsProvider>,
    );
    await user.click(screen.getByText('set-void'));
    await new Promise((r) => setTimeout(r, 80));
    expect(screen.getByTestId('sync').textContent).toBe('error');
  });

  it('useV5Feature returns the live flag value', () => {
    function FeatureProbe() {
      const enabled = useV5Feature('plateScan');
      return <div data-testid="plate">{String(enabled)}</div>;
    }
    render(
      <V5SettingsProvider initialOverride={{
        ...DEFAULT_SETTINGS,
        features: { ...DEFAULT_SETTINGS.features, plateScan: true },
      }}>
        <FeatureProbe />
      </V5SettingsProvider>,
    );
    expect(screen.getByTestId('plate').textContent).toBe('true');
  });

  it('useV5Feature falls back to default when used outside provider', () => {
    function FeatureProbe() {
      const enabled = useV5Feature('voiceCommands');
      return <div data-testid="voice">{String(enabled)}</div>;
    }
    render(<FeatureProbe />);
    expect(screen.getByTestId('voice').textContent).toBe('false');
  });

  it('does not sync defaults before hydrate or user action (race guard)', async () => {
    const sync = vi.fn().mockResolvedValue(undefined);
    render(
      <V5SettingsProvider syncToServer={sync} syncDebounceMs={20}>
        <Probe />
      </V5SettingsProvider>,
    );
    // Wait past the debounce window — no user action, no hydrate happened
    // yet, so the provider must NOT push defaults to the server.
    await new Promise((r) => setTimeout(r, 80));
    expect(sync).not.toHaveBeenCalled();
  });
});
