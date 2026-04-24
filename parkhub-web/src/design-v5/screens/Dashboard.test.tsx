import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';

vi.mock('../Toast', () => ({
  useV5Toast: () => vi.fn(),
  V5ToastProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('@number-flow/react', () => ({
  default: ({ value }: { value: number | string }) => <span>{value}</span>,
}));

import { DashboardV5 } from './Dashboard';

describe('DashboardV5 — credits progressbar a11y', () => {
  it('exposes role=progressbar with aria-label, valuenow, valuemin, valuemax', () => {
    render(<DashboardV5 navigate={vi.fn()} />);
    const meter = screen.getByRole('progressbar', { name: /\d+ von \d+ Credits/ });
    expect(meter).toHaveAttribute('aria-valuenow', '40');
    expect(meter).toHaveAttribute('aria-valuemin', '0');
    expect(meter).toHaveAttribute('aria-valuemax', '40');
  });
});

describe('DashboardV5 — recommendation card branding', () => {
  it('renders pattern-based recommendation card without AI/KI branding', () => {
    render(<DashboardV5 navigate={vi.fn()} />);
    expect(screen.getByText('Empfehlung')).toBeInTheDocument();
    expect(screen.queryByText('KI-Empfehlung')).toBeNull();
    expect(screen.queryByText(/^KI$/)).toBeNull();
  });
});
