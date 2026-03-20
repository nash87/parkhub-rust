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
        'auth.signUp': 'Sign Up',
        'auth.signIn': 'Sign In',
        'auth.hasAccount': 'Already have an account?',
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

  it('renders the registration form with all fields', () => {
    render(<RegisterPage />);

    expect(screen.getByLabelText('Name')).toBeInTheDocument();
    expect(screen.getByLabelText('Email')).toBeInTheDocument();
    expect(screen.getByLabelText('Password')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sign Up' })).toBeInTheDocument();
  });

  it('renders heading and subtitle', () => {
    render(<RegisterPage />);

    expect(screen.getByText('Create Account')).toBeInTheDocument();
    expect(screen.getByText('Join today')).toBeInTheDocument();
  });

  it('has required attributes on inputs', () => {
    render(<RegisterPage />);

    expect(screen.getByLabelText('Name')).toBeRequired();
    expect(screen.getByLabelText('Email')).toBeRequired();
    expect(screen.getByLabelText('Password')).toBeRequired();
  });

  it('password field has minLength of 8', () => {
    render(<RegisterPage />);

    expect(screen.getByLabelText('Password')).toHaveAttribute('minlength', '8');
  });

  it('shows the minimum length hint', () => {
    render(<RegisterPage />);

    expect(screen.getByText('Min. 8 characters')).toBeInTheDocument();
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

  it('calls register and navigates to login on success', async () => {
    mockRegister.mockResolvedValue({ success: true });
    const user = userEvent.setup();

    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test User');
    await user.type(screen.getByLabelText('Email'), 'test@example.com');
    await user.type(screen.getByLabelText('Password'), 'password123');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    await waitFor(() => {
      expect(mockRegister).toHaveBeenCalledWith({
        username: '',
        email: 'test@example.com',
        name: 'Test User',
        password: 'password123',
      });
    });

    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/login');
    });
  });

  it('shows error message on failed registration', async () => {
    mockRegister.mockResolvedValue({ success: false, error: { message: 'Email taken' } });
    const user = userEvent.setup();

    render(<RegisterPage />);

    await user.type(screen.getByLabelText('Name'), 'Test User');
    await user.type(screen.getByLabelText('Email'), 'taken@example.com');
    await user.type(screen.getByLabelText('Password'), 'password123');
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
    await user.type(screen.getByLabelText('Password'), 'password123');
    await user.click(screen.getByRole('button', { name: 'Sign Up' }));

    await waitFor(() => {
      expect(screen.getByRole('alert')).toHaveTextContent('Registration failed');
    });
  });

  it('renders ParkHub branding', () => {
    render(<RegisterPage />);

    expect(screen.getByText('ParkHub')).toBeInTheDocument();
  });
});
