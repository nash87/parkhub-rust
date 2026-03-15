import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { CarSimple, Eye, EyeSlash, SpinnerGap, ArrowLeft, Info } from '@phosphor-icons/react';
import { useAuth } from '../context/AuthContext';

export function LoginPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { login, user } = useAuth();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  // Redirect if already logged in
  if (user) {
    navigate('/', { replace: true });
    return null;
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setError('');
    setLoading(true);

    const result = await login(username, password);
    if (result.success) {
      navigate('/', { replace: true });
    } else {
      setError(result.error || t('auth.loginError'));
    }
    setLoading(false);
  }

  return (
    <div className="min-h-dvh mesh-gradient flex items-center justify-center px-4 py-12">
      <motion.div
        initial={{ opacity: 0, y: 30 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, ease: [0.22, 1, 0.36, 1] }}
        className="w-full max-w-md"
      >
        {/* Back to welcome */}
        <Link
          to="/welcome"
          className="inline-flex items-center gap-2 text-sm text-surface-500 hover:text-primary-600 mb-8 transition-colors"
        >
          <ArrowLeft weight="bold" className="w-4 h-4" />
          {t('welcome.greeting')}
        </Link>

        <div className="glass-card p-8 sm:p-10">
          {/* Header */}
          <div className="text-center mb-8">
            <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-primary-500 to-primary-600 flex items-center justify-center mx-auto mb-4 shadow-lg shadow-primary-500/20">
              <CarSimple weight="fill" className="w-8 h-8 text-white" />
            </div>
            <h1 className="text-2xl font-bold text-surface-900 dark:text-white">
              {t('auth.login')}
            </h1>
            <p className="text-surface-500 dark:text-surface-400 mt-1">
              {t('auth.loginSubtitle')}
            </p>
          </div>

          {/* Demo hint */}
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            transition={{ delay: 0.3 }}
            className="bg-primary-50 dark:bg-primary-900/20 border border-primary-200 dark:border-primary-800 rounded-xl p-3 mb-6"
          >
            <div className="flex items-start gap-2">
              <Info weight="fill" className="w-4 h-4 text-primary-600 dark:text-primary-400 mt-0.5 flex-shrink-0" />
              <p className="text-sm text-primary-700 dark:text-primary-300 font-mono">
                {t('auth.demoHint')}
              </p>
            </div>
          </motion.div>

          {/* Form */}
          <form onSubmit={handleSubmit} className="space-y-5">
            <div>
              <label htmlFor="username" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">
                {t('auth.email')}
              </label>
              <input
                id="username"
                type="text"
                value={username}
                onChange={e => setUsername(e.target.value)}
                className="input"
                placeholder="admin@parkhub-demo.de"
                autoComplete="username"
                required
                autoFocus
              />
            </div>

            <div>
              <div className="flex items-center justify-between mb-1.5">
                <label htmlFor="password" className="block text-sm font-medium text-surface-700 dark:text-surface-300">
                  {t('auth.password')}
                </label>
                <Link to="/forgot-password" className="text-sm text-primary-600 hover:text-primary-500 font-medium">
                  {t('auth.forgotPassword')}
                </Link>
              </div>
              <div className="relative">
                <input
                  id="password"
                  type={showPassword ? 'text' : 'password'}
                  value={password}
                  onChange={e => setPassword(e.target.value)}
                  className="input pr-11"
                  placeholder="ParkHub2026!"
                  autoComplete="current-password"
                  required
                />
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-surface-400 hover:text-surface-600"
                  aria-label={showPassword ? 'Hide password' : 'Show password'}
                >
                  {showPassword ? <EyeSlash weight="regular" className="w-5 h-5" /> : <Eye weight="regular" className="w-5 h-5" />}
                </button>
              </div>
            </div>

            {error && (
              <motion.div
                initial={{ opacity: 0, y: -10 }}
                animate={{ opacity: 1, y: 0 }}
                className="bg-danger/10 border border-danger/20 rounded-xl px-4 py-3 text-sm text-danger font-medium"
                role="alert"
              >
                {error}
              </motion.div>
            )}

            <button
              type="submit"
              disabled={loading || !username || !password}
              className="btn btn-primary w-full py-3 text-base disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {loading ? (
                <><SpinnerGap weight="bold" className="w-5 h-5 animate-spin" /> {t('auth.loggingIn')}</>
              ) : (
                t('auth.signIn')
              )}
            </button>
          </form>

          {/* Register link */}
          <p className="text-center text-sm text-surface-500 dark:text-surface-400 mt-6">
            {t('auth.noAccount')}{' '}
            <Link to="/register" className="text-primary-600 hover:text-primary-500 font-semibold">
              {t('auth.signUp')}
            </Link>
          </p>
        </div>
      </motion.div>
    </div>
  );
}
