import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockForgotPassword = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    forgotPassword: (...args: any[]) => mockForgotPassword(...args),
  },
}));

vi.mock('react-router-dom', () => ({
  Link: ({ to, children, ...props }: any) => <a href={to} {...props}>{children}</a>,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'auth.signIn': 'Sign In',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  CarSimple: (props: any) => <span data-testid="icon-car" {...props} />,
  ArrowLeft: (props: any) => <span data-testid="icon-arrow-left" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  CheckCircle: (props: any) => <span data-testid="icon-check-circle" {...props} />,
}));

import { ForgotPasswordPage } from './ForgotPassword';

describe('ForgotPasswordPage', () => {
  beforeEach(() => {
    mockForgotPassword.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the email input field', () => {
    render(<ForgotPasswordPage />);

    expect(screen.getByLabelText('Email')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('you@company.com')).toBeInTheDocument();
  });

  it('renders the reset password heading', () => {
    render(<ForgotPasswordPage />);

    expect(screen.getByText('Reset password')).toBeInTheDocument();
    expect(screen.getByText("Enter your email and we'll send you a reset link.")).toBeInTheDocument();
  });

  it('submit button is disabled when email is empty', () => {
    render(<ForgotPasswordPage />);

    expect(screen.getByRole('button', { name: 'Send reset link' })).toBeDisabled();
  });

  it('submit button is enabled when email is entered', async () => {
    const user = userEvent.setup();
    render(<ForgotPasswordPage />);

    await user.type(screen.getByLabelText('Email'), 'test@example.com');

    expect(screen.getByRole('button', { name: 'Send reset link' })).toBeEnabled();
  });

  it('calls api.forgotPassword on submit', async () => {
    mockForgotPassword.mockResolvedValue({ success: true });
    const user = userEvent.setup();

    render(<ForgotPasswordPage />);

    await user.type(screen.getByLabelText('Email'), 'user@example.com');
    await user.click(screen.getByRole('button', { name: 'Send reset link' }));

    await waitFor(() => {
      expect(mockForgotPassword).toHaveBeenCalledWith('user@example.com');
    });
  });

  it('shows success state after submit', async () => {
    mockForgotPassword.mockResolvedValue({ success: true });
    const user = userEvent.setup();

    render(<ForgotPasswordPage />);

    await user.type(screen.getByLabelText('Email'), 'user@example.com');
    await user.click(screen.getByRole('button', { name: 'Send reset link' }));

    await waitFor(() => {
      expect(screen.getByText('Check your email')).toBeInTheDocument();
    });
    expect(screen.getByText(/password reset link/)).toBeInTheDocument();
    // Email input should no longer be visible
    expect(screen.queryByLabelText('Email')).not.toBeInTheDocument();
  });

  it('shows "Back to Sign In" link in success state', async () => {
    mockForgotPassword.mockResolvedValue({ success: true });
    const user = userEvent.setup();

    render(<ForgotPasswordPage />);

    await user.type(screen.getByLabelText('Email'), 'user@example.com');
    await user.click(screen.getByRole('button', { name: 'Send reset link' }));

    await waitFor(() => {
      expect(screen.getByText('Back to Sign In')).toBeInTheDocument();
    });
    expect(screen.getByText('Back to Sign In').closest('a')).toHaveAttribute('href', '/login');
  });

  it('has back link to login page in initial state', () => {
    render(<ForgotPasswordPage />);

    const backLink = screen.getByText('Sign In');
    expect(backLink.closest('a')).toHaveAttribute('href', '/login');
  });

  it('renders ParkHub branding', () => {
    render(<ForgotPasswordPage />);

    expect(screen.getByText('ParkHub')).toBeInTheDocument();
  });

  it('email input has correct type and autocomplete attributes', () => {
    render(<ForgotPasswordPage />);

    const emailInput = screen.getByLabelText('Email');
    expect(emailInput).toHaveAttribute('type', 'email');
    expect(emailInput).toHaveAttribute('autocomplete', 'email');
  });

  it('email input is required', () => {
    render(<ForgotPasswordPage />);

    expect(screen.getByLabelText('Email')).toBeRequired();
  });
});
