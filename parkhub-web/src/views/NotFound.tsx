import { Link } from 'react-router-dom';
import { CarSimple, ArrowLeft } from '@phosphor-icons/react';

export function NotFoundPage() {
  return (
    <main className="min-h-dvh bg-white dark:bg-surface-950 flex items-center justify-center px-6">
      <div className="text-center max-w-sm" role="alert">
        <div className="w-12 h-12 rounded-lg bg-primary-600 flex items-center justify-center mx-auto mb-6">
          <CarSimple weight="fill" className="w-7 h-7 text-white" />
        </div>
        <p className="text-6xl font-extrabold text-surface-200 dark:text-surface-800 mb-2">404</p>
        <h1 className="text-xl font-bold text-surface-900 dark:text-white mb-2">Page not found</h1>
        <p className="text-surface-500 dark:text-surface-400 text-sm mb-8">
          The page you're looking for doesn't exist or has been moved.
        </p>
        <Link to="/" className="btn btn-primary inline-flex">
          <ArrowLeft weight="bold" className="w-4 h-4" />
          Back to Dashboard
        </Link>
      </div>
    </main>
  );
}
