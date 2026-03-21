import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';
import { z } from 'zod';
import { CarSimple, SpinnerGap, ArrowLeft, Check, X } from '@phosphor-icons/react';
import { api } from '../api/client';
import { FormField, FormInput } from '../components/ui/FormField';

const registerSchema = z.object({
  name: z.string().min(1, 'Required'),
  email: z.string().email('Invalid email'),
  password: z
    .string()
    .min(8, '8+ characters')
    .regex(/[a-z]/, 'Needs lowercase')
    .regex(/[A-Z]/, 'Needs uppercase')
    .regex(/[0-9]/, 'Needs digit'),
  password_confirmation: z.string().min(1, 'Required'),
}).refine(data => data.password === data.password_confirmation, {
  message: 'Passwords do not match',
  path: ['password_confirmation'],
});

type RegisterForm = z.infer<typeof registerSchema>;

function PasswordRule({ met, label }: { met: boolean; label: string }) {
  return (
    <span className={`inline-flex items-center gap-1 text-xs ${met ? 'text-green-600 dark:text-green-400' : 'text-surface-400'}`}>
      {met ? <Check weight="bold" className="w-3 h-3" /> : <X weight="bold" className="w-3 h-3" />}
      {label}
    </span>
  );
}

export function RegisterPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [serverError, setServerError] = useState('');

  const {
    register,
    handleSubmit,
    watch,
    formState: { errors, isSubmitting },
  } = useForm<RegisterForm>({
    resolver: zodResolver(registerSchema),
    defaultValues: { name: '', email: '', password: '', password_confirmation: '' },
    mode: 'onBlur',
  });

  const pw = watch('password', '');
  const rules = {
    minLength: pw.length >= 8,
    hasLower: /[a-z]/.test(pw),
    hasUpper: /[A-Z]/.test(pw),
    hasDigit: /[0-9]/.test(pw),
  };

  async function onSubmit(data: RegisterForm) {
    setServerError('');
    const res = await api.register(data);
    if (res.success) {
      navigate('/login');
    } else {
      setServerError(res.error?.message || t('auth.registrationFailed'));
    }
  }

  return (
    <div className="min-h-dvh bg-white dark:bg-surface-950 flex items-center justify-center px-6 py-12">
      <motion.div
        initial={{ opacity: 0, y: 12 }}
        animate={{ opacity: 1, y: 0 }}
        className="w-full max-w-sm"
      >
        <Link to="/login" className="inline-flex items-center gap-2 text-sm text-surface-500 hover:text-primary-600 mb-8 transition-colors">
          <ArrowLeft weight="bold" className="w-4 h-4" /> {t('auth.signIn')}
        </Link>

        <div className="flex items-center gap-3 mb-8">
          <div className="w-9 h-9 rounded-lg bg-primary-600 flex items-center justify-center">
            <CarSimple weight="fill" className="w-5 h-5 text-white" />
          </div>
          <span className="text-lg font-bold text-surface-900 dark:text-white tracking-tight">ParkHub</span>
        </div>

        <h1 className="text-2xl font-bold text-surface-900 dark:text-white mb-1">{t('auth.register')}</h1>
        <p className="text-surface-500 dark:text-surface-400 text-sm mb-8">{t('auth.registerSubtitle')}</p>

        <form onSubmit={handleSubmit(onSubmit)} className="space-y-5" noValidate>
          <FormField label={t('auth.name')} htmlFor="reg-name" error={errors.name} required>
            <FormInput registration={register('name')} hasError={!!errors.name} id="reg-name" type="text" autoComplete="name" />
          </FormField>

          <FormField label={t('auth.email')} htmlFor="reg-email" error={errors.email} required>
            <FormInput registration={register('email')} hasError={!!errors.email} id="reg-email" type="email" autoComplete="email" />
          </FormField>

          <div>
            <FormField label={t('auth.password')} htmlFor="reg-password" error={errors.password} required>
              <FormInput
                registration={register('password')}
                hasError={!!errors.password}
                id="reg-password"
                type="password"
                autoComplete="new-password"
              />
            </FormField>
            <div className="flex flex-wrap gap-x-3 gap-y-1 mt-1.5">
              <PasswordRule met={rules.minLength} label={t('auth.rule8Chars')} />
              <PasswordRule met={rules.hasLower} label={t('auth.ruleLower')} />
              <PasswordRule met={rules.hasUpper} label={t('auth.ruleUpper')} />
              <PasswordRule met={rules.hasDigit} label={t('auth.ruleDigit')} />
            </div>
          </div>

          <FormField label={t('auth.confirmPassword')} htmlFor="reg-password-confirm" error={errors.password_confirmation} required>
            <FormInput
              registration={register('password_confirmation')}
              hasError={!!errors.password_confirmation}
              id="reg-password-confirm"
              type="password"
              autoComplete="new-password"
            />
          </FormField>

          {serverError && (
            <motion.p initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="text-sm text-red-600 dark:text-red-400" role="alert">
              {serverError}
            </motion.p>
          )}

          <button type="submit" disabled={isSubmitting} className="btn btn-primary w-full py-2.5 disabled:opacity-50">
            {isSubmitting ? <><SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> {t('auth.creatingAccount')}</> : t('auth.signUp')}
          </button>
        </form>

        <p className="text-center text-sm text-surface-500 dark:text-surface-400 mt-6">
          {t('auth.hasAccount')}{' '}
          <Link to="/login" className="text-primary-600 dark:text-primary-400 font-medium hover:underline">{t('auth.signIn')}</Link>
        </p>
      </motion.div>
    </div>
  );
}
