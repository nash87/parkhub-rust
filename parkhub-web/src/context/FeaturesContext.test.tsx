import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Hoisted localStorage mock ──
const { localStorageMock } = vi.hoisted(() => {
  let store: Record<string, string> = {};
  const localStorageMock = {
    getItem: vi.fn((key: string) => store[key] ?? null),
    setItem: vi.fn((key: string, val: string) => { store[key] = val; }),
    removeItem: vi.fn((key: string) => { delete store[key]; }),
    clear: vi.fn(() => { store = {}; }),
  };

  Object.defineProperty(globalThis.window ?? globalThis, 'localStorage', {
    value: localStorageMock, writable: true, configurable: true,
  });

  return { localStorageMock };
});

// Mock UseCaseContext (FeaturesContext depends on it)
vi.mock('./UseCaseContext', () => ({
  useUseCase: () => ({ useCase: 'business', setUseCase: vi.fn(), hasChosen: true }),
}));

import {
  FeaturesProvider,
  useFeatures,
  FEATURE_REGISTRY,
  USE_CASE_PRESETS,
  type FeatureModule,
} from './FeaturesContext';

// Helper component to consume the context
function FeaturesConsumer() {
  const { features, isEnabled, setFeature, setFeatures, applyPreset, configured, loading } = useFeatures();
  return (
    <div>
      <span data-testid="loading">{String(loading)}</span>
      <span data-testid="configured">{String(configured)}</span>
      <span data-testid="count">{features.length}</span>
      <span data-testid="credits">{String(isEnabled('credits'))}</span>
      <span data-testid="invoices">{String(isEnabled('invoices'))}</span>
      <span data-testid="vehicles">{String(isEnabled('vehicles'))}</span>
      <button data-testid="enable-invoices" onClick={() => setFeature('invoices', true)}>Enable Invoices</button>
      <button data-testid="disable-credits" onClick={() => setFeature('credits', false)}>Disable Credits</button>
      <button data-testid="apply-personal" onClick={() => applyPreset('personal')}>Apply Personal</button>
      <button data-testid="apply-residential" onClick={() => applyPreset('residential')}>Apply Residential</button>
      <button data-testid="set-minimal" onClick={() => setFeatures(['vehicles'])}>Set Minimal</button>
    </div>
  );
}

