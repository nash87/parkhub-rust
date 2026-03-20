import { useState, useMemo } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { CarSimple, SpinnerGap, ArrowLeft, Check, X } from '@phosphor-icons/react';
import { api } from '../api/client';

interface PasswordRules {
  minLength: boolean;
  hasLower: boolean;
  hasUpper: boolean;
  hasDigit: boolean;
}

function validatePassword(pw: string): PasswordRules {
  return {
    minLength: pw.length >= 8,
    hasLower: /[a-z]/.test(pw),
    hasUpper: /[A-Z]/.test(pw),
    hasDigit: /[0-9]/.test(pw),
  };
}

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
  const [form, setForm] = useState({ name: '', email: '', password: '', password_confirmation: '' });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const rules = useMemo(() => validatePassword(form.password), [form.password]);
  const allRulesMet = rules.minLength && rules.hasLower && rules.hasUpper && rules.hasDigit;
  const confirmationMismatch = form.password_confirmation.length > 0 && form.password !== form.password_confirmation;
  const canSubmit = allRulesMet && form.password === form.password_confirmation && form.name.length > 0 && form.email.length > 0;

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!canSubmit) return;
    setLoading(true);
    setError('');
    const res = await api.register(form);
    if (res.success) {
      navigate('/login');
    } else {
      setError(res.error?.message || t('auth.registrationFailed'));
    }
    setLoading(false);
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

        <form onSubmit={handleSubmit} className="space-y-5">
          <div>
            <label htmlFor="reg-name" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">{t('auth.name')}</label>
            <input id="reg-name" type="text" value={form.name} onChange={e => setForm({ ...form, name: e.target.value })} className="input" required autoComplete="name" />
          </div>
          <div>
            <label htmlFor="reg-email" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">{t('auth.email')}</label>
            <input id="reg-email" type="email" value={form.email} onChange={e => setForm({ ...form, email: e.target.value })} className="input" required autoComplete="email" />
          </div>
          <div>
            <label htmlFor="reg-password" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">{t('auth.password')}</label>
            <input id="reg-password" type="password" value={form.password} onChange={e => setForm({ ...form, password: e.target.value })} className="input" required minLength={8} autoComplete="new-password" />
            <div className="flex flex-wrap gap-x-3 gap-y-1 mt-1.5">
              <PasswordRule met={rules.minLength} label={t('auth.rule8Chars')} />
              <PasswordRule met={rules.hasLower} label={t('auth.ruleLower')} />
              <PasswordRule met={rules.hasUpper} label={t('auth.ruleUpper')} />
              <PasswordRule met={rules.hasDigit} label={t('auth.ruleDigit')} />
            </div>
          </div>
          <div>
            <label htmlFor="reg-password-confirm" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">{t('auth.confirmPassword')}</label>
            <input
              id="reg-password-confirm"
              type="password"
              value={form.password_confirmation}
              onChange={e => setForm({ ...form, password_confirmation: e.target.value })}
              className={`input ${confirmationMismatch ? 'border-red-500 dark:border-red-400' : ''}`}
              required
              minLength={8}
              autoComplete="new-password"
            />
            {confirmationMismatch && (
              <p className="text-xs text-red-600 dark:text-red-400 mt-1">{t('auth.passwordMismatch')}</p>
            )}
          </div>

          {error && (
            <motion.p initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="text-sm text-red-600 dark:text-red-400" role="alert">
              {error}
            </motion.p>
          )}

          <button type="submit" disabled={loading || !canSubmit} className="btn btn-primary w-full py-2.5 disabled:opacity-50">
            {loading ? <><SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> {t('auth.creatingAccount')}</> : t('auth.signUp')}
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
