import { useActionState, useState } from 'react';
import { Link } from 'react-router-dom';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { CarSimple, ArrowLeft, SpinnerGap, CheckCircle } from '@phosphor-icons/react';
import { api } from '../api/client';

/**
 * Forgot-password flow.
 *
 * Uses React 19's `useActionState` + form `action` attribute instead of the
 * classic `useState` + `onSubmit` + manual loading-boolean pattern. The
 * third tuple entry (`isPending`) replaces the hand-rolled loading flag;
 * the action swallows preventDefault and the returned state drives the
 * success panel.
 *
 * Anti-enumeration: we always show the success panel regardless of whether
 * the email exists — the backend does the same, and the action mirrors
 * that contract so the UI never branches on the result.
 */

interface ForgotPasswordState {
  sent: boolean;
}

const INITIAL_STATE: ForgotPasswordState = { sent: false };

async function forgotPasswordAction(
  _prev: ForgotPasswordState,
  formData: FormData,
): Promise<ForgotPasswordState> {
  const email = String(formData.get('email') ?? '').trim();
  if (!email) return { sent: false };
  // Errors are intentionally swallowed — the backend returns 200 regardless
  // so we don't leak account existence; mirror that at the UI.
  await api.forgotPassword(email).catch(() => undefined);
  return { sent: true };
}

export function ForgotPasswordPage() {
  const { t } = useTranslation();
  const [state, formAction, isPending] = useActionState(forgotPasswordAction, INITIAL_STATE);
  // Tracked locally so the submit button can disable while the input is
  // empty (pure-uncontrolled inputs don't expose emptiness to React).
  const [emailDraft, setEmailDraft] = useState('');

  return (
    <main className="min-h-dvh bg-white dark:bg-surface-950 flex items-center justify-center px-6 py-12">
      <motion.div
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        className="w-full max-w-sm"
      >
        <Link
          to="/login"
          className="inline-flex items-center gap-2 text-sm text-surface-500 hover:text-primary-600 mb-8 transition-colors"
        >
          <ArrowLeft weight="bold" className="w-4 h-4" />
          {t('auth.signIn')}
        </Link>

        <div className="flex items-center gap-3 mb-8">
          <div className="w-9 h-9 rounded-lg bg-primary-600 flex items-center justify-center">
            <CarSimple weight="fill" className="w-5 h-5 text-white" />
          </div>
          <span className="text-lg font-bold text-surface-900 dark:text-white tracking-tight">ParkHub</span>
        </div>

        {state.sent ? (
          <div className="space-y-4" role="status">
            <div className="flex items-center gap-3 text-emerald-600 dark:text-emerald-400">
              <CheckCircle weight="fill" className="w-6 h-6" />
              <h1 className="text-xl font-bold text-surface-900 dark:text-white">{t('forgotPassword.successTitle')}</h1>
            </div>
            <p className="text-surface-500 dark:text-surface-400 text-sm leading-relaxed">
              {t('forgotPassword.successMessage')}
            </p>
            <Link to="/login" className="btn btn-primary w-full mt-6">
              {t('forgotPassword.backToSignIn')}
            </Link>
          </div>
        ) : (
          <>
            <h1 className="text-2xl font-bold text-surface-900 dark:text-white mb-1">
              {t('forgotPassword.title')}
            </h1>
            <p className="text-surface-500 dark:text-surface-400 text-sm mb-8">
              {t('forgotPassword.subtitle')}
            </p>

            <form action={formAction} className="space-y-5">
              <div>
                <label htmlFor="email" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">
                  {t('forgotPassword.emailLabel')}
                </label>
                <input
                  id="email"
                  name="email"
                  type="email"
                  value={emailDraft}
                  onChange={(e) => setEmailDraft(e.target.value)}
                  placeholder={t('auth.email')}
                  autoComplete="email"
                  className="input"
                  required
                />
              </div>

              <button
                type="submit"
                disabled={isPending || !emailDraft.trim()}
                className="btn btn-primary w-full py-2.5 disabled:opacity-50"
              >
                {isPending ? (
                  <><SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> {t('forgotPassword.sending')}</>
                ) : (
                  t('forgotPassword.sendResetLink')
                )}
              </button>
            </form>
          </>
        )}
      </motion.div>
    </main>
  );
}
