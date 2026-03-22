import { useState, useEffect, useRef } from 'react';
import { motion } from 'framer-motion';
import {
  UserCircle, Envelope, PencilSimple, FloppyDisk, SpinnerGap, Lock,
  CalendarCheck, House, ChartBar, DownloadSimple, Trash, CaretDown, CaretUp,
  Shield,
} from '@phosphor-icons/react';
import { useAuth } from '../context/AuthContext';
import { api, type UserStats } from '../api/client';
import { useTranslation } from 'react-i18next';
import { staggerSlow, fadeUp } from '../constants/animations';
import toast from 'react-hot-toast';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';
import { TwoFactorSetupComponent } from '../components/TwoFactorSetup';
import { NotificationPreferencesComponent } from '../components/NotificationPreferences';
import { LoginHistoryComponent } from '../components/LoginHistory';
import { ProfileThemeSection } from '../components/ProfileThemeSection';

export function ProfilePage() {
  const { t } = useTranslation();
  const { user, logout } = useAuth();
  const [editing, setEditing] = useState(false);
  const [formData, setFormData] = useState({ name: user?.name || '', email: user?.email || '' });
  const [saving, setSaving] = useState(false);
  const [stats, setStats] = useState<UserStats | null>(null);
  const [exporting, setExporting] = useState(false);
  const [confirmState, setConfirmState] = useState<{open: boolean, action: () => void}>({open: false, action: () => {}});

  // Password change
  const [pwOpen, setPwOpen] = useState(false);
  const [pwForm, setPwForm] = useState({ current: '', newPw: '', confirm: '' });
  const [pwSaving, setPwSaving] = useState(false);

  useEffect(() => {
    api.getUserStats().then(res => { if (res.success && res.data) setStats(res.data); }).catch(() => {});
  }, []);

  async function handleSave() {
    setSaving(true);
    try {
      const res = await api.updateMe({ name: formData.name, email: formData.email });
      if (res.success) {
        setEditing(false);
        toast.success(t('profile.updated', 'Profil aktualisiert'));
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } finally { setSaving(false); }
  }

  async function handleChangePassword() {
    if (pwForm.newPw.length < 8) { toast.error(t('profile.passwordTooShort', 'Mind. 8 Zeichen')); return; }
    if (pwForm.newPw !== pwForm.confirm) { toast.error(t('profile.passwordsMismatch', 'Passw\u00f6rter stimmen nicht \u00fcberein')); return; }
    if (!pwForm.current) { toast.error(t('profile.currentPasswordRequired', 'Aktuelles Passwort eingeben')); return; }
    setPwSaving(true);
    try {
      const res = await api.changePassword(pwForm.current, pwForm.newPw, pwForm.confirm);
      if (res.success) {
        toast.success(t('profile.passwordChanged', 'Passwort ge\u00e4ndert'));
        setPwForm({ current: '', newPw: '', confirm: '' });
        setPwOpen(false);
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } finally { setPwSaving(false); }
  }

  async function handleExportData() {
    setExporting(true);
    try {
      const blob = await api.exportMyData();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url; a.download = 'my-parkhub-data.json'; a.click();
      URL.revokeObjectURL(url);
      toast.success(t('gdpr.exported'));
    } catch { toast.error(t('gdpr.exportFailed')); }
    finally { setExporting(false); }
  }

  function handleDeleteAccount() {
    setConfirmState({
      open: true,
      action: async () => {
        setConfirmState({open: false, action: () => {}});
        try {
          const res = await api.deleteMyAccount();
          if (res.success) {
            toast.success(t('gdpr.deleted'));
            logout();
          } else {
            toast.error(res.error?.message || t('gdpr.deleteFailed'));
          }
        } catch { toast.error(t('gdpr.deleteFailed')); }
      },
    });
  }

  function AnimatedNumber({ value, suffix = '' }: { value: number; suffix?: string }) {
    const [display, setDisplay] = useState(0);
    const rafRef = useRef<number>(0);
    useEffect(() => {
      if (value === 0) { setDisplay(0); return; }
      const duration = 600;
      const start = performance.now();
      function tick(now: number) {
        const elapsed = now - start;
        const progress = Math.min(elapsed / duration, 1);
        const eased = 1 - Math.pow(1 - progress, 3);
        setDisplay(Math.round(eased * value));
        if (progress < 1) rafRef.current = requestAnimationFrame(tick);
      }
      rafRef.current = requestAnimationFrame(tick);
      return () => cancelAnimationFrame(rafRef.current);
    }, [value]);
    return <>{display}{suffix}</>;
  }

  const initials = user?.name?.split(' ').map(n => n[0]).join('').toUpperCase() || '?';
  const roleLabels: Record<string, string> = { user: t('profile.roles.user'), admin: t('profile.roles.admin'), superadmin: t('profile.roles.superadmin') };

  const container = staggerSlow;
  const item = fadeUp;

  return (
    <motion.div variants={container} initial="hidden" animate="show" className="max-w-3xl mx-auto space-y-6">
      <motion.div variants={item}>
        <h1 className="text-2xl font-bold text-surface-900 dark:text-white">{t('profile.title', 'Profil')}</h1>
        <p className="text-sm text-surface-500 dark:text-surface-400 mt-1">{t('profile.subtitle', 'Pers\u00f6nliche Daten verwalten')}</p>
      </motion.div>

      {/* Profile card */}
      <motion.div variants={item} className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-6">
        <div className="flex items-start gap-5">
          <div className="w-14 h-14 rounded-lg bg-surface-100 dark:bg-surface-800 flex items-center justify-center flex-shrink-0">
            <span className="text-xl font-bold text-surface-600 dark:text-surface-300">{initials}</span>
          </div>
          <div className="flex-1">
            {editing ? (
              <div className="space-y-3">
                <div>
                  <label htmlFor="profile-name" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('profile.name', 'Name')}</label>
                  <input id="profile-name" type="text" value={formData.name} onChange={e => setFormData({ ...formData, name: e.target.value })} className="input" />
                </div>
                <div>
                  <label htmlFor="profile-email" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('profile.email', 'E-Mail')}</label>
                  <input id="profile-email" type="email" value={formData.email} onChange={e => setFormData({ ...formData, email: e.target.value })} className="input" />
                </div>
                <div className="flex gap-2">
                  <button onClick={handleSave} disabled={saving} className="btn btn-primary btn-sm">
                    {saving ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> : <FloppyDisk weight="bold" className="w-4 h-4" />}
                    {t('common.save', 'Speichern')}
                  </button>
                  <button onClick={() => setEditing(false)} className="btn btn-secondary btn-sm">{t('common.cancel', 'Abbrechen')}</button>
                </div>
              </div>
            ) : (
              <>
                <div className="flex items-center gap-3">
                  <h2 className="text-xl font-bold text-surface-900 dark:text-white">{user?.name}</h2>
                  <span className="text-xs font-medium text-surface-500 dark:text-surface-400 bg-surface-100 dark:bg-surface-800 px-2 py-0.5 rounded-md">
                    {roleLabels[user?.role || 'user']}
                  </span>
                </div>
                <div className="flex flex-wrap items-center gap-4 mt-2 text-sm text-surface-500 dark:text-surface-400">
                  <span className="flex items-center gap-1.5"><UserCircle weight="regular" className="w-4 h-4" />@{user?.username}</span>
                  <span className="flex items-center gap-1.5"><Envelope weight="regular" className="w-4 h-4" />{user?.email}</span>
                </div>
                <div className="mt-3">
                  <button onClick={() => setEditing(true)} className="btn btn-secondary btn-sm">
                    <PencilSimple weight="bold" className="w-3.5 h-3.5" /> {t('common.edit', 'Bearbeiten')}
                  </button>
                </div>
              </>
            )}
          </div>
        </div>
      </motion.div>

      {/* Stats */}
      <motion.div variants={item} className="grid grid-cols-1 sm:grid-cols-3 gap-4" aria-live="polite">
        <StatCard
          label={t('profile.bookingsThisMonth', 'Buchungen (Monat)')}
          value={stats ? <AnimatedNumber value={stats.bookings_this_month} /> : '-'}
          color="text-primary-600 dark:text-primary-400"
        />
        <StatCard
          label={t('profile.homeOfficeDays', 'Homeoffice-Tage')}
          value={stats ? <AnimatedNumber value={stats.homeoffice_days_this_month} /> : '-'}
          color="text-surface-900 dark:text-white"
        />
        <StatCard
          label={t('profile.avgDuration', 'Durchschn. Dauer')}
          value={stats ? <AnimatedNumber value={stats.avg_duration_minutes} suffix=" min" /> : '-'}
          color="text-surface-900 dark:text-white"
        />
      </motion.div>

      {/* Password change */}
      <motion.div variants={item} className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-5">
        <button onClick={() => setPwOpen(!pwOpen)} className="w-full flex items-center justify-between" aria-expanded={pwOpen}>
          <h3 className="text-base font-semibold text-surface-900 dark:text-white">
            {t('profile.changePassword', 'Passwort \u00e4ndern')}
          </h3>
          {pwOpen ? <CaretUp weight="bold" className="w-4 h-4 text-surface-400" /> : <CaretDown weight="bold" className="w-4 h-4 text-surface-400" />}
        </button>
        {pwOpen && (
          <div className="mt-4 space-y-3">
            <div>
              <label htmlFor="pw-current" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('profile.currentPassword', 'Aktuelles Passwort')}</label>
              <input id="pw-current" type="password" value={pwForm.current} onChange={e => setPwForm({ ...pwForm, current: e.target.value })} className="input" autoComplete="current-password" />
            </div>
            <div>
              <label htmlFor="pw-new" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('profile.newPassword', 'Neues Passwort')}</label>
              <input id="pw-new" type="password" value={pwForm.newPw} onChange={e => setPwForm({ ...pwForm, newPw: e.target.value })} className="input" autoComplete="new-password" />
              {pwForm.newPw.length > 0 && pwForm.newPw.length < 8 && <p className="text-xs text-amber-600 mt-1">{t('profile.minChars')}</p>}
            </div>
            <div>
              <label htmlFor="pw-confirm" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">{t('profile.confirmPassword', 'Passwort best\u00e4tigen')}</label>
              <input id="pw-confirm" type="password" value={pwForm.confirm} onChange={e => setPwForm({ ...pwForm, confirm: e.target.value })} className="input" autoComplete="new-password" />
              {pwForm.confirm.length > 0 && pwForm.newPw !== pwForm.confirm && <p className="text-xs text-red-600 mt-1">{t('profile.passwordsNoMatch')}</p>}
            </div>
            <button onClick={handleChangePassword} disabled={pwSaving || pwForm.newPw.length < 8 || pwForm.newPw !== pwForm.confirm || !pwForm.current} className="btn btn-primary btn-sm disabled:opacity-60">
              {pwSaving ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> : <Lock weight="bold" className="w-4 h-4" />}
              {t('profile.changePasswordBtn', 'Passwort \u00e4ndern')}
            </button>
          </div>
        )}
      </motion.div>

      {/* GDPR */}
      <motion.div variants={item} className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-5 space-y-4">
        <div>
          <h3 className="text-base font-semibold text-surface-900 dark:text-white">DSGVO / GDPR</h3>
          <p className="text-xs text-surface-500 dark:text-surface-400 mt-1">{t('gdpr.rights', 'Ihre Rechte gem\u00e4\u00df DSGVO Art. 15, 17 und 20.')}</p>
        </div>
        <div className="flex flex-col sm:flex-row gap-3">
          <button onClick={handleExportData} disabled={exporting} className="btn btn-secondary flex-1">
            <DownloadSimple weight="bold" className="w-4 h-4" />
            <div className="text-left">
              <div className="font-medium">{t('gdpr.dataExport', 'Daten exportieren')}</div>
              <div className="text-xs opacity-60">{t('gdpr.dataExportDesc', 'Art. 20 Datenportabilit\u00e4t')}</div>
            </div>
          </button>
          <button onClick={handleDeleteAccount} className="btn btn-secondary flex-1 border-red-300 dark:border-red-600 hover:bg-red-50 dark:hover:bg-red-900/20">
            <Trash weight="bold" className="w-4 h-4 text-red-600" />
            <div className="text-left">
              <div className="font-medium">{t('gdpr.deleteAccount', 'Konto l\u00f6schen')}</div>
              <div className="text-xs opacity-60">{t('gdpr.deleteAccountDesc', 'Alle Daten unwiderruflich l\u00f6schen')}</div>
            </div>
          </button>
        </div>
      </motion.div>
      {/* Design Theme */}
      <motion.div variants={fadeUp} className="card p-6">
        <ProfileThemeSection />
      </motion.div>

      {/* Security: 2FA */}
      <motion.div variants={fadeUp} className="card p-6">
        <TwoFactorSetupComponent />
      </motion.div>

      {/* Notification Preferences */}
      <motion.div variants={fadeUp} className="card p-6">
        <NotificationPreferencesComponent />
      </motion.div>

      {/* Login History & Sessions */}
      <motion.div variants={fadeUp} className="card p-6">
        <LoginHistoryComponent />
      </motion.div>

      <ConfirmDialog
        open={confirmState.open}
        title={t('gdpr.deleteAccount')}
        message={t('gdpr.deleteConfirmMessage')}
        variant="danger"
        onConfirm={confirmState.action}
        onCancel={() => setConfirmState({open: false, action: () => {}})}
      />
    </motion.div>
  );
}

function StatCard({ label, value, color }: { label: string; value: React.ReactNode; color: string }) {
  return (
    <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-4">
      <p className="text-xs font-medium text-surface-500 dark:text-surface-400 mb-2">{label}</p>
      <p className={`text-2xl font-bold tabular-nums ${color}`}>{value}</p>
    </div>
  );
}
