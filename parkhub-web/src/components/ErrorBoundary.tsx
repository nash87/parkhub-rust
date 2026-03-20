import React from 'react';
import { CarSimple, ArrowClockwise, Warning } from '@phosphor-icons/react';

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends React.Component<
  { children: React.ReactNode },
  ErrorBoundaryState
> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    console.error('[ErrorBoundary]', error, errorInfo);
  }

  handleReload = () => {
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      return (
        <div role="alert" className="min-h-dvh bg-white dark:bg-surface-950 flex items-center justify-center">
          <div className="flex flex-col items-center gap-6 max-w-sm text-center px-6">
            <div className="w-16 h-16 rounded-xl bg-primary-600 flex items-center justify-center">
              <CarSimple weight="fill" className="w-8 h-8 text-white" />
            </div>

            <div className="flex items-center gap-2 text-amber-600 dark:text-amber-400">
              <Warning weight="fill" className="w-5 h-5" />
              <h1 className="text-lg font-semibold">Something went wrong</h1>
            </div>

            <p className="text-sm text-surface-500 dark:text-surface-400">
              An unexpected error occurred. Please try reloading the page.
            </p>

            {this.state.error && (
              <pre className="text-xs text-left w-full bg-surface-100 dark:bg-surface-800 rounded-xl p-4 overflow-auto max-h-32 text-surface-600 dark:text-surface-400">
                {this.state.error.message}
              </pre>
            )}

            <button onClick={this.handleReload} className="btn btn-primary">
              <ArrowClockwise weight="bold" className="w-4 h-4" />
              Reload Page
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
