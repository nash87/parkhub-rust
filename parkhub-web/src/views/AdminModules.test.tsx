import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallback?: string) => fallback ?? key,
  }),
}));

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
});
