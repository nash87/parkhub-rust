import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockNavigate = vi.fn();
const mockChangeLanguage = vi.fn();
const mockSetTheme = vi.fn();

vi.mock('react-router-dom', () => ({
  useNavigate: () => mockNavigate,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'welcome.subtitle': 'Smart parking management',
        'welcome.selfHosted': 'Self-hosted',
        'welcome.features.booking': 'Booking',
        'welcome.features.bookingDesc': 'Book your spot',
        'welcome.features.credits': 'Credits',
        'welcome.features.creditsDesc': 'Manage credits',
        'welcome.features.analytics': 'Analytics',
        'welcome.features.analyticsDesc': 'Track usage',
        'welcome.getStarted': 'Get Started',
        'nav.switchToLight': 'Switch to light mode',
        'nav.switchToDark': 'Switch to dark mode',
      };
      return map[key] || key;
    },
    i18n: {
      language: 'en',
      changeLanguage: mockChangeLanguage,
    },
  }),
}));

vi.mock('../context/ThemeContext', () => ({
  useTheme: () => ({
    resolved: 'light' as const,
    setTheme: mockSetTheme,
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, variants, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    p: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <p ref={ref} {...props}>{children}</p>
    )),
    button: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <button ref={ref} {...props}>{children}</button>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  CarSimple: (props: any) => <span data-testid="icon-car" {...props} />,
  CalendarCheck: (props: any) => <span data-testid="icon-calendar" {...props} />,
  ChartLineUp: (props: any) => <span data-testid="icon-chart" {...props} />,
  Sparkle: (props: any) => <span data-testid="icon-sparkle" {...props} />,
  ArrowRight: (props: any) => <span data-testid="icon-arrow" {...props} />,
  Globe: (props: any) => <span data-testid="icon-globe" {...props} />,
  SunDim: (props: any) => <span data-testid="icon-sun" {...props} />,
  Moon: (props: any) => <span data-testid="icon-moon" {...props} />,
}));

vi.mock('../i18n', () => ({
  languages: [
    { code: 'en', name: 'English', flag: '\u{1F1EC}\u{1F1E7}', native: 'English' },
    { code: 'de', name: 'German', flag: '\u{1F1E9}\u{1F1EA}', native: 'Deutsch' },
    { code: 'fr', name: 'French', flag: '\u{1F1EB}\u{1F1F7}', native: 'Francais' },
    { code: 'es', name: 'Spanish', flag: '\u{1F1EA}\u{1F1F8}', native: 'Espanol' },
    { code: 'it', name: 'Italian', flag: '\u{1F1EE}\u{1F1F9}', native: 'Italiano' },
    { code: 'pt', name: 'Portuguese', flag: '\u{1F1F5}\u{1F1F9}', native: 'Portugues' },
    { code: 'tr', name: 'Turkish', flag: '\u{1F1F9}\u{1F1F7}', native: 'Turkce' },
    { code: 'pl', name: 'Polish', flag: '\u{1F1F5}\u{1F1F1}', native: 'Polski' },
    { code: 'ja', name: 'Japanese', flag: '\u{1F1EF}\u{1F1F5}', native: '\u65E5\u672C\u8A9E' },
    { code: 'zh', name: 'Chinese', flag: '\u{1F1E8}\u{1F1F3}', native: '\u4E2D\u6587' },
  ],
}));

import { WelcomePage } from './Welcome';

describe('WelcomePage', () => {
  beforeEach(() => {
    mockNavigate.mockClear();
    mockChangeLanguage.mockClear();
    mockSetTheme.mockClear();
    localStorage.clear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the initial greeting text', () => {
    render(<WelcomePage />);
    expect(screen.getByText('Welcome')).toBeInTheDocument();
  });

  it('cycles through greeting texts on interval', () => {
    vi.useFakeTimers();
    try {
      render(<WelcomePage />);
      expect(screen.getByText('Welcome')).toBeInTheDocument();

      // Advance past one cycle (2500ms)
      act(() => { vi.advanceTimersByTime(2500); });
      expect(screen.getByText('Willkommen')).toBeInTheDocument();

      act(() => { vi.advanceTimersByTime(2500); });
      expect(screen.getByText('Bienvenue')).toBeInTheDocument();
    } finally {
      vi.useRealTimers();
    }
  });

  it('renders the subtitle', () => {
    render(<WelcomePage />);
    expect(screen.getByText('Smart parking management')).toBeInTheDocument();
  });

  it('renders three feature cards', () => {
    render(<WelcomePage />);
    expect(screen.getByText('Booking')).toBeInTheDocument();
    expect(screen.getByText('Credits')).toBeInTheDocument();
    expect(screen.getByText('Analytics')).toBeInTheDocument();
  });

  it('renders the language selector button with current language', () => {
    render(<WelcomePage />);
    // The button shows the current language native name
    const buttons = screen.getAllByRole('button');
    const langButton = buttons.find(b => b.textContent?.includes('English'));
    expect(langButton).toBeDefined();
  });

  it('shows all 10 languages when language selector is clicked', async () => {
    const user = userEvent.setup();
    render(<WelcomePage />);

    // Click the language selector button (contains Globe icon + "English")
    const buttons = screen.getAllByRole('button');
    const langButton = buttons.find(b => b.textContent?.includes('English'));
    await user.click(langButton!);

    // All 10 language options should be visible
    expect(screen.getByText('Deutsch')).toBeInTheDocument();
    expect(screen.getByText('Francais')).toBeInTheDocument();
    expect(screen.getByText('Espanol')).toBeInTheDocument();
    expect(screen.getByText('Italiano')).toBeInTheDocument();
    expect(screen.getByText('Portugues')).toBeInTheDocument();
    expect(screen.getByText('Turkce')).toBeInTheDocument();
    expect(screen.getByText('Polski')).toBeInTheDocument();
    expect(screen.getByText('\u65E5\u672C\u8A9E')).toBeInTheDocument();
    expect(screen.getByText('\u4E2D\u6587')).toBeInTheDocument();
  });

  it('calls i18n.changeLanguage when a language is selected', async () => {
    const user = userEvent.setup();
    render(<WelcomePage />);

    // Open language selector
    const buttons = screen.getAllByRole('button');
    const langButton = buttons.find(b => b.textContent?.includes('English'));
    await user.click(langButton!);

    // Click German
    await user.click(screen.getByText('Deutsch'));

    expect(mockChangeLanguage).toHaveBeenCalledWith('de');
  });

  it('sets localStorage flag and navigates to /login on "Get Started"', async () => {
    const user = userEvent.setup();
    render(<WelcomePage />);

    await user.click(screen.getByText('Get Started'));

    expect(localStorage.getItem('parkhub_welcome_seen')).toBe('1');
    expect(mockNavigate).toHaveBeenCalledWith('/login');
  });

  it('renders the theme toggle button', () => {
    render(<WelcomePage />);
    expect(screen.getByLabelText('Switch to dark mode')).toBeInTheDocument();
  });

  it('calls setTheme when theme toggle is clicked', async () => {
    const user = userEvent.setup();
    render(<WelcomePage />);

    await user.click(screen.getByLabelText('Switch to dark mode'));

    // resolved is 'light', so clicking should toggle to 'dark'
    expect(mockSetTheme).toHaveBeenCalledWith('dark');
  });
});
