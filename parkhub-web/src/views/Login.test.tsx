import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockNavigate = vi.fn();
const mockLogin = vi.fn();

vi.mock('react-router-dom', () => ({
  Link: ({ to, children, ...props }: any) => <a href={to} {...props}>{children}</a>,
  useNavigate: () => mockNavigate,
}));

vi.mock('../context/AuthContext', () => ({
  useAuth: () => ({
    login: mockLogin,
    user: null,
    loading: false,
    logout: vi.fn(),
    refreshUser: vi.fn(),
  }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => {
      const map: Record<string, string> = {
        'auth.login': 'Sign In',
        'auth.loginSubtitle': 'Welcome back',
        'auth.email': 'Email',
        'auth.password': 'Password',
        'auth.signIn': 'Sign In',
        'auth.loggingIn': 'Signing in...',
        'auth.forgotPassword': 'Forgot password?',
        'auth.noAccount': 'No account?',
        'auth.signUp': 'Sign Up',
        'auth.loginError': 'Login failed',
        'auth.demoHint': 'Use admin/demo to log in',
        'auth.back': 'Back',
        'auth.heroTitle': 'Your parking,\nyour server,\nyour rules.',
        'auth.heroSubtitle': 'Self-hosted parking management. No cloud, no tracking, no monthly fees.',
        'auth.hidePassword': 'Hide password',
        'auth.showPassword': 'Show password',
        'welcome.greeting': 'Welcome',
        'welcome.subtitle': 'Smart parking management',
        'login.feature.quick': 'Quick booking',
        'login.feature.secure': 'Secure',
        'login.feature.selfHosted': 'Self-hosted',
      };
      return map[key] || fallback || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
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
  Eye: (props: any) => <span data-testid="icon-eye" {...props} />,
  EyeSlash: (props: any) => <span data-testid="icon-eye-slash" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  ArrowLeft: (props: any) => <span data-testid="icon-arrow-left" {...props} />,
  Info: (props: any) => <span data-testid="icon-info" {...props} />,
  ShieldCheck: (props: any) => <span data-testid="icon-shield" {...props} />,
  Lightning: (props: any) => <span data-testid="icon-lightning" {...props} />,
  Globe: (props: any) => <span data-testid="icon-globe" {...props} />,
}));

import { LoginPage } from './Login';

describe('LoginPage', () => {
  beforeEach(() => {
    mockNavigate.mockClear();
    mockLogin.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the login form with username and password fields', () => {
    render(<LoginPage />);

    expect(screen.getByLabelText('Email')).toBeInTheDocument();
    expect(screen.getByLabelText('Password')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sign In' })).toBeInTheDocument();
  });

  it('renders the demo hint', () => {
    render(<LoginPage />);
    expect(screen.getByText('Use admin/demo to log in')).toBeInTheDocument();
  });

  it('has required attribute on both inputs', () => {
    render(<LoginPage />);

    expect(screen.getByLabelText('Email')).toBeRequired();
    expect(screen.getByLabelText('Password')).toBeRequired();
  });

  it('submit button is disabled when fields are empty', () => {
    render(<LoginPage />);
    expect(screen.getByRole('button', { name: 'Sign In' })).toBeDisabled();
  });

  it('submit button is enabled when both fields are filled', async () => {
    const user = userEvent.setup();
    render(<LoginPage />);

    await user.type(screen.getByLabelText('Email'), 'admin');
    await user.type(screen.getByLabelText('Password'), 'demo');

    expect(screen.getByRole('button', { name: 'Sign In' })).toBeEnabled();
  });

  it('toggles password visibility', async () => {
    const user = userEvent.setup();
    render(<LoginPage />);

    const passwordInput = screen.getByLabelText('Password');
    const toggleBtn = screen.getByLabelText('Show password');

    expect(passwordInput).toHaveAttribute('type', 'password');

    await user.click(toggleBtn);

    expect(passwordInput).toHaveAttribute('type', 'text');
    // After toggle, label changes
    expect(screen.getByLabelText('Hide password')).toBeInTheDocument();

    await user.click(screen.getByLabelText('Hide password'));
    expect(passwordInput).toHaveAttribute('type', 'password');
  });

  it('calls login on form submission and navigates on success', async () => {
    mockLogin.mockResolvedValue({ success: true });
    const user = userEvent.setup();

    render(<LoginPage />);

    await user.type(screen.getByLabelText('Email'), 'admin');
    await user.type(screen.getByLabelText('Password'), 'demo');
    await user.click(screen.getByRole('button', { name: 'Sign In' }));

    await waitFor(() => {
      expect(mockLogin).toHaveBeenCalledWith('admin', 'demo');
    });

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/', { replace: true });
    });
  });

  it('shows error message on failed login', async () => {
    mockLogin.mockResolvedValue({ success: false, error: 'Invalid credentials' });
    const user = userEvent.setup();

    render(<LoginPage />);

    await user.type(screen.getByLabelText('Email'), 'admin');
    await user.type(screen.getByLabelText('Password'), 'wrong');
    await user.click(screen.getByRole('button', { name: 'Sign In' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Invalid credentials');
    });
  });

  it('falls back to translated error when login returns no error string', async () => {
    mockLogin.mockResolvedValue({ success: false });
    const user = userEvent.setup();

    render(<LoginPage />);

    await user.type(screen.getByLabelText('Email'), 'admin');
    await user.type(screen.getByLabelText('Password'), 'wrong');
    await user.click(screen.getByRole('button', { name: 'Sign In' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Login failed');
    });
  });

  it('shows links to register and forgot password', () => {
    render(<LoginPage />);

    expect(screen.getByText('Sign Up')).toHaveAttribute('href', '/register');
    expect(screen.getByText('Forgot password?')).toHaveAttribute('href', '/forgot-password');
  });

  it('renders the version badge', () => {
    render(<LoginPage />);
    expect(screen.getByText(/ParkHub v\d+\.\d+/)).toBeInTheDocument();
  });
});
