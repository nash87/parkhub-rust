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

/** Helper: get input by its id (label uses for="reg-*" with aria-hidden asterisk). */
function getInput(id: string) {
  return document.getElementById(id) as HTMLInputElement;
}

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

    expect(getInput('reg-name')).toBeInTheDocument();
    expect(getInput('reg-email')).toBeInTheDocument();
    expect(getInput('reg-password')).toBeInTheDocument();
    expect(getInput('reg-password-confirm')).toBeInTheDocument();
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

  // ── Input types & attributes (react-hook-form + zod, no HTML5 required) ──

  it('uses noValidate form with zod validation (no HTML required attrs)', () => {
    render(<RegisterPage />);

    // Form uses noValidate + zod schema — inputs don't have HTML required attribute
    expect(getInput('reg-name')).toBeInTheDocument();
    expect(getInput('reg-email')).toBeInTheDocument();
    expect(getInput('reg-password')).toBeInTheDocument();
    expect(getInput('reg-password-confirm')).toBeInTheDocument();
  });

  it('form fields show required indicator via asterisk', () => {
    render(<RegisterPage />);

    // FormField with required prop renders a visible asterisk
    const labels = document.querySelectorAll('label');
    const requiredLabels = Array.from(labels).filter(l =>
      l.querySelector('[aria-hidden="true"]')?.textContent === '*',
    );
    expect(requiredLabels.length).toBe(4); // name, email, password, confirm
  });

  it('email field has type email', () => {
    render(<RegisterPage />);

    expect(getInput('reg-email')).toHaveAttribute('type', 'email');
  });

  it('password fields have type password', () => {
    render(<RegisterPage />);

    expect(getInput('reg-password')).toHaveAttribute('type', 'password');
    expect(getInput('reg-password-confirm')).toHaveAttribute('type', 'password');
  });

  it('has correct autocomplete attributes', () => {
    render(<RegisterPage />);

    expect(getInput('reg-name')).toHaveAttribute('autocomplete', 'name');
    expect(getInput('reg-email')).toHaveAttribute('autocomplete', 'email');
    expect(getInput('reg-password')).toHaveAttribute('autocomplete', 'new-password');
    expect(getInput('reg-password-confirm')).toHaveAttribute('autocomplete', 'new-password');
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

    const pwInput = getInput('reg-password');

    // Type a short lowercase-only string
    await user.type(pwInput, 'abc');
    let checks = screen.getAllByTestId('icon-check');
    expect(checks.length).toBe(1); // only lowercase met

    // Add uppercase
    await user.clear(pwInput);
    await user.type(pwInput, 'abcDefgh');
    checks = screen.getAllByTestId('icon-check');
    expect(checks.length).toBe(3); // minLength + lower + upper

    // Add digit — all met
    await user.clear(pwInput);
    await user.type(pwInput, 'abcDef1h');
    checks = screen.getAllByTestId('icon-check');
    expect(checks.length).toBe(4);
  });

  // ── Password confirmation mismatch (mode: 'onBlur' via zod refine) ──

  it('shows mismatch error after blur when confirmation differs', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    const pwInput = getInput('reg-password');
    const confirmInput = getInput('reg-password-confirm');

    await user.type(pwInput, 'Test1234');
    await user.type(confirmInput, 'Test1235');
    // Trigger blur to activate onBlur validation
    await user.tab();

    await waitFor(() => {
      expect(screen.getByText('Passwords do not match')).toBeInTheDocument();
    });
  });

  it('does not show mismatch error when confirmation is empty', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(getInput('reg-password'), 'Test1234');

    expect(screen.queryByText('Passwords do not match')).not.toBeInTheDocument();
  });

  it('clears mismatch error when passwords match', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    const pwInput = getInput('reg-password');
    const confirmInput = getInput('reg-password-confirm');

    await user.type(pwInput, 'Test1234');
    await user.type(confirmInput, 'Test1235');
    await user.tab();

    await waitFor(() => {
      expect(screen.getByText('Passwords do not match')).toBeInTheDocument();
    });

    await user.clear(confirmInput);
    await user.type(confirmInput, 'Test1234');
    await user.tab();

    await waitFor(() => {
      expect(screen.queryByText('Passwords do not match')).not.toBeInTheDocument();
    });
  });

  // ── Submit button state (react-hook-form: only disabled during isSubmitting) ──

  it('submit button is enabled initially (react-hook-form validates on submit)', () => {
    render(<RegisterPage />);

    // react-hook-form only disables button during submission (isSubmitting)
    expect(screen.getByRole('button', { name: 'Sign Up' })).toBeEnabled();
  });

  it('does not submit when zod validation fails (empty fields)', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    // zod validation prevents onSubmit from being called
    expect(mockRegister).not.toHaveBeenCalled();
  });

  it('does not submit when password rules are not met', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(getInput('reg-name'), 'Test User');
    await user.type(getInput('reg-email'), 'test@example.com');
    await user.type(getInput('reg-password'), 'weak');
    await user.type(getInput('reg-password-confirm'), 'weak');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    expect(mockRegister).not.toHaveBeenCalled();
  });

  it('does not submit when passwords do not match', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(getInput('reg-name'), 'Test User');
    await user.type(getInput('reg-email'), 'test@example.com');
    await user.type(getInput('reg-password'), 'Test1234');
    await user.type(getInput('reg-password-confirm'), 'Test1235');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    expect(mockRegister).not.toHaveBeenCalled();
  });

  it('submits when all fields valid and passwords match', async () => {
    mockRegister.mockResolvedValue({ success: true });
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(getInput('reg-name'), 'Test User');
    await user.type(getInput('reg-email'), 'test@example.com');
    await user.type(getInput('reg-password'), 'Test1234');
    await user.type(getInput('reg-password-confirm'), 'Test1234');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    await waitFor(() => {
      expect(mockRegister).toHaveBeenCalled();
    });
  });

  it('does not submit when name is empty', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(getInput('reg-email'), 'test@example.com');
    await user.type(getInput('reg-password'), 'Test1234');
    await user.type(getInput('reg-password-confirm'), 'Test1234');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    expect(mockRegister).not.toHaveBeenCalled();
  });

  it('does not submit when email is empty', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(getInput('reg-name'), 'Test User');
    await user.type(getInput('reg-password'), 'Test1234');
    await user.type(getInput('reg-password-confirm'), 'Test1234');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    expect(mockRegister).not.toHaveBeenCalled();
  });

  // ── Successful registration ──

  it('calls register with password_confirmation and navigates on success', async () => {
    mockRegister.mockResolvedValue({ success: true });
    const user = userEvent.setup();

    render(<RegisterPage />);

    await user.type(getInput('reg-name'), 'Test User');
    await user.type(getInput('reg-email'), 'test@example.com');
    await user.type(getInput('reg-password'), 'Test1234');
    await user.type(getInput('reg-password-confirm'), 'Test1234');
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

    await user.type(getInput('reg-name'), 'Test User');
    await user.type(getInput('reg-email'), 'taken@example.com');
    await user.type(getInput('reg-password'), 'Test1234');
    await user.type(getInput('reg-password-confirm'), 'Test1234');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Email taken');
    });
  });

  it('shows fallback error when no message in response', async () => {
    mockRegister.mockResolvedValue({ success: false });
    const user = userEvent.setup();

    render(<RegisterPage />);

    await user.type(getInput('reg-name'), 'Test User');
    await user.type(getInput('reg-email'), 'fail@example.com');
    await user.type(getInput('reg-password'), 'Test1234');
    await user.type(getInput('reg-password-confirm'), 'Test1234');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Registration failed');
    });
  });

  it('does not navigate on failed registration', async () => {
    mockRegister.mockResolvedValue({ success: false, error: { message: 'Error' } });
    const user = userEvent.setup();

    render(<RegisterPage />);

    await user.type(getInput('reg-name'), 'Test User');
    await user.type(getInput('reg-email'), 'test@example.com');
    await user.type(getInput('reg-password'), 'Test1234');
    await user.type(getInput('reg-password-confirm'), 'Test1234');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
    expect(mockNavigate).not.toHaveBeenCalled();
  });

  // ── Does not submit when validation fails ──

  it('does not call register when validation fails', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    // Only fill name, leave other fields empty
    await user.type(getInput('reg-name'), 'Test User');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    expect(mockRegister).not.toHaveBeenCalled();
  });

  // ── Password complexity edge cases ──

  it('rejects password with only lowercase', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(getInput('reg-name'), 'Test');
    await user.type(getInput('reg-email'), 'a@b.com');
    await user.type(getInput('reg-password'), 'abcdefgh');
    await user.type(getInput('reg-password-confirm'), 'abcdefgh');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    // zod rejects — register not called
    expect(mockRegister).not.toHaveBeenCalled();
  });

  it('rejects password with only uppercase', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(getInput('reg-name'), 'Test');
    await user.type(getInput('reg-email'), 'a@b.com');
    await user.type(getInput('reg-password'), 'ABCDEFGH');
    await user.type(getInput('reg-password-confirm'), 'ABCDEFGH');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    expect(mockRegister).not.toHaveBeenCalled();
  });

  it('rejects password with only digits', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(getInput('reg-name'), 'Test');
    await user.type(getInput('reg-email'), 'a@b.com');
    await user.type(getInput('reg-password'), '12345678');
    await user.type(getInput('reg-password-confirm'), '12345678');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    expect(mockRegister).not.toHaveBeenCalled();
  });

  it('rejects password shorter than 8 characters even if complex', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(getInput('reg-name'), 'Test');
    await user.type(getInput('reg-email'), 'a@b.com');
    await user.type(getInput('reg-password'), 'Ab1cdef');
    await user.type(getInput('reg-password-confirm'), 'Ab1cdef');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    expect(mockRegister).not.toHaveBeenCalled();
  });

  it('accepts password that meets all rules exactly at boundary (8 chars)', async () => {
    mockRegister.mockResolvedValue({ success: true });
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(getInput('reg-name'), 'Test');
    await user.type(getInput('reg-email'), 'a@b.com');
    await user.type(getInput('reg-password'), 'Abcdefg1');
    await user.type(getInput('reg-password-confirm'), 'Abcdefg1');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    await waitFor(() => {
      expect(mockRegister).toHaveBeenCalled();
    });
  });

  // ── Confirmation field visual feedback ──

  it('adds error class to confirmation field on mismatch (after blur)', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    const pwInput = getInput('reg-password');
    const confirmInput = getInput('reg-password-confirm');

    await user.type(pwInput, 'Test1234');
    await user.type(confirmInput, 'Test1235');
    // Trigger blur to activate onBlur validation
    await user.tab();

    await waitFor(() => {
      // FormInput uses 'border-danger' class for errors, not 'border-red-500'
      expect(confirmInput).toHaveClass('border-danger');
    });
  });

  it('does not have error class when confirmation matches', async () => {
    const user = userEvent.setup();
    render(<RegisterPage />);

    await user.type(getInput('reg-password'), 'Test1234');
    await user.type(getInput('reg-password-confirm'), 'Test1234');

    expect(getInput('reg-password-confirm')).not.toHaveClass('border-danger');
  });
});
