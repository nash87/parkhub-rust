import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ShieldCheck } from '@phosphor-icons/react';

interface SsoProviderPublic {
  slug: string;
  display_name: string;
  enabled: boolean;
}

/** Fetch configured SSO providers from the backend. */
async function fetchSsoProviders(): Promise<SsoProviderPublic[]> {
  try {
    const base = (import.meta as Record<string, any>).env?.VITE_API_URL || '';
    const res = await fetch(`${base}/api/v1/auth/sso/providers`, {
      headers: { Accept: 'application/json' },
    });
    if (!res.ok) return [];
    const json = await res.json();
    return json?.data?.providers || [];
  } catch {
    return [];
  }
}

/** Initiate SSO login for a given provider. */
async function initiateSsoLogin(slug: string): Promise<void> {
  try {
    const base = (import.meta as Record<string, any>).env?.VITE_API_URL || '';
    const res = await fetch(`${base}/api/v1/auth/sso/${slug}/login`);
    if (!res.ok) throw new Error('SSO login failed');
    const json = await res.json();
    if (json?.redirect_url) {
      window.location.href = json.redirect_url;
    }
  } catch (err) {
    console.error('SSO login error:', err);
  }
}

/**
 * SSO / SAML Login buttons.
 *
 * Only renders buttons for providers that are configured and enabled.
 * If no SSO providers are configured, renders nothing.
 */
export function SSOButtons() {
  const { t } = useTranslation();
  const [providers, setProviders] = useState<SsoProviderPublic[]>([]);

  useEffect(() => {
    fetchSsoProviders().then(setProviders);
  }, []);

  if (providers.length === 0) return null;

  return (
    <div className="flex flex-col gap-2">
      {providers.map((provider) => (
        <button
          key={provider.slug}
          onClick={() => initiateSsoLogin(provider.slug)}
          className="flex items-center justify-center gap-3 w-full py-2.5 px-4 rounded-lg border border-surface-200 dark:border-surface-700 bg-white dark:bg-surface-800 text-surface-700 dark:text-surface-200 hover:bg-surface-50 dark:hover:bg-surface-700 transition-colors font-medium text-sm"
          aria-label={t('sso.continueWith', { provider: provider.display_name })}
        >
          <ShieldCheck size={20} weight="bold" className="text-primary-500" />
          {t('sso.continueWith', { provider: provider.display_name })}
        </button>
      ))}
    </div>
  );
}
