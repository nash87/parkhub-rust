import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

vi.mock('@phosphor-icons/react', () => ({
  CarSimple: (props: any) => <span data-testid="icon-car" {...props} />,
  ArrowClockwise: (props: any) => <span data-testid="icon-reload" {...props} />,
  Warning: (props: any) => <span data-testid="icon-warning" {...props} />,
}));

import { ErrorBoundary } from './ErrorBoundary';

// ── Test components ──

function ThrowingComponent({ message }: { message: string }): React.ReactElement {
  throw new Error(message);
}

function GoodComponent() {
  return <div data-testid="good-component">All good</div>;
}

function ThrowingWithStackComponent(): React.ReactElement {
  const err = new Error('Stack trace error');
  err.stack = 'Error: Stack trace error\n    at ThrowingWithStackComponent (file.tsx:1:1)';
  throw err;
}

describe('ErrorBoundary (extended)', () => {
  const originalConsoleError = console.error;
  const mockReload = vi.fn();
  const originalLocation = window.location;

  beforeEach(() => {
    console.error = vi.fn();
    Object.defineProperty(window, 'location', {
      writable: true,
      value: { ...originalLocation, reload: mockReload },
    });
    mockReload.mockClear();
  });

  afterEach(() => {
    console.error = originalConsoleError;
    Object.defineProperty(window, 'location', { writable: true, value: originalLocation });
    vi.restoreAllMocks();
  });

  // ── Error recovery ──

  it('renders error UI with role="alert" for accessibility', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent message="a11y test" />
      </ErrorBoundary>,
    );
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('displays the exact error message text', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent message="Detailed failure reason" />
      </ErrorBoundary>,
    );
    expect(screen.getByText('Detailed failure reason')).toBeInTheDocument();
  });

  it('error message is rendered in a pre element for formatting', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent message="Formatted error" />
      </ErrorBoundary>,
    );
    const pre = screen.getByText('Formatted error');
    expect(pre.tagName.toLowerCase()).toBe('pre');
  });

  // ── Error reporting (componentDidCatch) ──

  it('logs error and error info via console.error', () => {
    const spy = console.error as ReturnType<typeof vi.fn>;
    render(
      <ErrorBoundary>
        <ThrowingComponent message="logged error" />
      </ErrorBoundary>,
    );
    // React itself calls console.error, plus our componentDidCatch
    const calls = spy.mock.calls.map(c => c[0]);
    const hasBoundaryLog = calls.some(
      c => typeof c === 'string' && c.includes('[ErrorBoundary]')
    );
    expect(hasBoundaryLog).toBe(true);
  });

  // ── Reload button ──

  it('reload button triggers window.location.reload', async () => {
    const user = userEvent.setup();
    render(
      <ErrorBoundary>
        <ThrowingComponent message="reload test" />
      </ErrorBoundary>,
    );

    const reloadBtn = screen.getByText('Reload Page');
    await user.click(reloadBtn);

    expect(mockReload).toHaveBeenCalledOnce();
  });

  it('reload button has the reload icon', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent message="icon test" />
      </ErrorBoundary>,
    );
    const reloadBtn = screen.getByText('Reload Page').closest('button');
    expect(reloadBtn?.querySelector('[data-testid="icon-reload"]')).toBeTruthy();
  });

  // ── Fallback UI rendering ──

  it('shows ParkHub car icon in error state', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent message="car icon test" />
      </ErrorBoundary>,
    );
    expect(screen.getByTestId('icon-car')).toBeInTheDocument();
  });

  it('shows warning icon in error state', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent message="warning test" />
      </ErrorBoundary>,
    );
    expect(screen.getByTestId('icon-warning')).toBeInTheDocument();
  });

  it('renders title and description text', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent message="title test" />
      </ErrorBoundary>,
    );
    // i18n keys rendered via i18n.t() (not useTranslation)
    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getByText('An unexpected error occurred. Please try reloading the page.')).toBeInTheDocument();
  });

  // ── Multiple children ──

  it('renders first child that does not throw', () => {
    render(
      <ErrorBoundary>
        <GoodComponent />
      </ErrorBoundary>,
    );
    expect(screen.getByTestId('good-component')).toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  // ── Error with long message ──

  it('handles very long error messages without layout breakage', () => {
    const longMessage = 'E'.repeat(500);
    render(
      <ErrorBoundary>
        <ThrowingComponent message={longMessage} />
      </ErrorBoundary>,
    );
    expect(screen.getByText(longMessage)).toBeInTheDocument();
    // The pre element should have overflow-auto class for scrollability
    const pre = screen.getByText(longMessage);
    expect(pre.className).toContain('overflow-auto');
  });

  // ── Error with special characters ──

  it('handles error messages with HTML-like content safely', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent message="<script>alert('xss')</script>" />
      </ErrorBoundary>,
    );
    // Should render as text, not as HTML
    expect(screen.getByText("<script>alert('xss')</script>")).toBeInTheDocument();
  });

  // ── Nested error boundaries ──

  it('inner boundary catches error without affecting outer', () => {
    render(
      <ErrorBoundary>
        <div data-testid="outer-content">
          <ErrorBoundary>
            <ThrowingComponent message="inner crash" />
          </ErrorBoundary>
        </div>
      </ErrorBoundary>,
    );
    // Outer content should still be rendered
    expect(screen.getByTestId('outer-content')).toBeInTheDocument();
    // Inner boundary should show error UI
    expect(screen.getByText('inner crash')).toBeInTheDocument();
  });
});
