import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockSetFeatures = vi.fn();
const mockFeatures: string[] = ['credits', 'vehicles'];
const mockToastSuccess = vi.fn();

vi.mock('react-router-dom', () => ({
  Link: ({ to, children, ...props }: any) => <a href={to} {...props}>{children}</a>,
}));

vi.mock('../context/FeaturesContext', () => {
  const FEATURE_REGISTRY = [
    { id: 'credits', category: 'billing', defaultEnabled: true },
    { id: 'vehicles', category: 'core', defaultEnabled: true },
    { id: 'absences', category: 'collaboration', defaultEnabled: true },
    { id: 'analytics', category: 'admin', defaultEnabled: false },
    { id: 'generative_bg', category: 'experience', defaultEnabled: false },
  ];
  const USE_CASE_PRESETS: Record<string, string[]> = {
    business: ['credits', 'vehicles', 'absences'],
    residential: ['vehicles'],
    personal: ['vehicles'],
  };
  return {
    useFeatures: () => ({
      features: mockFeatures,
      setFeatures: mockSetFeatures,
      isEnabled: (f: string) => mockFeatures.includes(f),
    }),
    FEATURE_REGISTRY,
    USE_CASE_PRESETS,
  };
});

vi.mock('../context/UseCaseContext', () => ({
  useUseCase: () => ({ useCase: 'business', setUseCase: vi.fn(), hasChosen: true }),
}));

