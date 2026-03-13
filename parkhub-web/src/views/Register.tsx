import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { CarSimple, SpinnerGap, ArrowLeft } from '@phosphor-icons/react';
import { api } from '../api/client';
import { useBgClass } from '../components/GenerativeBg';

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

  const bgClass = useBgClass('mesh');

  return (
    <div className={`min-h-dvh ${bgClass || 'mesh-gradient'} flex items-center justify-center px-4 py-12`}>
      <motion.div initial={{ opacity: 0, y: 24 }} animate={{ opacity: 1, y: 0 }} className="w-full max-w-sm">
        <Link to="/login" className="inline-flex items-center gap-2 text-xs text-surface-500 hover:text-accent-600 mb-8 transition-colors cursor-pointer uppercase tracking-wider font-semibold">
          <ArrowLeft weight="bold" className="w-3.5 h-3.5" /> {t('auth.signIn')}
        </Link>

        <div className="glass-card p-7 sm:p-8">
          <div className="text-center mb-7">
            <div className="w-12 h-12 bg-primary-900 dark:bg-surface-800 flex items-center justify-center mx-auto mb-4 border border-primary-800 dark:border-surface-700">
              <CarSimple weight="fill" className="w-6 h-6 text-accent-500" />
            </div>
            <h1 className="text-xl font-bold text-surface-900 dark:text-white tracking-tight">{t('auth.register')}</h1>
            <p className="text-surface-500 dark:text-surface-400 mt-1 text-sm">{t('auth.registerSubtitle')}</p>
          </div>

          <form onSubmit={handleSubmit} className="space-y-3.5">
            {(['name', 'username', 'email'] as const).map(field => (
              <div key={field}>
                <label htmlFor={`register-${field}`} className="block text-xs font-semibold text-surface-600 dark:text-surface-400 mb-1.5 uppercase tracking-wider">{t(`auth.${field}`)}</label>
                <input id={`register-${field}`} type={field === 'email' ? 'email' : 'text'} autoComplete={field === 'email' ? 'email' : field === 'username' ? 'username' : 'name'} value={form[field]} onChange={e => setForm({ ...form, [field]: e.target.value })} className="input" required />
              </div>
            ))}
            <div>
              <label htmlFor="register-password" className="block text-xs font-semibold text-surface-600 dark:text-surface-400 mb-1.5 uppercase tracking-wider">{t('auth.password')}</label>
              <input id="register-password" type="password" autoComplete="new-password" value={form.password} onChange={e => setForm({ ...form, password: e.target.value })} className="input" required minLength={8} />
            </div>
            <div>
              <label htmlFor="register-password-confirm" className="block text-xs font-semibold text-surface-600 dark:text-surface-400 mb-1.5 uppercase tracking-wider">{t('auth.passwordConfirmation')}</label>
              <input id="register-password-confirm" type="password" autoComplete="new-password" value={form.password_confirmation} onChange={e => setForm({ ...form, password_confirmation: e.target.value })} className="input" required minLength={8} />
            </div>

            {error && <div role="alert" className="bg-danger/10 border border-danger/20 rounded-md px-3 py-2.5 text-xs text-danger font-medium">{error}</div>}

            <button type="submit" disabled={loading} className="btn btn-primary w-full py-2.5 text-sm cursor-pointer">
              {loading ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> : t('auth.signUp')}
            </button>
          </form>

          <p className="text-center text-xs text-surface-500 mt-5">
            {t('auth.hasAccount')} <Link to="/login" className="text-accent-600 font-semibold cursor-pointer">{t('auth.signIn')}</Link>
          </p>
        </div>
      </motion.div>
    </div>
  );
}
