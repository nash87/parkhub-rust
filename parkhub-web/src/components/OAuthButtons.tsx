import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

interface OAuthProviders {
  google: boolean;
  github: boolean;
}

/** Fetch which OAuth providers are configured from the backend. */
async function fetchProviders(): Promise<OAuthProviders> {
  try {
    const base = (import.meta as Record<string, any>).env?.VITE_API_URL || '';
    const res = await fetch(`${base}/api/v1/auth/oauth/providers`, {
      headers: { Accept: 'application/json' },
    });
    if (!res.ok) return { google: false, github: false };
    const json = await res.json();
    return json?.data || { google: false, github: false };
  } catch {
    return { google: false, github: false };
  }
}

/** Google icon SVG (brand colors) */
function GoogleIcon() {
  return (
    <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
      <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92a5.06 5.06 0 0 1-2.2 3.32v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.1z" fill="#4285F4" />
      <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853" />
      <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18A10.96 10.96 0 0 0 1 12c0 1.77.42 3.45 1.18 4.93l3.66-2.84z" fill="#FBBC05" />
      <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" fill="#EA4335" />
    </svg>
  );
}

/** GitHub icon SVG */
function GitHubIcon() {
  return (
    <svg viewBox="0 0 24 24" width="20" height="20" fill="currentColor" aria-hidden="true">
      <path d="M12 2C6.477 2 2 6.477 2 12c0 4.42 2.865 8.166 6.839 9.49.5.092.682-.217.682-.482 0-.237-.009-.866-.013-1.7-2.782.604-3.369-1.34-3.369-1.34-.454-1.156-1.11-1.463-1.11-1.463-.908-.62.069-.608.069-.608 1.003.07 1.531 1.03 1.531 1.03.892 1.529 2.341 1.087 2.91.831.092-.646.35-1.086.636-1.336-2.22-.253-4.555-1.11-4.555-4.943 0-1.091.39-1.984 1.029-2.683-.103-.253-.446-1.27.098-2.647 0 0 .84-.269 2.75 1.025A9.578 9.578 0 0 1 12 6.836a9.59 9.59 0 0 1 2.504.337c1.909-1.294 2.747-1.025 2.747-1.025.546 1.377.203 2.394.1 2.647.64.699 1.028 1.592 1.028 2.683 0 3.842-2.339 4.687-4.566 4.935.359.309.678.919.678 1.852 0 1.336-.012 2.415-.012 2.743 0 .267.18.578.688.48C19.138 20.161 22 16.416 22 12c0-5.523-4.477-10-10-10z" />
    </svg>
  );
}

/**
 * OAuth / Social Login buttons.
 *
 * Only renders buttons for providers that are actually configured on the backend.
 * If no providers are configured, renders nothing.
 */
export function OAuthButtons() {
  const { t } = useTranslation();
  const [providers, setProviders] = useState<OAuthProviders | null>(null);

  useEffect(() => {
    fetchProviders().then(setProviders);
  }, []);

  // Don't render anything if providers haven't loaded or none are configured
  if (!providers || (!providers.google && !providers.github)) {
    return null;
  }

  const base = (import.meta as Record<string, any>).env?.VITE_API_URL || '';

  return (
    <div className="space-y-3">
      {providers.google && (
        <a
          href={`${base}/api/v1/auth/oauth/google`}
          className="flex items-center justify-center gap-3 w-full py-2.5 px-4 rounded-lg border border-surface-200 dark:border-surface-700 bg-white dark:bg-surface-800 text-surface-700 dark:text-surface-200 font-medium text-sm hover:bg-surface-50 dark:hover:bg-surface-750 transition-colors shadow-sm"
          data-testid="oauth-google"
        >
          <GoogleIcon />
          {t('auth.continueWithGoogle')}
        </a>
      )}

      {providers.github && (
        <a
          href={`${base}/api/v1/auth/oauth/github`}
          className="flex items-center justify-center gap-3 w-full py-2.5 px-4 rounded-lg bg-surface-900 dark:bg-surface-100 text-white dark:text-surface-900 font-medium text-sm hover:bg-surface-800 dark:hover:bg-surface-200 transition-colors shadow-sm"
          data-testid="oauth-github"
        >
          <GitHubIcon />
          {t('auth.continueWithGitHub')}
        </a>
      )}

      <div className="flex items-center gap-3 my-1">
        <div className="flex-1 h-px bg-surface-200 dark:bg-surface-700" />
        <span className="text-xs text-surface-400 dark:text-surface-500 uppercase tracking-wider">{t('auth.orContinueWith')}</span>
        <div className="flex-1 h-px bg-surface-200 dark:bg-surface-700" />
      </div>
    </div>
  );
}
