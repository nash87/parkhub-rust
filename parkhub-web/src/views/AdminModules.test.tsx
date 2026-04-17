import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string, opts?: Record<string, unknown>) => {
      const base = fallback ?? key;
      if (opts && typeof base === 'string') {
        return Object.entries(opts).reduce(
          (acc, [k, v]) => acc.replace(new RegExp(`{{\\s*${k}\\s*}}`, 'g'), String(v)),
          base,
        );
      }
      return base;
    },
  }),
}));

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
vi.mock('react-hot-toast', () => ({
  default: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

// Auth mock — test file flips the role via setAuthUser() helper below.
interface TestUser {
  role: string;
}
let currentUser: TestUser | null = { role: 'admin' };
vi.mock('../context/AuthContext', () => ({
  useAuth: () => ({ user: currentUser, loading: false }),
}));

function setAuthUser(user: TestUser | null) {
  currentUser = user;
}

import { AdminModulesPage } from './AdminModules';

const fetchMock = vi.fn();

function renderPage() {
  return render(
    <MemoryRouter>
      <AdminModulesPage />
    </MemoryRouter>,
  );
}

function mkModule(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    name: 'bookings',
    category: 'core',
    description: 'Core bookings',
    enabled: true,
    runtime_toggleable: false,
    runtime_enabled: true,
    config_keys: ['booking.min-duration'],
    ui_route: '/bookings',
    depends_on: [],
    version: '4.12.0',
    ...overrides,
  };
}

describe('AdminModulesPage', () => {
  beforeEach(() => {
    vi.stubGlobal('fetch', fetchMock);
    setAuthUser({ role: 'admin' });
    toastSuccessMock.mockReset();
    toastErrorMock.mockReset();
  });

  afterEach(() => {
    fetchMock.mockReset();
    vi.unstubAllGlobals();
  });

  it('renders the empty-state when the server returns no modules', async () => {
    fetchMock.mockResolvedValue({
      ok: true,
      json: async () => ({ data: [], module_info: [] }),
    });

    renderPage();

    // Header lands immediately (independent of the fetch outcome)
    expect(screen.getByText('Modules')).toBeInTheDocument();

    // Once the empty response settles, the grid shows the no-matches copy
    await waitFor(() => {
      expect(screen.getByText('No modules match your filters.')).toBeInTheDocument();
    });

    // 0/0 active summary
    expect(screen.getByText(/0\s*\/\s*0\s*active/)).toBeInTheDocument();
  });

  it('renders grouped module cards and category headings for a populated response', async () => {
    fetchMock.mockResolvedValue({
      ok: true,
      json: async () => ({
        module_info: [
          mkModule({ name: 'bookings', category: 'core' }),
          mkModule({
            name: 'stripe',
            category: 'payment',
            description: 'Stripe integration',
            enabled: false,
            runtime_enabled: false,
            config_keys: ['stripe.api-key'],
            ui_route: null,
          }),
          mkModule({
            name: 'absences',
            category: 'booking',
            description: 'Home-office & vacation',
            ui_route: '/absences',
          }),
        ],
      }),
    });

    renderPage();

    await waitFor(() => {
      expect(screen.getByTestId('module-card-bookings')).toBeInTheDocument();
    });
    expect(screen.getByTestId('module-card-stripe')).toBeInTheDocument();
    expect(screen.getByTestId('module-card-absences')).toBeInTheDocument();

    // Category headings (h2) render for each group with modules
    const headings = screen.getAllByRole('heading', { level: 2 }).map((h) => h.textContent);
    expect(headings.some((h) => h?.startsWith('Core'))).toBe(true);
    expect(headings.some((h) => h?.startsWith('Payment'))).toBe(true);
    expect(headings.some((h) => h?.startsWith('Booking'))).toBe(true);

    // 2/3 active — stripe is disabled
    expect(screen.getByText(/2\s*\/\s*3\s*active/)).toBeInTheDocument();
  });

  it('surfaces fetch errors without crashing', async () => {
    fetchMock.mockResolvedValue({ ok: false, status: 500 });
    renderPage();
    await waitFor(() => {
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
  });

  // ── T-1720 v2: runtime toggle ──

  it('shows a working switch for runtime_toggleable modules and a disabled switch otherwise', async () => {
    fetchMock.mockResolvedValue({
      ok: true,
      json: async () => ({
        module_info: [
          mkModule({
            name: 'toggleme',
            category: 'core',
            runtime_toggleable: true,
            runtime_enabled: true,
          }),
          mkModule({
            name: 'frozen',
            category: 'core',
            runtime_toggleable: false,
            runtime_enabled: true,
          }),
        ],
      }),
    });

    renderPage();

    const toggleable = await screen.findByTestId('module-toggle-toggleme');
    const frozen = await screen.findByTestId('module-toggle-frozen');

    expect(toggleable).toHaveAttribute('aria-checked', 'true');
    expect(toggleable).not.toBeDisabled();

    expect(frozen).toBeDisabled();
    expect(frozen.getAttribute('aria-label')).toContain('Not runtime toggleable');
  });

  it('PATCHes the module API with the correct payload when toggled', async () => {
    fetchMock.mockImplementation((url: string, init?: { method?: string; body?: string }) => {
      if (typeof url === 'string' && url.includes('/api/v1/modules/info')) {
        return Promise.resolve({
          ok: true,
          status: 200,
          json: async () => ({
            module_info: [
              mkModule({
                name: 'toggleme',
                category: 'core',
                runtime_toggleable: true,
                runtime_enabled: true,
              }),
            ],
          }),
        });
      }
      if (typeof url === 'string' && url.includes('/api/v1/admin/modules/')) {
        expect(init?.method).toBe('PATCH');
        expect(JSON.parse(init?.body ?? '{}')).toEqual({ runtime_enabled: false });
        return Promise.resolve({
          ok: true,
          status: 200,
          json: async () => ({
            success: true,
            data: mkModule({
              name: 'toggleme',
              runtime_toggleable: true,
              runtime_enabled: false,
            }),
          }),
        });
      }
      return Promise.resolve({ ok: false, status: 404, json: async () => null });
    });

    renderPage();

    const toggle = await screen.findByTestId('module-toggle-toggleme');
    expect(toggle).toHaveAttribute('aria-checked', 'true');

    fireEvent.click(toggle);

    await waitFor(() => {
      expect(toggle).toHaveAttribute('aria-checked', 'false');
    });
    expect(toastSuccessMock).toHaveBeenCalled();

    // Confirm the PATCH was issued with the encoded module name.
    const patchCall = fetchMock.mock.calls.find(
      (c) => typeof c[0] === 'string' && c[0].includes('/api/v1/admin/modules/'),
    );
    expect(patchCall?.[0]).toContain('/api/v1/admin/modules/toggleme');
  });

  it('reverts the switch and shows an error toast when the server returns 409', async () => {
    fetchMock.mockImplementation((url: string) => {
      if (typeof url === 'string' && url.includes('/api/v1/modules/info')) {
        return Promise.resolve({
          ok: true,
          status: 200,
          json: async () => ({
            module_info: [
              mkModule({
                name: 'toggleme',
                category: 'core',
                runtime_toggleable: true,
                runtime_enabled: true,
              }),
            ],
          }),
        });
      }
      if (typeof url === 'string' && url.includes('/api/v1/admin/modules/')) {
        return Promise.resolve({
          ok: false,
          status: 409,
          json: async () => ({
            success: false,
            error: { code: 'CONFLICT', message: 'Dependency violation' },
          }),
        });
      }
      return Promise.resolve({ ok: false, status: 404, json: async () => null });
    });

    renderPage();

    const toggle = await screen.findByTestId('module-toggle-toggleme');
    expect(toggle).toHaveAttribute('aria-checked', 'true');

    fireEvent.click(toggle);

    // Eventually the switch reverts back to true (optimistic flip → error → revert).
    await waitFor(() => {
      expect(toggle).toHaveAttribute('aria-checked', 'true');
    });

    // Error toast was emitted.
    expect(toastErrorMock).toHaveBeenCalled();
    expect(toastSuccessMock).not.toHaveBeenCalled();
  });

  it('hides the switch entirely for non-admins', async () => {
    setAuthUser({ role: 'user' });
    fetchMock.mockResolvedValue({
      ok: true,
      json: async () => ({
        module_info: [
          mkModule({
            name: 'toggleme',
            category: 'core',
            runtime_toggleable: true,
            runtime_enabled: true,
          }),
        ],
      }),
    });

    renderPage();

    await screen.findByTestId('module-card-toggleme');
    expect(screen.queryByTestId('module-toggle-toggleme')).toBeNull();
  });
});
