import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockNavigate = vi.fn();
const mockRegister = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    register: (...args: any[]) => mockRegister(...args),
  },
}));

vi.mock('react-router-dom', () => ({
  Link: ({ to, children, ...props }: any) => <a href={to} {...props}>{children}</a>,
  useNavigate: () => mockNavigate,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'auth.register': 'Create Account',
        'auth.registerSubtitle': 'Join today',
        'auth.name': 'Name',
        'auth.email': 'Email',
        'auth.password': 'Password',
        'auth.confirmPassword': 'Confirm Password',
        'auth.signUp': 'Sign Up',
        'auth.signIn': 'Sign In',
        'auth.hasAccount': 'Already have an account?',
        'auth.minChars': 'Min. 8 characters',
        'auth.registrationFailed': 'Registration failed',
        'auth.creatingAccount': 'Creating account...',
        'auth.passwordMismatch': 'Passwords do not match',
        'auth.rule8Chars': '8+ characters',
        'auth.ruleLower': 'Lowercase',
        'auth.ruleUpper': 'Uppercase',
        'auth.ruleDigit': 'Number',
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
    p: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <p ref={ref} {...props}>{children}</p>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  CarSimple: (props: any) => <span data-testid="icon-car" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  ArrowLeft: (props: any) => <span data-testid="icon-arrow-left" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
}));

import { RegisterPage } from './Register';

