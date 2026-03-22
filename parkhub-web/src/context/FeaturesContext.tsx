import { createContext, useContext, useEffect, useState, useCallback, type ReactNode } from 'react';
import { useUseCase, type UseCase } from './UseCaseContext';

// ── Feature Module Definitions ──

export type FeatureModule =
  | 'credits'
  | 'absences'
  | 'vehicles'
  | 'analytics'
  | 'team_view'
  | 'booking_types'
  | 'invoices'
  | 'self_registration'
  | 'generative_bg'
  | 'micro_animations'
  | 'fab_quick_actions'
  | 'rich_empty_states'
  | 'onboarding_hints'
  | 'themes';

export type FeatureCategory = 'core' | 'collaboration' | 'billing' | 'admin' | 'experience';

export interface FeatureInfo {
  id: FeatureModule;
  category: FeatureCategory;
  defaultEnabled: boolean;
}

/** Master registry of all feature modules with metadata */
export const FEATURE_REGISTRY: FeatureInfo[] = [
  // Core
  { id: 'vehicles',          category: 'core',          defaultEnabled: true },
  { id: 'booking_types',     category: 'core',          defaultEnabled: true },
  // Collaboration
  { id: 'absences',          category: 'collaboration', defaultEnabled: true },
  { id: 'team_view',         category: 'collaboration', defaultEnabled: true },
  // Billing
  { id: 'credits',           category: 'billing',       defaultEnabled: true },
  { id: 'invoices',          category: 'billing',       defaultEnabled: false },
  // Admin
  { id: 'analytics',         category: 'admin',         defaultEnabled: true },
  { id: 'self_registration', category: 'admin',         defaultEnabled: false },
  // Experience — UI/UX enhancements
  { id: 'generative_bg',     category: 'experience',    defaultEnabled: true },
  { id: 'micro_animations',  category: 'experience',    defaultEnabled: true },
  { id: 'fab_quick_actions', category: 'experience',    defaultEnabled: true },
  { id: 'rich_empty_states', category: 'experience',    defaultEnabled: true },
  { id: 'onboarding_hints',  category: 'experience',    defaultEnabled: false },
  { id: 'themes',            category: 'experience',    defaultEnabled: true },
];

/** Use-case presets — which features are enabled by default per use case */
export const USE_CASE_PRESETS: Record<UseCase, FeatureModule[]> = {
  business: ['credits', 'absences', 'vehicles', 'analytics', 'team_view', 'booking_types', 'invoices', 'generative_bg', 'micro_animations', 'fab_quick_actions', 'rich_empty_states', 'onboarding_hints', 'themes'],
  residential: ['vehicles', 'booking_types', 'self_registration', 'generative_bg', 'micro_animations', 'rich_empty_states', 'themes'],
  personal: ['vehicles', 'booking_types', 'generative_bg', 'micro_animations', 'fab_quick_actions', 'themes'],
};

const STORAGE_KEY = 'parkhub_features';

// ── Context ──

interface FeaturesState {
  /** Currently enabled feature modules */
  features: FeatureModule[];
  /** Check if a specific feature is enabled */
  isEnabled: (feature: FeatureModule) => boolean;
  /** Enable or disable a feature */
  setFeature: (feature: FeatureModule, enabled: boolean) => void;
  /** Bulk-set features (e.g. from preset) */
  setFeatures: (features: FeatureModule[]) => void;
  /** Apply a use-case preset */
  applyPreset: (useCase: UseCase) => void;
  /** Whether features have been configured (not just defaults) */
  configured: boolean;
  /** Loading state */
  loading: boolean;
}

const FeaturesContext = createContext<FeaturesState | null>(null);

export function FeaturesProvider({ children }: { children: ReactNode }) {
  const { useCase } = useUseCase();
  const [features, setFeaturesState] = useState<FeatureModule[]>(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) {
      try { return JSON.parse(stored); } catch { /* fall through */ }
    }
    return USE_CASE_PRESETS[useCase] || USE_CASE_PRESETS.business;
  });
  const [configured, setConfigured] = useState(() => localStorage.getItem(STORAGE_KEY) !== null);
  const [loading, setLoading] = useState(true);

  // Load from localStorage on mount (features are client-side only for now)
  useEffect(() => {
    setLoading(false);
  }, []);

  const persist = useCallback((f: FeatureModule[]) => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(f));
  }, []);

  const isEnabled = useCallback((feature: FeatureModule) => features.includes(feature), [features]);

  const setFeature = useCallback((feature: FeatureModule, enabled: boolean) => {
    setFeaturesState(prev => {
      const next = enabled
        ? [...new Set([...prev, feature])]
        : prev.filter(f => f !== feature);
      persist(next);
      return next;
    });
    setConfigured(true);
  }, [persist]);

  const setFeatures = useCallback((f: FeatureModule[]) => {
    setFeaturesState(f);
    persist(f);
    setConfigured(true);
  }, [persist]);

  const applyPreset = useCallback((uc: UseCase) => {
    const preset = USE_CASE_PRESETS[uc] || USE_CASE_PRESETS.business;
    setFeaturesState(preset);
    persist(preset);
    setConfigured(true);
  }, [persist]);

  return (
    <FeaturesContext.Provider value={{ features, isEnabled, setFeature, setFeatures, applyPreset, configured, loading }}>
      {children}
    </FeaturesContext.Provider>
  );
}

export function useFeatures() {
  const ctx = useContext(FeaturesContext);
  if (!ctx) throw new Error('useFeatures must be used within FeaturesProvider');
  return ctx;
}
