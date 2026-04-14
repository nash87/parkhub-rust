import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockNavigate = vi.fn();
const mockSetUseCase = vi.fn();
const mockSetFeatures = vi.fn();
const mockApplyPreset = vi.fn();
const mockSetTheme = vi.fn();

vi.mock('react-router-dom', () => ({
  useNavigate: () => mockNavigate,
}));

vi.mock('../context/UseCaseContext', () => ({
  useUseCase: () => ({ useCase: 'business', setUseCase: mockSetUseCase, hasChosen: false }),
}));

vi.mock('../context/ThemeContext', () => ({
  useTheme: () => ({ resolved: 'light', setTheme: mockSetTheme }),
}));

vi.mock('../components/GenerativeBg', () => ({
  useBgClass: () => '',
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
      features: [],
      setFeatures: mockSetFeatures,
      applyPreset: mockApplyPreset,
      isEnabled: () => false,
    }),
    FEATURE_REGISTRY,
    USE_CASE_PRESETS,
  };
});

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, variants, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    button: React.forwardRef(({ children, initial, animate, exit, transition, variants, whileHover, whileTap, ...props }: any, ref: any) => (
      <button ref={ref} {...props}>{children}</button>
    )),
  },
  AnimatePresence: ({ children, mode }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Buildings: (props: any) => <span data-testid="icon-buildings" {...props} />,
  House: (props: any) => <span data-testid="icon-house" {...props} />,
  UsersThree: (props: any) => <span data-testid="icon-users" {...props} />,
  Car: (props: any) => <span data-testid="icon-car" {...props} />,
  ArrowRight: (props: any) => <span data-testid="icon-right" {...props} />,
  ArrowLeft: (props: any) => <span data-testid="icon-left" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  SunDim: (props: any) => <span data-testid="icon-sun" {...props} />,
  Moon: (props: any) => <span data-testid="icon-moon" {...props} />,
  ToggleLeft: (props: any) => <span data-testid="toggle-off" {...props} />,
  ToggleRight: (props: any) => <span data-testid="toggle-on" {...props} />,
  Info: (props: any) => <span data-testid="icon-info" {...props} />,
  ShieldCheck: (props: any) => <span data-testid="icon-shield" {...props} />,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'useCase.title': 'How will you use ParkHub?',
        'useCase.subtitle': 'Choose your use case',
        'useCase.business.name': 'Business',
        'useCase.business.desc': 'Company parking management',
        'useCase.residential.name': 'Residential',
        'useCase.residential.desc': 'Apartment complex parking',
        'useCase.personal.name': 'Personal',
        'useCase.personal.desc': 'Personal parking tracker',
        'useCase.continue': 'Continue',
        'useCase.skip': 'Skip setup',
        'useCase.applying': 'Applying...',
        'features.onboardingTitle': 'Customize Features',
        'features.onboardingSubtitle': 'Toggle features on or off',
        'features.enabled': 'Enabled',
        'features.disabled': 'Disabled',
        'features.compliance.title': 'Compliance',
        'features.compliance.gdpr': 'GDPR',
        'features.compliance.audit': 'Audit',
        'features.compliance.encryption': 'Encryption',
        'onboarding.back': 'Back',
        'onboarding.finish': 'Finish',
        'common.info': 'More info',
        'nav.switchToLight': 'Switch to light',
        'nav.switchToDark': 'Switch to dark',
      };
      return map[key] || key;
    },
  }),
}));

import { UseCaseSelectorPage } from './UseCaseSelector';

