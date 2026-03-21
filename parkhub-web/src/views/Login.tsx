import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { CarSimple, Eye, EyeSlash, SpinnerGap, ArrowLeft } from '@phosphor-icons/react';
import { useAuth } from '../context/AuthContext';
import { FormField, FormInput } from '../components/ui/FormField';
// @ts-ignore — Vite resolves JSON imports at build time
import { version as APP_VERSION } from '../../package.json';

const loginSchema = z.object({
  username: z.string().min(1, 'Required'),
  password: z.string().min(1, 'Required'),
});

type LoginForm = z.infer<typeof loginSchema>;

export function LoginPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { login, user } = useAuth();
  const [showPassword, setShowPassword] = useState(false);
  const [serverError, setServerError] = useState<string | null>(null);

  const {
    register,
    handleSubmit,
    setValue,
    formState: { errors, isSubmitting },
  } = useForm<LoginForm>({
    resolver: zodResolver(loginSchema),
    defaultValues: { username: '', password: '' },
  });

  if (user) {
    navigate('/', { replace: true });
    return null;
  }

  async function onSubmit(data: LoginForm) {
    setServerError(null);
    const result = await login(data.username, data.password);
    if (result.success) {
      navigate('/', { replace: true });
    } else {
      setServerError(result.error || t('auth.loginError'));
    }
  }

  function autofillDemo() {
    setValue('username', 'admin@parkhub.test');
    setValue('password', 'demo');
  }

  return (
    <div className="min-h-dvh bg-white dark:bg-surface-950 flex">
      {/* Left panel — clean branding */}
      <div className="hidden lg:flex lg:w-[45%] bg-surface-950 dark:bg-surface-900 relative items-end p-12 overflow-hidden">
        <div className="absolute top-0 left-0 w-full h-1 bg-gradient-to-r from-primary-500 via-primary-400 to-emerald-400" />
        <div className="relative z-10">
          <div className="flex items-center gap-3 mb-8">
            <div className="w-10 h-10 rounded-lg bg-primary-600 flex items-center justify-center">
              <CarSimple weight="fill" className="w-6 h-6 text-white" />
            </div>
            <span className="text-xl font-bold text-white tracking-tight">ParkHub</span>
          </div>
          <h2 className="text-3xl font-bold text-white mb-4 leading-tight whitespace-pre-line">
            {t('auth.heroTitle')}
          </h2>
          <p className="text-surface-400 text-sm leading-relaxed max-w-sm">
            {t('auth.heroSubtitle')}
          </p>
        </div>
      </div>

      {/* Right panel — form */}
      <div className="flex-1 flex items-center justify-center px-6 py-12 lg:px-16">
        <motion.div
          initial={{ opacity: 0, y: 12 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.3 }}
          className="w-full max-w-sm"
        >
          {/* Mobile back link */}
          <Link
            to="/welcome"
            className="inline-flex items-center gap-2 text-sm text-surface-500 hover:text-primary-600 mb-8 transition-colors lg:hidden"
          >
            <ArrowLeft weight="bold" className="w-4 h-4" />
            {t('auth.back')}
          </Link>

          {/* Mobile logo */}
          <div className="flex items-center gap-3 mb-8 lg:hidden">
            <div className="w-9 h-9 rounded-lg bg-primary-600 flex items-center justify-center">
              <CarSimple weight="fill" className="w-5 h-5 text-white" />
            </div>
            <span className="text-lg font-bold text-surface-900 dark:text-white tracking-tight">ParkHub</span>
          </div>

          <h1 className="text-2xl font-bold text-surface-900 dark:text-white mb-1">
            {t('auth.login')}
          </h1>
          <p className="text-surface-500 dark:text-surface-400 text-sm mb-8">
            {t('auth.loginSubtitle')}
          </p>

          {/* Demo hint — click to auto-fill credentials */}
          <button
            type="button"
            id="demo-autofill"
            onClick={autofillDemo}
            className="flex items-center gap-2 px-3 py-2 rounded-lg bg-primary-50 dark:bg-primary-950/30 border border-primary-200 dark:border-primary-800 text-sm text-primary-700 dark:text-primary-300 mb-6 w-full text-left cursor-pointer hover:bg-primary-100 dark:hover:bg-primary-950/50 transition-colors"
          >
            <span className="w-1.5 h-1.5 rounded-full bg-primary-500 flex-shrink-0" />
            {t('auth.demoHint')}
          </button>

          <form onSubmit={handleSubmit(onSubmit)} className="space-y-5" noValidate>
            <FormField label={t('auth.email')} htmlFor="username" error={errors.username}>
              <FormInput
                registration={register('username')}
                hasError={!!errors.username}
                id="username"
                type="text"
                placeholder="admin@parkhub.test"
                autoComplete="username"
              />
            </FormField>

            <div>
              <div className="flex items-center justify-between mb-1.5">
                <label htmlFor="password" className="block text-sm font-medium text-surface-700 dark:text-surface-300">
                  {t('auth.password')}
                </label>
                <Link to="/forgot-password" className="text-xs text-primary-600 dark:text-primary-400 hover:underline">
                  {t('auth.forgotPassword')}
                </Link>
              </div>
              <div className="relative">
                <FormInput
                  registration={register('password')}
                  hasError={!!errors.password}
                  id="password"
                  type={showPassword ? 'text' : 'password'}
                  placeholder="demo"
                  autoComplete="current-password"
                  className="pr-10"
                />
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-surface-400 hover:text-surface-600 dark:hover:text-surface-300"
                  aria-label={showPassword ? t('auth.hidePassword') : t('auth.showPassword')}
                >
                  {showPassword ? <EyeSlash weight="bold" className="w-4 h-4" /> : <Eye weight="bold" className="w-4 h-4" />}
                </button>
              </div>
            </div>

            {serverError && (
              <motion.p
                initial={{ opacity: 0, y: -4 }}
                animate={{ opacity: 1, y: 0 }}
                className="text-sm text-red-600 dark:text-red-400"
                role="alert"
              >
                {serverError}
              </motion.p>
            )}

            <button
              id="login-submit"
              type="submit"
              disabled={isSubmitting}
              className="btn btn-primary w-full py-2.5 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSubmitting ? (
                <><SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> {t('auth.loggingIn')}</>
              ) : (
                t('auth.signIn')
              )}
            </button>
          </form>

          <p className="text-center text-sm text-surface-500 dark:text-surface-400 mt-6">
            {t('auth.noAccount')}{' '}
            <Link to="/register" className="text-primary-600 dark:text-primary-400 font-medium hover:underline">
              {t('auth.signUp')}
            </Link>
          </p>

          <p className="text-center text-xs text-surface-400 mt-8">
            ParkHub v{APP_VERSION}
          </p>
        </motion.div>
      </div>
    </div>
  );
}
