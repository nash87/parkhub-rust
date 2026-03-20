import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { CarSimple, SpinnerGap, ArrowLeft } from '@phosphor-icons/react';
import { api } from '../api/client';

export function RegisterPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [form, setForm] = useState({ username: '', email: '', name: '', password: '' });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
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
            <p className="text-xs text-surface-400 mt-1">{t('auth.minChars')}</p>
          </div>

          {error && (
            <motion.p initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="text-sm text-red-600 dark:text-red-400" role="alert">
              {error}
            </motion.p>
          )}

          <button type="submit" disabled={loading} className="btn btn-primary w-full py-2.5 disabled:opacity-50">
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