describe('UseCaseSelectorPage', () => {
  beforeEach(() => {
    mockNavigate.mockClear();
    mockSetUseCase.mockClear();
    mockSetFeatures.mockClear();
    mockApplyPreset.mockClear();
    mockSetTheme.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the use case selection step', () => {
    render(<UseCaseSelectorPage />);
    expect(screen.getByText('How will you use ParkHub?')).toBeInTheDocument();
    expect(screen.getByText('Choose your use case')).toBeInTheDocument();
  });

  it('renders all three use case cards', () => {
    render(<UseCaseSelectorPage />);
    expect(screen.getByText('Business')).toBeInTheDocument();
    expect(screen.getByText('Residential')).toBeInTheDocument();
    expect(screen.getByText('Personal')).toBeInTheDocument();
  });

  it('continue button is disabled until a use case is selected', () => {
    render(<UseCaseSelectorPage />);
    const continueBtn = screen.getByText('Continue');
    expect(continueBtn).toBeDisabled();
  });

  it('selecting a use case enables the continue button', async () => {
    const user = userEvent.setup();
    render(<UseCaseSelectorPage />);

    await user.click(screen.getByText('Business'));
    const continueBtn = screen.getByText('Continue');
    expect(continueBtn).not.toBeDisabled();
  });

  it('continue advances to features step', async () => {
    const user = userEvent.setup();
    render(<UseCaseSelectorPage />);

    await user.click(screen.getByText('Business'));
    await user.click(screen.getByText('Continue'));

    expect(screen.getByText('Customize Features')).toBeInTheDocument();
    expect(screen.getByText('Toggle features on or off')).toBeInTheDocument();
  });

  it('back button returns to use case step', async () => {
    const user = userEvent.setup();
    render(<UseCaseSelectorPage />);

    await user.click(screen.getByText('Business'));
    await user.click(screen.getByText('Continue'));
    expect(screen.getByText('Customize Features')).toBeInTheDocument();

    await user.click(screen.getByText('Back'));
    expect(screen.getByText('How will you use ParkHub?')).toBeInTheDocument();
  });

  it('finish button sets use case and features, navigates to welcome', async () => {
    vi.useFakeTimers();
    const { container } = render(<UseCaseSelectorPage />);

    // Use fireEvent for fake timers compatibility
    const businessBtn = screen.getByText('Business').closest('button')!;
    await act(async () => { businessBtn.click(); });

    const continueBtn = screen.getByText('Continue').closest('button')!;
    await act(async () => { continueBtn.click(); });

    const finishBtn = screen.getByText('Finish').closest('button')!;
    await act(async () => { finishBtn.click(); });

    expect(mockSetUseCase).toHaveBeenCalledWith('business');
    expect(mockSetFeatures).toHaveBeenCalled();

    // Shows "Applying..." state
    expect(screen.getByText('Applying...')).toBeInTheDocument();

    await act(async () => { vi.advanceTimersByTime(400); });
    expect(mockNavigate).toHaveBeenCalledWith('/welcome');
    vi.useRealTimers();
  });

  it('skip button sets business defaults and navigates', async () => {
    render(<UseCaseSelectorPage />);

    const skipBtn = screen.getByText('Skip setup');
    await act(async () => { skipBtn.click(); });

    expect(mockSetUseCase).toHaveBeenCalledWith('business');
    expect(mockApplyPreset).toHaveBeenCalledWith('business');
    expect(mockNavigate).toHaveBeenCalledWith('/welcome');
  });

  it('theme toggle switches theme', async () => {
    render(<UseCaseSelectorPage />);

    const themeBtn = screen.getByLabelText('Switch to dark');
    await act(async () => { themeBtn.click(); });

    expect(mockSetTheme).toHaveBeenCalledWith('dark');
  });

  it('features step shows feature toggles', async () => {
    render(<UseCaseSelectorPage />);

    const businessBtn = screen.getByText('Business').closest('button')!;
    await act(async () => { businessBtn.click(); });

    const continueBtn = screen.getByText('Continue').closest('button')!;
    await act(async () => { continueBtn.click(); });

    // Feature toggles should be visible
    const toggleButtons = screen.getAllByRole('button').filter(
      btn => btn.getAttribute('aria-label')?.startsWith('features.modules.')
    );
    expect(toggleButtons.length).toBeGreaterThan(0);
  });

  it('features step shows compliance section', async () => {
    render(<UseCaseSelectorPage />);

    const resBtn = screen.getByText('Residential').closest('button')!;
    await act(async () => { resBtn.click(); });

    const continueBtn = screen.getByText('Continue').closest('button')!;
    await act(async () => { continueBtn.click(); });

    expect(screen.getByText('Compliance')).toBeInTheDocument();
  });

  it('feature info button expands help text', async () => {
    render(<UseCaseSelectorPage />);

    const businessBtn = screen.getByText('Business').closest('button')!;
    await act(async () => { businessBtn.click(); });

    const continueBtn = screen.getByText('Continue').closest('button')!;
    await act(async () => { continueBtn.click(); });

    const infoButtons = screen.getAllByLabelText('More info');
    expect(infoButtons.length).toBeGreaterThan(0);
    await act(async () => { infoButtons[0].click(); });
    // Help text expanded
    const helpTexts = screen.getAllByText(/features\.modules\..+\.help/);
    expect(helpTexts.length).toBeGreaterThan(0);
  });

  it('renders step indicators', () => {
    render(<UseCaseSelectorPage />);
    // Two step dots should exist
    const dots = document.querySelectorAll('.rounded-full');
    expect(dots.length).toBeGreaterThanOrEqual(2);
  });
});
