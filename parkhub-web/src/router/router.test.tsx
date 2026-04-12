import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, Routes, Route, Navigate } from 'react-router-dom';

// ── Mocks ──

const mockUser = { current: null as any };

vi.mock('../context/AuthContext', () => ({
  AuthProvider: ({ children }: any) => <>{children}</>,
  useAuth: () => ({
    user: mockUser.current,
    loading: false,
    login: vi.fn(),
    logout: vi.fn(),
    refreshUser: vi.fn(),
  }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, ...props }: any, ref: any) => <div ref={ref} {...props}>{children}</div>),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () =>
  new Proxy({}, { get: (_, name) => (props: any) => <span data-testid={`icon-${String(name)}`} {...props} /> })
);

// ── Minimal route guards (replicate App.tsx logic) ──

function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { useAuth } = require('../context/AuthContext');
  const { user } = useAuth();
  if (!user) return <Navigate to="/login" replace />;
  return <>{children}</>;
}

function AdminRoute({ children }: { children: React.ReactNode }) {
  const { useAuth } = require('../context/AuthContext');
  const { user } = useAuth();
  if (!user || !['admin', 'superadmin'].includes(user.role)) return <Navigate to="/" replace />;
  return <>{children}</>;
}

// ── Simple page stubs ──

function DashboardStub() { return <div data-testid="page-dashboard">Dashboard</div>; }
function LoginStub() { return <div data-testid="page-login">Login</div>; }
function BookStub() { return <div data-testid="page-book">Book a Spot</div>; }
function ProfileStub() { return <div data-testid="page-profile">Profile</div>; }
function AdminStub() { return <div data-testid="page-admin">Admin Panel</div>; }
function NotFoundStub() { return <div data-testid="page-not-found">Page Not Found</div>; }

function TestApp({ initialRoute }: { initialRoute: string }) {
  return (
    <MemoryRouter initialEntries={[initialRoute]}>
      <Routes>
        <Route path="/login" element={<LoginStub />} />
        <Route path="/" element={<ProtectedRoute><DashboardStub /></ProtectedRoute>} />
        <Route path="/book" element={<ProtectedRoute><BookStub /></ProtectedRoute>} />
        <Route path="/profile" element={<ProtectedRoute><ProfileStub /></ProtectedRoute>} />
        <Route path="/admin" element={<ProtectedRoute><AdminRoute><AdminStub /></AdminRoute></ProtectedRoute>} />
        <Route path="*" element={<NotFoundStub />} />
      </Routes>
    </MemoryRouter>
  );
}

describe('Router', () => {
  beforeEach(() => {
    mockUser.current = null;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  // ── Public routes ──

  it('renders login page at /login', () => {
    render(<TestApp initialRoute="/login" />);
    expect(screen.getByTestId('page-login')).toBeInTheDocument();
  });

  // ── Protected route redirects ──

  it('redirects unauthenticated user from / to /login', () => {
    render(<TestApp initialRoute="/" />);
    expect(screen.getByTestId('page-login')).toBeInTheDocument();
    expect(screen.queryByTestId('page-dashboard')).not.toBeInTheDocument();
  });

  it('redirects unauthenticated user from /book to /login', () => {
    render(<TestApp initialRoute="/book" />);
    expect(screen.getByTestId('page-login')).toBeInTheDocument();
  });

  it('redirects unauthenticated user from /profile to /login', () => {
    render(<TestApp initialRoute="/profile" />);
    expect(screen.getByTestId('page-login')).toBeInTheDocument();
  });

  // ── Authenticated routes ──

  it('renders dashboard for authenticated user at /', () => {
    mockUser.current = { id: '1', username: 'alice', role: 'user' };
    render(<TestApp initialRoute="/" />);
    expect(screen.getByTestId('page-dashboard')).toBeInTheDocument();
  });

  it('renders book page for authenticated user at /book', () => {
    mockUser.current = { id: '1', username: 'alice', role: 'user' };
    render(<TestApp initialRoute="/book" />);
    expect(screen.getByTestId('page-book')).toBeInTheDocument();
  });

  it('renders profile page for authenticated user at /profile', () => {
    mockUser.current = { id: '1', username: 'alice', role: 'user' };
    render(<TestApp initialRoute="/profile" />);
    expect(screen.getByTestId('page-profile')).toBeInTheDocument();
  });

  // ── Admin route guard ──

  it('redirects non-admin user from /admin to /', () => {
    mockUser.current = { id: '1', username: 'alice', role: 'user' };
    render(<TestApp initialRoute="/admin" />);
    // Non-admin gets redirected to / which renders dashboard
    expect(screen.getByTestId('page-dashboard')).toBeInTheDocument();
    expect(screen.queryByTestId('page-admin')).not.toBeInTheDocument();
  });

  it('renders admin page for admin user at /admin', () => {
    mockUser.current = { id: '1', username: 'admin', role: 'admin' };
    render(<TestApp initialRoute="/admin" />);
    expect(screen.getByTestId('page-admin')).toBeInTheDocument();
  });

  it('renders admin page for superadmin user at /admin', () => {
    mockUser.current = { id: '1', username: 'super', role: 'superadmin' };
    render(<TestApp initialRoute="/admin" />);
    expect(screen.getByTestId('page-admin')).toBeInTheDocument();
  });

  // ── 404 ──

  it('renders 404 page for unknown routes', () => {
    render(<TestApp initialRoute="/this-does-not-exist" />);
    expect(screen.getByTestId('page-not-found')).toBeInTheDocument();
  });

  it('renders 404 for deep unknown routes', () => {
    render(<TestApp initialRoute="/admin/nonexistent/deep/path" />);
    expect(screen.getByTestId('page-not-found')).toBeInTheDocument();
  });

  // ── Deep linking ──

  it('deep links to /book for authenticated user', () => {
    mockUser.current = { id: '1', username: 'alice', role: 'user' };
    render(<TestApp initialRoute="/book" />);
    expect(screen.getByTestId('page-book')).toBeInTheDocument();
    expect(screen.getByText('Book a Spot')).toBeInTheDocument();
  });

  it('deep links to /profile for authenticated user', () => {
    mockUser.current = { id: '1', username: 'alice', role: 'user' };
    render(<TestApp initialRoute="/profile" />);
    expect(screen.getByTestId('page-profile')).toBeInTheDocument();
  });

  it('deep links to /admin for admin user', () => {
    mockUser.current = { id: '1', username: 'admin', role: 'admin' };
    render(<TestApp initialRoute="/admin" />);
    expect(screen.getByTestId('page-admin')).toBeInTheDocument();
    expect(screen.getByText('Admin Panel')).toBeInTheDocument();
  });
});
