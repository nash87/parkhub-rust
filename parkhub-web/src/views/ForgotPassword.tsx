import { useState } from 'react';
import { Link } from 'react-router-dom';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { CarSimple, ArrowLeft, SpinnerGap, CheckCircle } from '@phosphor-icons/react';
import { api } from '../api/client';

export function ForgotPasswordPage() {
  const { t } = useTranslation();
  const [email, setEmail] = useState('');
  const [loading, setLoading] = useState(false);
  const [sent, setSent] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    await api.forgotPassword(email);
    // Always show success (anti-enumeration — backend does the same)
    setSent(true);
    setLoading(false);
  }

  return (
    <div className="min-h-dvh bg-white dark:bg-surface-950 flex items-center justify-center px-6 py-12">
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

        {sent ? (
          <div className="space-y-4">
            <div className="flex items-center gap-3 text-emerald-600 dark:text-emerald-400">
              <CheckCircle weight="fill" className="w-6 h-6" />
              <h1 className="text-xl font-bold text-surface-900 dark:text-white">Check your email</h1>
            </div>
            <p className="text-surface-500 dark:text-surface-400 text-sm leading-relaxed">
              If an account with that email exists, we've sent a password reset link. Check your inbox and spam folder.
            </p>
            <Link to="/login" className="btn btn-primary w-full mt-6">
              Back to Sign In
            </Link>
          </div>
        ) : (
          <>
            <h1 className="text-2xl font-bold text-surface-900 dark:text-white mb-1">
              Reset password
            </h1>
            <p className="text-surface-500 dark:text-surface-400 text-sm mb-8">
              Enter your email and we'll send you a reset link.
            </p>

            <form onSubmit={handleSubmit} className="space-y-5">
              <div>
                <label htmlFor="email" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">
                  Email
                </label>
                <input
                  id="email"
                  type="email"
                  value={email}
                  onChange={e => setEmail(e.target.value)}
                  placeholder="you@company.com"
                  autoComplete="email"
                  className="input"
                  required
                />
              </div>

              <button
                type="submit"
                disabled={loading || !email}
                className="btn btn-primary w-full py-2.5 disabled:opacity-50"
              >
                {loading ? (
                  <><SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> Sending...</>
                ) : (
                  'Send reset link'
                )}
              </button>
            </form>
          </>
        )}
      </motion.div>
    </div>
  );
}