describe('FeaturesContext', () => {
  beforeEach(() => {
    localStorageMock.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('useFeatures throws outside FeaturesProvider', () => {
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});
    expect(() => render(<FeaturesConsumer />)).toThrow(
      'useFeatures must be used within FeaturesProvider',
    );
    spy.mockRestore();
  });

  it('defaults to business preset features when no localStorage', async () => {
    render(
      <FeaturesProvider>
        <FeaturesConsumer />
      </FeaturesProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('loading').textContent).toBe('false');
    });

    // Business preset includes credits
    expect(screen.getByTestId('credits').textContent).toBe('true');
    expect(screen.getByTestId('vehicles').textContent).toBe('true');
    expect(screen.getByTestId('configured').textContent).toBe('false');
  });

  it('reads features from localStorage', async () => {
    localStorageMock.setItem('parkhub_features', JSON.stringify(['vehicles', 'credits']));

    render(
      <FeaturesProvider>
        <FeaturesConsumer />
      </FeaturesProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('loading').textContent).toBe('false');
    });

    expect(screen.getByTestId('count').textContent).toBe('2');
    expect(screen.getByTestId('configured').textContent).toBe('true');
  });

  it('setFeature enables a feature and persists', async () => {
    const user = userEvent.setup();

    render(
      <FeaturesProvider>
        <FeaturesConsumer />
      </FeaturesProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('loading').textContent).toBe('false');
    });

    // Invoices is off by default in business (it's in the list but check)
    // Actually business preset includes invoices, let's check the preset
    await user.click(screen.getByTestId('enable-invoices'));

    expect(screen.getByTestId('invoices').textContent).toBe('true');
    expect(screen.getByTestId('configured').textContent).toBe('true');
    expect(localStorageMock.setItem).toHaveBeenCalledWith(
      'parkhub_features',
      expect.any(String),
    );
  });

  it('setFeature disables a feature and persists', async () => {
    const user = userEvent.setup();

    render(
      <FeaturesProvider>
        <FeaturesConsumer />
      </FeaturesProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('loading').textContent).toBe('false');
    });

    expect(screen.getByTestId('credits').textContent).toBe('true');

    await user.click(screen.getByTestId('disable-credits'));
    expect(screen.getByTestId('credits').textContent).toBe('false');
  });

  it('applyPreset replaces features with preset values', async () => {
    const user = userEvent.setup();

    render(
      <FeaturesProvider>
        <FeaturesConsumer />
      </FeaturesProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('loading').textContent).toBe('false');
    });

    await user.click(screen.getByTestId('apply-personal'));

    // Personal preset is smaller than business
    const personalCount = USE_CASE_PRESETS.personal.length;
    expect(screen.getByTestId('count').textContent).toBe(String(personalCount));
    expect(screen.getByTestId('configured').textContent).toBe('true');
  });

  it('setFeatures bulk-sets features', async () => {
    const user = userEvent.setup();

    render(
      <FeaturesProvider>
        <FeaturesConsumer />
      </FeaturesProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('loading').textContent).toBe('false');
    });

    await user.click(screen.getByTestId('set-minimal'));
    expect(screen.getByTestId('count').textContent).toBe('1');
    expect(screen.getByTestId('vehicles').textContent).toBe('true');
    expect(screen.getByTestId('credits').textContent).toBe('false');
  });

  it('handles corrupted localStorage gracefully', async () => {
    localStorageMock.setItem('parkhub_features', '{invalid json}');

    render(
      <FeaturesProvider>
        <FeaturesConsumer />
      </FeaturesProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('loading').textContent).toBe('false');
    });

    // Falls back to business preset
    const businessCount = USE_CASE_PRESETS.business.length;
    expect(screen.getByTestId('count').textContent).toBe(String(businessCount));
  });
});

describe('FEATURE_REGISTRY', () => {
  it('contains all expected feature modules', () => {
    const ids = FEATURE_REGISTRY.map(f => f.id);
    expect(ids).toContain('credits');
    expect(ids).toContain('absences');
    expect(ids).toContain('vehicles');
    expect(ids).toContain('analytics');
    expect(ids).toContain('themes');
    expect(ids).toContain('history');
    expect(ids).toContain('geofence');
  });

  it('every entry has required fields', () => {
    for (const feature of FEATURE_REGISTRY) {
      expect(feature.id).toBeTruthy();
      expect(feature.category).toBeTruthy();
      expect(typeof feature.defaultEnabled).toBe('boolean');
    }
  });

  it('categories are valid', () => {
    const validCategories = ['core', 'collaboration', 'billing', 'admin', 'experience'];
    for (const feature of FEATURE_REGISTRY) {
      expect(validCategories).toContain(feature.category);
    }
  });
});

describe('USE_CASE_PRESETS', () => {
  it('defines presets for all three use cases', () => {
    expect(USE_CASE_PRESETS.business).toBeDefined();
    expect(USE_CASE_PRESETS.residential).toBeDefined();
    expect(USE_CASE_PRESETS.personal).toBeDefined();
  });

  it('business preset is the most feature-rich', () => {
    expect(USE_CASE_PRESETS.business.length).toBeGreaterThanOrEqual(USE_CASE_PRESETS.residential.length);
    expect(USE_CASE_PRESETS.business.length).toBeGreaterThanOrEqual(USE_CASE_PRESETS.personal.length);
  });

  it('all presets include vehicles', () => {
    expect(USE_CASE_PRESETS.business).toContain('vehicles');
    expect(USE_CASE_PRESETS.residential).toContain('vehicles');
    expect(USE_CASE_PRESETS.personal).toContain('vehicles');
  });

  it('only business preset includes absences', () => {
    expect(USE_CASE_PRESETS.business).toContain('absences');
    expect(USE_CASE_PRESETS.residential).not.toContain('absences');
    expect(USE_CASE_PRESETS.personal).not.toContain('absences');
  });
});