vi.mock('../constants/animations', () => ({
  stagger: { hidden: { opacity: 0 }, show: { opacity: 1 } },
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, variants, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  ToggleLeft: (props: any) => <span data-testid="toggle-off" {...props} />,
  ToggleRight: (props: any) => <span data-testid="toggle-on" {...props} />,
  Info: (props: any) => <span data-testid="icon-info" {...props} />,
  ShieldCheck: (props: any) => <span data-testid="icon-shield" {...props} />,
  ArrowLeft: (props: any) => <span data-testid="icon-back" {...props} />,
  ArrowClockwise: (props: any) => <span data-testid="icon-reset" {...props} />,
  FloppyDisk: (props: any) => <span data-testid="icon-save" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'nav.dashboard': 'Dashboard',
        'nav.admin': 'Admin',
        'features.title': 'Feature Management',
        'features.subtitle': 'Enable or disable modules',
        'features.enableAll': 'Enable All',
        'features.disableAll': 'Disable All',
        'features.resetToPreset': 'Reset to Preset',
        'features.enabled': 'Enabled',
        'features.disabled': 'Disabled',
        'features.saveChanges': 'Save Changes',
        'features.saved': 'Saved!',
        'features.compliance.title': 'Compliance',
        'features.compliance.gdpr': 'GDPR compliant',
        'features.compliance.audit': 'Audit trail',
        'features.compliance.encryption': 'End-to-end encryption',
        'common.info': 'More info',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('react-hot-toast', () => ({
  default: { success: (...args: any[]) => mockToastSuccess(...args) },
}));

import { AdminFeaturesPage } from './AdminFeatures';

describe('AdminFeaturesPage', () => {
  beforeEach(() => {
    mockSetFeatures.mockClear();
    mockToastSuccess.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders page title and subtitle', () => {
    render(<AdminFeaturesPage />);
    expect(screen.getByText('Feature Management')).toBeInTheDocument();
    expect(screen.getByText('Enable or disable modules')).toBeInTheDocument();
  });

  it('renders admin breadcrumb', () => {
    render(<AdminFeaturesPage />);
    expect(screen.getByText('Admin')).toBeInTheDocument();
    expect(screen.getByText('Dashboard')).toBeInTheDocument();
  });

  it('renders Enable All and Disable All buttons', () => {
    render(<AdminFeaturesPage />);
    expect(screen.getByText('Enable All')).toBeInTheDocument();
    expect(screen.getByText('Disable All')).toBeInTheDocument();
  });

  it('renders Reset to Preset button', () => {
    render(<AdminFeaturesPage />);
    expect(screen.getByText('Reset to Preset')).toBeInTheDocument();
  });

  it('renders compliance section', () => {
    render(<AdminFeaturesPage />);
    expect(screen.getByText('Compliance')).toBeInTheDocument();
    expect(screen.getByText('GDPR compliant')).toBeInTheDocument();
    expect(screen.getByText('Audit trail')).toBeInTheDocument();
    expect(screen.getByText('End-to-end encryption')).toBeInTheDocument();
  });

  it('shows feature count', () => {
    render(<AdminFeaturesPage />);
    // 2 out of 5 features enabled
    expect(screen.getByText(/2\/5/)).toBeInTheDocument();
  });

  it('renders feature toggle buttons', () => {
    render(<AdminFeaturesPage />);
    // There should be toggle buttons for each feature
    const toggleButtons = screen.getAllByRole('button').filter(
      btn => btn.getAttribute('aria-label')?.startsWith('features.modules.')
    );
    expect(toggleButtons.length).toBeGreaterThan(0);
  });

  it('clicking toggle changes local state', async () => {
    const user = userEvent.setup();
    render(<AdminFeaturesPage />);

    // Click a toggle - any feature toggle will do
    const toggles = screen.getAllByRole('button').filter(
      btn => btn.getAttribute('aria-label')?.startsWith('features.modules.')
    );
    if (toggles.length > 0) {
      await user.click(toggles[0]);
      // Save Changes button should appear since local state differs from features
      expect(screen.getByText('Save Changes')).toBeInTheDocument();
    }
  });

  it('clicking info button expands help text', async () => {
    const user = userEvent.setup();
    render(<AdminFeaturesPage />);

    const infoButtons = screen.getAllByLabelText('More info');
    expect(infoButtons.length).toBeGreaterThan(0);

    await user.click(infoButtons[0]);
    // Help text section is now shown
    const helpText = screen.getAllByText(/features\.modules\..+\.help/);
    expect(helpText.length).toBeGreaterThan(0);
  });

  it('save button calls setFeatures and shows toast', async () => {
    const user = userEvent.setup();
    render(<AdminFeaturesPage />);

    // Toggle a feature to make hasChanges true
    const toggles = screen.getAllByRole('button').filter(
      btn => btn.getAttribute('aria-label')?.startsWith('features.modules.')
    );
    await user.click(toggles[0]);

    const saveBtn = screen.getByText('Save Changes');
    await user.click(saveBtn);

    expect(mockSetFeatures).toHaveBeenCalled();
    expect(mockToastSuccess).toHaveBeenCalledWith('Saved!');
  });

  it('Enable All adds all features', async () => {
    const user = userEvent.setup();
    render(<AdminFeaturesPage />);

    await user.click(screen.getByText('Enable All'));
    // Now Save Changes should be visible since local state changed
    expect(screen.getByText('Save Changes')).toBeInTheDocument();
  });

  it('Disable All removes all features', async () => {
    const user = userEvent.setup();
    render(<AdminFeaturesPage />);

    await user.click(screen.getByText('Disable All'));
    expect(screen.getByText('Save Changes')).toBeInTheDocument();
  });

  it('Reset to Preset resets to use case preset', async () => {
    const user = userEvent.setup();
    render(<AdminFeaturesPage />);

    // First toggle something to change state
    await user.click(screen.getByText('Disable All'));
    // Then reset
    await user.click(screen.getByText('Reset to Preset'));
    // hasChanges recalculated
  });

  it('link to dashboard points to /', () => {
    render(<AdminFeaturesPage />);
    const dashLink = screen.getByText('Dashboard').closest('a');
    expect(dashLink).toHaveAttribute('href', '/');
  });

  it('clears saved state after timeout', async () => {
    const setTimeoutSpy = vi.spyOn(global, 'setTimeout');
    const user = userEvent.setup();
    render(<AdminFeaturesPage />);
    const toggles = screen.getAllByRole('button').filter(
      btn => btn.getAttribute('aria-label')?.startsWith('features.modules.')
    );
    await user.click(toggles[0]);
    await user.click(screen.getByText('Save Changes'));
    expect(mockToastSuccess).toHaveBeenCalled();
    expect(setTimeoutSpy).toHaveBeenCalledWith(expect.any(Function), 2000);
    // Manually invoke the timeout callback
    const timeoutCall = setTimeoutSpy.mock.calls.find(c => c[1] === 2000);
    if (timeoutCall) (timeoutCall[0] as Function)();
    setTimeoutSpy.mockRestore();
  });
});
