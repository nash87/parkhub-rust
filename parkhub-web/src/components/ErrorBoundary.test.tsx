import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';

// ── Mocks ──

vi.mock('@phosphor-icons/react', () => ({
  CarSimple: (props: any) => <span data-testid="icon-car" {...props} />,
  ArrowClockwise: (props: any) => <span data-testid="icon-reload" {...props} />,
  Warning: (props: any) => <span data-testid="icon-warning" {...props} />,
}));

import { ErrorBoundary } from './ErrorBoundary';

// A component that throws an error
function ThrowingComponent({ message }: { message: string }) {
  throw new Error(message);
}

// A component that renders normally
function GoodComponent() {
  return <div data-testid="good-component">All good</div>;
}

describe('ErrorBoundary', () => {
  // Suppress console.error noise from React error boundaries in test output
  const originalConsoleError = console.error;
  beforeEach(() => {
    console.error = vi.fn();
  });
  afterEach(() => {
    console.error = originalConsoleError;
    vi.restoreAllMocks();
  });

  it('renders children when there is no error', () => {
    render(
      <ErrorBoundary>
        <GoodComponent />
      </ErrorBoundary>
    );
    expect(screen.getByTestId('good-component')).toBeInTheDocument();
    expect(screen.getByText('All good')).toBeInTheDocument();
  });

  it('renders error UI when a child throws', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent message="Test explosion" />
      </ErrorBoundary>
    );

    expect(screen.getByText('Something went wrong')).toBeInTheDocument();
    expect(screen.getByText('An unexpected error occurred. Please try reloading the page.')).toBeInTheDocument();
    expect(screen.getByText('Test explosion')).toBeInTheDocument();
  });

  it('renders the car icon in error state', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent message="crash" />
      </ErrorBoundary>
    );
    expect(screen.getByTestId('icon-car')).toBeInTheDocument();
  });

  it('renders the warning icon', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent message="crash" />
      </ErrorBoundary>
    );
    expect(screen.getByTestId('icon-warning')).toBeInTheDocument();
  });

  it('renders a reload button', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent message="crash" />
      </ErrorBoundary>
    );
    expect(screen.getByText('Reload Page')).toBeInTheDocument();
    expect(screen.getByTestId('icon-reload')).toBeInTheDocument();
  });

  it('displays the error message in a pre block', () => {
    render(
      <ErrorBoundary>
        <ThrowingComponent message="Something broke badly" />
      </ErrorBoundary>
    );
    const pre = screen.getByText('Something broke badly');
    expect(pre.tagName.toLowerCase()).toBe('pre');
  });
});
