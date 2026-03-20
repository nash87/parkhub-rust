import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';

// ── Mocks ──

vi.mock('react-router-dom', () => ({
  Link: ({ to, children, ...props }: any) => <a href={to} {...props}>{children}</a>,
}));

vi.mock('@phosphor-icons/react', () => ({
  CarSimple: (props: any) => <span data-testid="icon-car" {...props} />,
  ArrowLeft: (props: any) => <span data-testid="icon-arrow-left" {...props} />,
}));

import { NotFoundPage } from './NotFound';

describe('NotFoundPage', () => {
  it('renders 404 text', () => {
    render(<NotFoundPage />);
    expect(screen.getByText('404')).toBeInTheDocument();
  });

  it('renders "Page not found" heading', () => {
    render(<NotFoundPage />);
    expect(screen.getByText('Page not found')).toBeInTheDocument();
  });

  it('renders the descriptive message', () => {
    render(<NotFoundPage />);
    expect(screen.getByText(/doesn't exist or has been moved/)).toBeInTheDocument();
  });

  it('renders a link back to dashboard', () => {
    render(<NotFoundPage />);
    const link = screen.getByText('Back to Dashboard');
    expect(link).toBeInTheDocument();
    expect(link.closest('a')).toHaveAttribute('href', '/');
  });

  it('renders the car icon', () => {
    render(<NotFoundPage />);
    expect(screen.getByTestId('icon-car')).toBeInTheDocument();
  });

  it('renders the back arrow icon', () => {
    render(<NotFoundPage />);
    expect(screen.getByTestId('icon-arrow-left')).toBeInTheDocument();
  });
});
