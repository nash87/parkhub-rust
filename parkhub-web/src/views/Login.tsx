import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { CarSimple, Eye, EyeSlash, SpinnerGap, ArrowLeft, Info, ShieldCheck, Lightning, Globe } from '@phosphor-icons/react';
import { useAuth } from '../context/AuthContext';

const FEATURES = [
  { icon: Lightning, key: 'quick' },
  { icon: ShieldCheck, key: 'secure' },
  { icon: Globe, key: 'selfHosted' },
];

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

  return (
    <div className="min-h-dvh mesh-gradient flex">
      {/* Left panel — hero (hidden on mobile) */}
      <div className="hidden lg:flex lg:w-1/2 relative items-center justify-center p-12 overflow-hidden">
        {/* Decorative floating shapes */}
        <div className="absolute inset-0 overflow-hidden pointer-events-none">
          <motion.div
            animate={{ y: [0, -30, 0], rotate: [0, 8, 0] }}
            transition={{ duration: 8, repeat: Infinity, ease: 'easeInOut' }}
            className="absolute top-[20%] right-[15%] w-24 h-24 rounded-3xl bg-primary-500/10 backdrop-blur-sm border border-primary-500/10"
          />
          <motion.div
            animate={{ y: [0, 20, 0], rotate: [0, -5, 0] }}
            transition={{ duration: 10, repeat: Infinity, ease: 'easeInOut', delay: 1 }}
            className="absolute bottom-[25%] left-[10%] w-16 h-16 rounded-2xl bg-accent-500/10 backdrop-blur-sm border border-accent-500/10"
          />
          <motion.div
            animate={{ scale: [1, 1.1, 1], opacity: [0.3, 0.6, 0.3] }}
            transition={{ duration: 6, repeat: Infinity, ease: 'easeInOut', delay: 2 }}
            className="absolute top-[60%] right-[25%] w-32 h-32 rounded-full bg-primary-400/5"
          />
        </div>

        <motion.div
          initial={{ opacity: 0, x: -40 }}
          animate={{ opacity: 1, x: 0 }}
          transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1] }}
          className="relative z-10 max-w-lg"
        >
          {/* Logo */}
          <motion.div
            initial={{ scale: 0.8, opacity: 0 }}
            animate={{ scale: 1, opacity: 1 }}
            transition={{ type: 'spring', damping: 20, stiffness: 300, delay: 0.2 }}
            className="w-20 h-20 rounded-3xl bg-gradient-to-br from-primary-500 to-primary-600 flex items-center justify-center mb-8 shadow-2xl shadow-primary-500/30"
          >
            <CarSimple weight="fill" className="w-10 h-10 text-white" />
          </motion.div>

          <h1 className="text-4xl font-bold text-surface-900 dark:text-white mb-3 tracking-tight">
            ParkHub
          </h1>
          <p className="text-lg text-surface-600 dark:text-surface-300 mb-10 leading-relaxed">
            {t('welcome.subtitle', 'Smart parking management for your organization. Self-hosted, privacy-first, beautifully designed.')}
          </p>

          {/* Feature pills */}
          <div className="space-y-4">
            {FEATURES.map(({ icon: Icon, key }, i) => (
              <motion.div
                key={key}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: 0.4 + i * 0.15, duration: 0.5, ease: [0.22, 1, 0.36, 1] }}
                className="flex items-center gap-3"
              >
                <div className="w-10 h-10 rounded-xl bg-primary-500/10 dark:bg-primary-500/20 flex items-center justify-center flex-shrink-0">
                  <Icon weight="duotone" className="w-5 h-5 text-primary-600 dark:text-primary-400" />
                </div>
                <span className="text-surface-700 dark:text-surface-200 font-medium">
                  {t(`login.feature.${key}`, key === 'quick' ? 'Book in seconds with Quick Book' : key === 'secure' ? 'AES-256 encryption at rest' : 'Your data stays on your server')}
                </span>
              </motion.div>
            ))}
          </div>
        </motion.div>
      </div>

      {/* Right panel — login form */}
      <div className="flex-1 flex items-center justify-center px-4 py-12 lg:px-12">
        <motion.div
          initial={{ opacity: 0, y: 30 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, ease: [0.22, 1, 0.36, 1] }}
          className="w-full max-w-md"
        >
          {/* Back link — mobile only shows welcome, desktop shows nothing */}
          <Link
            to="/welcome"
            className="inline-flex items-center gap-2 text-sm text-surface-500 hover:text-primary-600 mb-8 transition-colors lg:hidden"
          >
            <ArrowLeft weight="bold" className="w-4 h-4" />
            {t('welcome.greeting')}
          </Link>

          <div className="glass-card p-8 sm:p-10">
            {/* Header */}
            <div className="text-center mb-8">
              <div className="lg:hidden w-16 h-16 rounded-2xl bg-gradient-to-br from-primary-500 to-primary-600 flex items-center justify-center mx-auto mb-4 shadow-lg shadow-primary-500/20">
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
                    className="absolute right-3 top-1/2 -translate-y-1/2 text-surface-400 hover:text-surface-600 transition-colors"
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

          {/* Version badge */}
          <p className="text-center text-xs text-surface-400 mt-4">
            ParkHub v1.3.0
          </p>
        </motion.div>
      </div>
    </div>
  );
}
