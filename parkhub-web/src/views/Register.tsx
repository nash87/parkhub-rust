import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { CarSimple, SpinnerGap, ArrowLeft } from '@phosphor-icons/react';
import { api } from '../api/client';

export function RegisterPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [form, setForm] = useState({ username: '', email: '', name: '', password: '', password_confirmation: '' });
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
      setError(res.error?.message || 'Registration failed');
    }
    setLoading(false);
  }

  return (
    <div className="min-h-dvh mesh-gradient flex items-center justify-center px-4 py-12">
      <motion.div initial={{ opacity: 0, y: 30 }} animate={{ opacity: 1, y: 0 }} className="w-full max-w-md">
        <Link to="/login" className="inline-flex items-center gap-2 text-sm text-surface-500 hover:text-primary-600 mb-8 transition-colors">
          <ArrowLeft weight="bold" className="w-4 h-4" /> {t('auth.signIn')}
        </Link>

        <div className="glass-card p-8 sm:p-10">
          <div className="text-center mb-8">
            <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-primary-500 to-primary-600 flex items-center justify-center mx-auto mb-4 shadow-lg shadow-primary-500/20">
              <CarSimple weight="fill" className="w-8 h-8 text-white" />
            </div>
            <h1 className="text-2xl font-bold text-surface-900 dark:text-white">{t('auth.register')}</h1>
            <p className="text-surface-500 dark:text-surface-400 mt-1">{t('auth.registerSubtitle')}</p>
          </div>

          <form onSubmit={handleSubmit} className="space-y-4">
            {(['name', 'username', 'email'] as const).map(field => (
              <div key={field}>
                <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">{t(`auth.${field}`)}</label>
                <input type={field === 'email' ? 'email' : 'text'} value={form[field]} onChange={e => setForm({ ...form, [field]: e.target.value })} className="input" required />
              </div>
            ))}
            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">{t('auth.password')}</label>
              <input type="password" value={form.password} onChange={e => setForm({ ...form, password: e.target.value })} className="input" required minLength={8} />
            </div>
            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">{t('auth.passwordConfirmation')}</label>
              <input type="password" value={form.password_confirmation} onChange={e => setForm({ ...form, password_confirmation: e.target.value })} className="input" required minLength={8} />
            </div>

            {error && <div className="bg-danger/10 border border-danger/20 rounded-xl px-4 py-3 text-sm text-danger font-medium">{error}</div>}

            <button type="submit" disabled={loading} className="btn btn-primary w-full py-3 text-base">
              {loading ? <SpinnerGap weight="bold" className="w-5 h-5 animate-spin" /> : t('auth.signUp')}
            </button>
          </form>

          <p className="text-center text-sm text-surface-500 mt-6">
            {t('auth.hasAccount')} <Link to="/login" className="text-primary-600 font-semibold">{t('auth.signIn')}</Link>
          </p>
        </div>
      </motion.div>
    </div>
  );
}
