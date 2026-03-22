import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, act } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mock API ──
// vi.mock is hoisted, so we use vi.hoisted to define mock fns that the factory can reference.
const { mockLogin, mockMe, mockLogout, mockSetInMemoryToken } = vi.hoisted(() => ({
  mockLogin: vi.fn(),
  mockMe: vi.fn(),
  mockLogout: vi.fn(),
  mockSetInMemoryToken: vi.fn(),
}));

vi.mock('../api/client', () => ({
  api: {
    login: mockLogin,
    me: mockMe,
    logout: mockLogout,
  },
  setInMemoryToken: mockSetInMemoryToken,
}));

import { AuthProvider, useAuth } from './AuthContext';

// Helper component to consume the context
function AuthConsumer() {
  const { user, loading, login, logout } = useAuth();
  return (
    <div>
      <span data-testid="loading">{String(loading)}</span>
      <span data-testid="user">{user ? user.username : 'null'}</span>
      <button data-testid="login-btn" onClick={() => login('admin', 'demo')}>Login</button>
      <button data-testid="logout-btn" onClick={logout}>Logout</button>
    </div>
  );
}

describe('AuthContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockLogout.mockResolvedValue({ success: true, data: null });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('useAuth throws outside AuthProvider', () => {
    // Suppress console.error from React error boundary
    const spy = vi.spyOn(console, 'error').mockImplementation(() => {});

    expect(() => render(<AuthConsumer />)).toThrow(
      'useAuth must be used within AuthProvider',
    );

    spy.mockRestore();
  });

  it('starts with loading=true and resolves to false when no cookie/token', async () => {
    mockMe.mockResolvedValue({ success: false, data: null });

    render(
      <AuthProvider>
        <AuthConsumer />
      </AuthProvider>,
    );

    // Resolves to loading=false after cookie auth attempt
    await waitFor(() => {
      expect(screen.getByTestId('loading').textContent).toBe('false');
    });

    expect(screen.getByTestId('user').textContent).toBe('null');
  });

  it('fetches user on mount via httpOnly cookie', async () => {
    mockMe.mockResolvedValue({
      success: true,
      data: { id: '1', username: 'alice', email: 'alice@test.com', name: 'Alice', role: 'user' },
    });

    render(
      <AuthProvider>
        <AuthConsumer />
      </AuthProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('user').textContent).toBe('alice');
    });
    expect(screen.getByTestId('loading').textContent).toBe('false');
  });

  it('login stores token in memory and sets user on success', async () => {
    const user = userEvent.setup();
    mockLogin.mockResolvedValue({
      success: true,
      data: { tokens: { access_token: 'new-jwt-token' } },
    });
    mockMe.mockResolvedValue({
      success: true,
      data: { id: '2', username: 'bob', email: 'bob@test.com', name: 'Bob', role: 'user' },
    });

    render(
      <AuthProvider>
        <AuthConsumer />
      </AuthProvider>,
    );

    // Wait for initial load
    await waitFor(() => {
      expect(screen.getByTestId('loading').textContent).toBe('false');
    });

    await user.click(screen.getByTestId('login-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('user').textContent).toBe('bob');
    });

    expect(mockSetInMemoryToken).toHaveBeenCalledWith('new-jwt-token');
  });

  it('login returns error on failure', async () => {
    const user = userEvent.setup();
    mockLogin.mockResolvedValue({
      success: false,
      data: null,
      error: { code: 'AUTH', message: 'Invalid credentials' },
    });

    // Need a component that captures login result
    function LoginResultConsumer() {
      const { login } = useAuth();
      const [result, setResult] = React.useState<string>('');
      return (
        <div>
          <button data-testid="do-login" onClick={async () => {
            const r = await login('admin', 'wrong');
            setResult(r.success ? 'ok' : r.error || 'error');
          }}>Login</button>
          <span data-testid="result">{result}</span>
        </div>
      );
    }

    render(
      <AuthProvider>
        <LoginResultConsumer />
      </AuthProvider>,
    );

    await user.click(screen.getByTestId('do-login'));

    await waitFor(() => {
      expect(screen.getByTestId('result').textContent).toBe('Invalid credentials');
    });
  });

  it('logout calls server, clears in-memory token, and resets user', async () => {
    const user = userEvent.setup();
    mockMe.mockResolvedValue({
      success: true,
      data: { id: '1', username: 'alice', email: 'alice@test.com', name: 'Alice', role: 'user' },
    });

    render(
      <AuthProvider>
        <AuthConsumer />
      </AuthProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('user').textContent).toBe('alice');
    });

    await user.click(screen.getByTestId('logout-btn'));

    await waitFor(() => {
      expect(screen.getByTestId('user').textContent).toBe('null');
    });
    expect(mockLogout).toHaveBeenCalledOnce();
    expect(mockSetInMemoryToken).toHaveBeenCalledWith(null);
  });

  it('shows no user when me() returns failure on mount (expired cookie)', async () => {
    mockMe.mockResolvedValue({ success: false, data: null });

    render(
      <AuthProvider>
        <AuthConsumer />
      </AuthProvider>,
    );

    await waitFor(() => {
      expect(screen.getByTestId('loading').textContent).toBe('false');
    });

    expect(screen.getByTestId('user').textContent).toBe('null');
  });

  it('login returns generic error when API gives no message', async () => {
    const user = userEvent.setup();
    mockLogin.mockResolvedValue({
      success: false,
      data: null,
    });

    function LoginResultConsumer() {
      const { login } = useAuth();
      const [result, setResult] = React.useState<string>('');
      return (
        <div>
          <button data-testid="do-login" onClick={async () => {
            const r = await login('admin', 'wrong');
            setResult(r.success ? 'ok' : r.error || 'fallback');
          }}>Login</button>
          <span data-testid="result">{result}</span>
        </div>
      );
    }

    render(
      <AuthProvider>
        <LoginResultConsumer />
      </AuthProvider>,
    );

    await user.click(screen.getByTestId('do-login'));

    await waitFor(() => {
      expect(screen.getByTestId('result').textContent).toBe('Login failed');
    });
  });
});