describe('RegisterPage', () => {
  beforeEach(() => {
    mockNavigate.mockClear();
    mockRegister.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ── Rendering ──

  it('renders all form fields including password confirmation', () => {
    render(<RegisterPage />);

    expect(screen.getByLabelText('Name')).toBeInTheDocument();
    expect(screen.getByLabelText('Email')).toBeInTheDocument();
    expect(screen.getByLabelText('Password')).toBeInTheDocument();
    expect(screen.getByLabelText('Confirm Password')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sign Up' })).toBeInTheDocument();
  });

  it('renders heading and subtitle', () => {
    render(<RegisterPage />);

    expect(screen.getByText('Create Account')).toBeInTheDocument();
    expect(screen.getByText('Join today')).toBeInTheDocument();
  });

  it('renders ParkHub branding', () => {
    render(<RegisterPage />);

    expect(screen.getByText('ParkHub')).toBeInTheDocument();
  });

  it('renders password complexity rules', () => {
    render(<RegisterPage />);

    expect(screen.getByText('8+ characters')).toBeInTheDocument();
    expect(screen.getByText('Lowercase')).toBeInTheDocument();
    expect(screen.getByText('Uppercase')).toBeInTheDocument();
    expect(screen.getByText('Number')).toBeInTheDocument();
  });

  it('has a link back to login', () => {
    render(<RegisterPage />);

    const links = screen.getAllByText('Sign In');
    const loginLink = links.find(el => el.getAttribute('href') === '/login');
    expect(loginLink).toBeDefined();
  });

  it('shows "Already have an account?" text', () => {
    render(<RegisterPage />);

    expect(screen.getByText('Already have an account?')).toBeInTheDocument();
  });

  // ── Required attributes ──

  it('has required attributes on all inputs', () => {
    render(<RegisterPage />);

    expect(screen.getByLabelText('Name')).toBeRequired();
    expect(screen.getByLabelText('Email')).toBeRequired();
    expect(screen.getByLabelText('Password')).toBeRequired();
    expect(screen.getByLabelText('Confirm Password')).toBeRequired();
  });

  it('password fields have minLength of 8', () => {
    render(<RegisterPage />);

    expect(screen.getByLabelText('Password')).toHaveAttribute('minlength', '8');
    expect(screen.getByLabelText('Confirm Password')).toHaveAttribute('minlength', '8');
  });

  it('email field has type email', () => {
    render(<RegisterPage />);

    expect(screen.getByLabelText('Email')).toHaveAttribute('type', 'email');
  });

  it('password fields have type password', () => {
    render(<RegisterPage />);

    expect(screen.getByLabelText('Password')).toHaveAttribute('type', 'password');
    expect(screen.getByLabelText('Confirm Password')).toHaveAttribute('type', 'password');
  });

  it('has correct autocomplete attributes', () => {
    render(<RegisterPage />);

    expect(screen.getByLabelText('Name')).toHaveAttribute('autocomplete', 'name');
    expect(screen.getByLabelText('Email')).toHaveAttribute('autocomplete', 'email');
    expect(screen.getByLabelText('Password')).toHaveAttribute('autocomplete', 'new-password');
    expect(screen.getByLabelText('Confirm Password')).toHaveAttribute('autocomplete', 'new-password');
  });

  // ── Password validation rules ──

  it('all rules show as unmet with empty password', () => {
    render(<RegisterPage />);

    const xIcons = screen.getAllByTestId('icon-x');
    expect(xIcons.length).toBe(4);
  });

  it('updates rule indicators as password is typed', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    // Type a short lowercase-only string
    await user.type(screen.getByLabelText('Password'), 'abc');
    let checks = screen.getAllByTestId('icon-check');
    expect(checks.length).toBe(1); // only lowercase met

    // Add uppercase
    await user.clear(screen.getByLabelText('Password'));
    await user.type(screen.getByLabelText('Password'), 'abcDefgh');
    checks = screen.getAllByTestId('icon-check');
    expect(checks.length).toBe(3); // minLength + lower + upper

    // Add digit — all met
    await user.clear(screen.getByLabelText('Password'));
    await user.type(screen.getByLabelText('Password'), 'abcDef1h');
    checks = screen.getAllByTestId('icon-check');
    expect(checks.length).toBe(4);
  });

  // ── Password confirmation mismatch ──

  it('shows mismatch error when confirmation differs', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Password'), 'Test1234');
    await user.type(screen.getByLabelText('Confirm Password'), 'Test1235');

    expect(screen.getByText('Passwords do not match')).toBeInTheDocument();
  });

  it('does not show mismatch error when confirmation is empty', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Password'), 'Test1234');

    expect(screen.queryByText('Passwords do not match')).not.toBeInTheDocument();
  });

  it('clears mismatch error when passwords match', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Password'), 'Test1234');
    await user.type(screen.getByLabelText('Confirm Password'), 'Test1235');
    expect(screen.getByText('Passwords do not match')).toBeInTheDocument();

    await user.clear(screen.getByLabelText('Confirm Password'));
    await user.type(screen.getByLabelText('Confirm Password'), 'Test1234');
    expect(screen.queryByText('Passwords do not match')).not.toBeInTheDocument();
  });

  // ── Submit button disabled state ──

  it('submit button is disabled when form is incomplete', () => {
    render(<RegisterPage />);

    expect(screen.getByRole('button', { name: 'Sign Up' })).toBeDisabled();
  });

  it('submit button is disabled when password rules are not met', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test User');
    await user.type(screen.getByLabelText('Email'), 'test@example.com');
    await user.type(screen.getByLabelText('Password'), 'weak');
    await user.type(screen.getByLabelText('Confirm Password'), 'weak');

    expect(screen.getByRole('button', { name: 'Sign Up' })).toBeDisabled();
  });

  it('submit button is disabled when passwords do not match', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test User');
    await user.type(screen.getByLabelText('Email'), 'test@example.com');
    await user.type(screen.getByLabelText('Password'), 'Test1234');
    await user.type(screen.getByLabelText('Confirm Password'), 'Test1235');

    expect(screen.getByRole('button', { name: 'Sign Up' })).toBeDisabled();
  });

  it('submit button is enabled when all fields valid and passwords match', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test User');
    await user.type(screen.getByLabelText('Email'), 'test@example.com');
    await user.type(screen.getByLabelText('Password'), 'Test1234');
    await user.type(screen.getByLabelText('Confirm Password'), 'Test1234');

    expect(screen.getByRole('button', { name: 'Sign Up' })).toBeEnabled();
  });

  it('submit button is disabled when name is empty', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Email'), 'test@example.com');
    await user.type(screen.getByLabelText('Password'), 'Test1234');
    await user.type(screen.getByLabelText('Confirm Password'), 'Test1234');

    expect(screen.getByRole('button', { name: 'Sign Up' })).toBeDisabled();
  });

  it('submit button is disabled when email is empty', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test User');
    await user.type(screen.getByLabelText('Password'), 'Test1234');
    await user.type(screen.getByLabelText('Confirm Password'), 'Test1234');

    expect(screen.getByRole('button', { name: 'Sign Up' })).toBeDisabled();
  });

  // ── Successful registration ──

  it('calls register with password_confirmation and navigates on success', async () => {
    mockRegister.mockResolvedValue({ success: true });
    const user = userEvent.setup();

    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test User');
    await user.type(screen.getByLabelText('Email'), 'test@example.com');
    await user.type(screen.getByLabelText('Password'), 'Test1234');
    await user.type(screen.getByLabelText('Confirm Password'), 'Test1234');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    await waitFor(() => {
      expect(mockRegister).toHaveBeenCalledWith({
        name: 'Test User',
        email: 'test@example.com',
        password: 'Test1234',
        password_confirmation: 'Test1234',
      });
    });

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/login');
    });
  });

  // ── Failed registration ──

  it('shows error message on failed registration', async () => {
    mockRegister.mockResolvedValue({ success: false, error: { message: 'Email taken' } });
    const user = userEvent.setup();

    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test User');
    await user.type(screen.getByLabelText('Email'), 'taken@example.com');
    await user.type(screen.getByLabelText('Password'), 'Test1234');
    await user.type(screen.getByLabelText('Confirm Password'), 'Test1234');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Email taken');
    });
  });

  it('shows fallback error when no message in response', async () => {
    mockRegister.mockResolvedValue({ success: false });
    const user = userEvent.setup();

    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test User');
    await user.type(screen.getByLabelText('Email'), 'fail@example.com');
    await user.type(screen.getByLabelText('Password'), 'Test1234');
    await user.type(screen.getByLabelText('Confirm Password'), 'Test1234');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Registration failed');
    });
  });

  it('does not navigate on failed registration', async () => {
    mockRegister.mockResolvedValue({ success: false, error: { message: 'Error' } });
    const user = userEvent.setup();

    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test User');
    await user.type(screen.getByLabelText('Email'), 'test@example.com');
    await user.type(screen.getByLabelText('Password'), 'Test1234');
    await user.type(screen.getByLabelText('Confirm Password'), 'Test1234');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  // ── Does not submit when validation fails ──

  it('does not call register when canSubmit is false', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test User');

    const btn = screen.getByRole('button', { name: 'Sign Up' });
    expect(btn).toBeDisabled();

    expect(mockRegister).not.toHaveBeenCalled();
  });

  // ── Password complexity edge cases ──

  it('rejects password with only lowercase', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test');
    await user.type(screen.getByLabelText('Email'), 'a@b.com');
    await user.type(screen.getByLabelText('Password'), 'abcdefgh');
    await user.type(screen.getByLabelText('Confirm Password'), 'abcdefgh');

    expect(screen.getByRole('button', { name: 'Sign Up' })).toBeDisabled();
  });

  it('rejects password with only uppercase', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test');
    await user.type(screen.getByLabelText('Email'), 'a@b.com');
    await user.type(screen.getByLabelText('Password'), 'ABCDEFGH');
    await user.type(screen.getByLabelText('Confirm Password'), 'ABCDEFGH');

    expect(screen.getByRole('button', { name: 'Sign Up' })).toBeDisabled();
  });

  it('rejects password with only digits', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test');
    await user.type(screen.getByLabelText('Email'), 'a@b.com');
    await user.type(screen.getByLabelText('Password'), '12345678');
    await user.type(screen.getByLabelText('Confirm Password'), '12345678');

    expect(screen.getByRole('button', { name: 'Sign Up' })).toBeDisabled();
  });

  it('rejects password shorter than 8 characters even if complex', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test');
    await user.type(screen.getByLabelText('Email'), 'a@b.com');
    await user.type(screen.getByLabelText('Password'), 'Ab1cdef');
    await user.type(screen.getByLabelText('Confirm Password'), 'Ab1cdef');

    expect(screen.getByRole('button', { name: 'Sign Up' })).toBeDisabled();
  });

  it('accepts password that meets all rules exactly at boundary (8 chars)', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test');
    await user.type(screen.getByLabelText('Email'), 'a@b.com');
    await user.type(screen.getByLabelText('Password'), 'Abcdefg1');
    await user.type(screen.getByLabelText('Confirm Password'), 'Abcdefg1');

    expect(screen.getByRole('button', { name: 'Sign Up' })).toBeEnabled();
  });

  // ── Confirmation field visual feedback ──

  it('adds red border class to confirmation field on mismatch', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Password'), 'Test1234');
    await user.type(screen.getByLabelText('Confirm Password'), 'Test1235');

    expect(screen.getByLabelText('Confirm Password')).toHaveClass('border-red-500');
  });

  it('does not have red border when confirmation matches', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Password'), 'Test1234');
    await user.type(screen.getByLabelText('Confirm Password'), 'Test1234');

    expect(screen.getByLabelText('Confirm Password')).not.toHaveClass('border-red-500');
  });
});
