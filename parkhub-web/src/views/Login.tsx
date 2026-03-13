import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { Car, Eye, EyeSlash, SpinnerGap, ArrowLeft, Info } from '@phosphor-icons/react';
import { useAuth } from '../context/AuthContext';
import { useBgClass } from '../components/GenerativeBg';

export function LoginPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { login, user } = useAuth();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

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

  const bgClass = useBgClass('dots');

  return (
    <div className={`min-h-dvh ${bgClass || 'parking-grid'} flex items-center justify-center px-4 py-12`}>
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4, ease: [0.22, 1, 0.36, 1] }}
        className="w-full max-w-sm"
      >
        <Link
          to="/welcome"
          className="inline-flex items-center gap-2 text-xs text-surface-500 hover:text-accent-600 mb-8 transition-colors cursor-pointer uppercase tracking-wider font-semibold"
        >
          <ArrowLeft weight="bold" className="w-3.5 h-3.5" />
          {t('welcome.greeting')}
        </Link>

        <div className="card p-7 sm:p-8">
          {/* Header */}
          <div className="text-center mb-7">
            <div className="w-12 h-12 bg-primary-900 dark:bg-surface-800 flex items-center justify-center mx-auto mb-4 border border-primary-800 dark:border-surface-700">
              <Car weight="fill" className="w-6 h-6 text-accent-500" />
            </div>
            <h1 className="text-xl font-bold text-surface-900 dark:text-white tracking-tight">
              {t('auth.login')}
            </h1>
            <p className="text-surface-500 dark:text-surface-400 mt-1 text-sm">
              {t('auth.loginSubtitle')}
            </p>
          </div>

          {/* Demo hint */}
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            transition={{ delay: 0.2 }}
            className="bg-accent-50/80 dark:bg-accent-900/10 border border-accent-200 dark:border-accent-800/30 rounded-md p-3 mb-5"
          >
            <div className="flex items-start gap-2">
              <Info weight="fill" className="w-3.5 h-3.5 text-accent-600 dark:text-accent-400 mt-0.5 flex-shrink-0" />
              <p className="text-xs text-accent-800 dark:text-accent-300 font-mono leading-relaxed">
                {t('auth.demoHint')}
              </p>
            </div>
          </motion.div>

          {/* Form */}
          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label htmlFor="username" className="block text-xs font-semibold text-surface-600 dark:text-surface-400 mb-1.5 uppercase tracking-wider">
                {t('auth.email')}
              </label>
              <input
                id="username"
                type="text"
                value={username}
                onChange={e => setUsername(e.target.value)}
                className="input"
                placeholder="admin@parkhub.test"
                autoComplete="username"
                required
                autoFocus
              />
            </div>

            <div>
              <div className="flex items-center justify-between mb-1.5">
                <label htmlFor="password" className="block text-xs font-semibold text-surface-600 dark:text-surface-400 uppercase tracking-wider">
                  {t('auth.password')}
                </label>
                <Link to="/forgot-password" className="text-xs text-accent-600 hover:text-accent-500 font-medium cursor-pointer">
                  {t('auth.forgotPassword')}
                </Link>
              </div>
              <div className="relative">
                <input
                  id="password"
                  type={showPassword ? 'text' : 'password'}
                  value={password}
                  onChange={e => setPassword(e.target.value)}
                  className="input pr-10"
                  placeholder="ParkHub2026!"
                  autoComplete="current-password"
                  required
                />
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-surface-400 hover:text-surface-600 cursor-pointer"
                  aria-label={showPassword ? 'Hide password' : 'Show password'}
                >
                  {showPassword ? <EyeSlash weight="regular" className="w-4 h-4" /> : <Eye weight="regular" className="w-4 h-4" />}
                </button>
              </div>
            </div>

            {error && (
              <motion.div
                initial={{ opacity: 0, y: -6 }}
                animate={{ opacity: 1, y: 0 }}
                className="bg-danger/10 border border-danger/20 rounded-md px-3 py-2.5 text-xs text-danger font-medium"
                role="alert"
              >
                {error}
              </motion.div>
            )}

            <button
              type="submit"
              disabled={loading || !username || !password}
              className="btn btn-primary w-full py-2.5 text-sm disabled:opacity-50 disabled:cursor-not-allowed cursor-pointer"
            >
              {loading ? (
                <><SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> {t('auth.loggingIn')}</>
              ) : (
                t('auth.signIn')
              )}
            </button>
          </form>

          <p className="text-center text-xs text-surface-500 dark:text-surface-400 mt-5">
            {t('auth.noAccount')}{' '}
            <Link to="/register" className="text-accent-600 hover:text-accent-500 font-semibold cursor-pointer">
              {t('auth.signUp')}
            </Link>
          </p>
        </div>
      </motion.div>
    </div>
  );
}
