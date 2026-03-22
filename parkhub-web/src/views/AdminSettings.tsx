import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { SpinnerGap, Check } from '@phosphor-icons/react';
import { api } from '../api/client';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

interface AppSettings {
  company_name: string;
  use_case: string;
  self_registration: string;
  license_plate_mode: string;
  max_bookings_per_day: string;
  allow_guest_bookings: string;
  auto_release_minutes: string;
  require_vehicle: string;
  waitlist_enabled: string;
  credits_enabled: string;
  credits_per_booking: string;
  [key: string]: string;
}

const defaultSettings: AppSettings = {
  company_name: 'ParkHub',
  use_case: 'company',
  self_registration: 'true',
  license_plate_mode: 'optional',
  max_bookings_per_day: '3',
  allow_guest_bookings: 'false',
  auto_release_minutes: '30',
  require_vehicle: 'false',
  waitlist_enabled: 'true',
  credits_enabled: 'false',
  credits_per_booking: '1',
};

function ToggleRow({ label, description, checked, onChange }: {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium text-surface-900 dark:text-white">{label}</p>
        {description && <p className="text-xs text-surface-500 dark:text-surface-400 mt-0.5">{description}</p>}
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 dark:focus:ring-offset-surface-900 ${
          checked ? 'bg-primary-600' : 'bg-surface-300 dark:bg-surface-600'
        }`}
      >
        <span className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform shadow-sm ${checked ? 'translate-x-6' : 'translate-x-1'}`} />
      </button>
    </div>
  );
}

export function AdminSettingsPage() {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => { loadSettings(); }, []);

  async function loadSettings() {
    try {
      const res = await api.adminGetSettings();
      if (res.success && res.data) {
        setSettings(prev => ({ ...prev, ...res.data }));
      }
    } catch {
      toast.error(t('admin.settingsLoadFailed'));
    } finally {
      setLoading(false);
    }
  }

  async function handleSave() {
    setSaving(true);
    try {
      const res = await api.adminUpdateSettings(settings);
      if (res.success) {
        toast.success(t('admin.settingsSaved'));
      } else {
        toast.error(res.error?.message || t('admin.settingsSaveFailed'));
      }
    } catch {
      toast.error(t('admin.settingsSaveFailed'));
    } finally {
      setSaving(false);
    }
  }

  function update(key: string, value: string) {
    setSettings(prev => ({ ...prev, [key]: value }));
  }

  function toBool(v: string): boolean { return v === 'true' || v === '1'; }
  function fromBool(v: boolean): string { return v ? 'true' : 'false'; }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64" role="status" aria-label={t('common.loading')}>
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" aria-hidden="true" />
      </div>
    );
  }

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-8">
      <h2 className="text-xl font-semibold text-surface-900 dark:text-white">{t('admin.systemSettings')}</h2>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-8">
        {/* Left column */}
        <div className="space-y-6">
          {/* General */}
          <div className="card p-6 space-y-4">
            <h3 className="text-sm font-semibold text-surface-900 dark:text-white uppercase tracking-wide">{t('admin.general')}</h3>

            <div>
              <label htmlFor="setting-company-name" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">
                {t('admin.companyName')}
              </label>
              <input
                id="setting-company-name"
                type="text"
                value={settings.company_name}
                onChange={e => update('company_name', e.target.value)}
                className="input"
                placeholder={t('admin.companyName')}
              />
            </div>

            <div>
              <label htmlFor="setting-use-case" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">
                {t('admin.useCaseLabel')}
              </label>
              <select id="setting-use-case" value={settings.use_case} onChange={e => update('use_case', e.target.value)} className="input">
                <option value="company">{t('admin.useCaseCompany')}</option>
                <option value="residential">{t('admin.useCaseResidential')}</option>
                <option value="shared">{t('admin.useCaseShared')}</option>
                <option value="rental">{t('admin.useCaseRental')}</option>
                <option value="personal">{t('admin.useCasePersonal')}</option>
              </select>
            </div>

            <ToggleRow
              label={t('admin.selfRegistration')}
              description={t('admin.selfRegistrationDesc')}
              checked={toBool(settings.self_registration)}
              onChange={v => update('self_registration', fromBool(v))}
            />
          </div>

          {/* Booking Rules */}
          <div className="card p-6 space-y-4">
            <h3 className="text-sm font-semibold text-surface-900 dark:text-white uppercase tracking-wide">{t('admin.bookingRules')}</h3>

            <div>
              <label htmlFor="setting-max-bookings" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">
                {t('admin.maxBookingsPerDay')}
              </label>
              <input
                id="setting-max-bookings"
                type="number"
                min={0}
                max={50}
                value={settings.max_bookings_per_day}
                onChange={e => update('max_bookings_per_day', e.target.value)}
                className="input"
              />
              <p className="text-xs text-surface-500 dark:text-surface-400 mt-1">{t('admin.maxBookingsUnlimited')}</p>
            </div>

            <ToggleRow
              label={t('admin.allowGuestBookings')}
              description={t('admin.allowGuestBookingsDesc')}
              checked={toBool(settings.allow_guest_bookings)}
              onChange={v => update('allow_guest_bookings', fromBool(v))}
            />

            <ToggleRow
              label={t('admin.requireVehicle')}
              description={t('admin.requireVehicleDesc')}
              checked={toBool(settings.require_vehicle)}
              onChange={v => update('require_vehicle', fromBool(v))}
            />
          </div>

          {/* Save Button */}
          <button onClick={handleSave} disabled={saving} className="btn btn-primary w-full">
            {saving ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> : <Check weight="bold" className="w-4 h-4" />}
            {t('admin.saveSettings')}
          </button>
        </div>

        {/* Right column */}
        <div className="space-y-6">
          {/* Auto-Release */}
          <div className="card p-6 space-y-4">
            <h3 className="text-sm font-semibold text-surface-900 dark:text-white uppercase tracking-wide">{t('admin.autoRelease')}</h3>

            <div>
              <label htmlFor="setting-auto-release" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">
                {t('admin.autoReleaseMinutes')}
              </label>
              <input
                id="setting-auto-release"
                type="number"
                min={0}
                max={480}
                value={settings.auto_release_minutes}
                onChange={e => update('auto_release_minutes', e.target.value)}
                className="input"
              />
              <p className="text-xs text-surface-500 dark:text-surface-400 mt-1">
                {t('admin.autoReleaseDesc')}
              </p>
            </div>
          </div>

          {/* Waitlist */}
          <div className="card p-6 space-y-4">
            <h3 className="text-sm font-semibold text-surface-900 dark:text-white uppercase tracking-wide">{t('admin.waitlist')}</h3>

            <ToggleRow
              label={t('admin.enableWaitlist')}
              description={t('admin.enableWaitlistDesc')}
              checked={toBool(settings.waitlist_enabled)}
              onChange={v => update('waitlist_enabled', fromBool(v))}
            />
          </div>

          {/* Credits System */}
          <div className="card p-6 space-y-4">
            <h3 className="text-sm font-semibold text-surface-900 dark:text-white uppercase tracking-wide">{t('admin.creditsSystem')}</h3>

            <ToggleRow
              label={t('admin.enableCredits')}
              description={t('admin.enableCreditsDesc')}
              checked={toBool(settings.credits_enabled)}
              onChange={v => update('credits_enabled', fromBool(v))}
            />

            {toBool(settings.credits_enabled) && (
              <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }}>
                <label htmlFor="setting-credits-per-booking" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">
                  {t('admin.creditsPerBooking')}
                </label>
                <input
                  id="setting-credits-per-booking"
                  type="number"
                  min={1}
                  max={100}
                  value={settings.credits_per_booking}
                  onChange={e => update('credits_per_booking', e.target.value)}
                  className="input"
                />
              </motion.div>
            )}
          </div>

          {/* License Plate Mode */}
          <div className="card p-6 space-y-4">
            <h3 className="text-sm font-semibold text-surface-900 dark:text-white uppercase tracking-wide">{t('admin.licensePlate')}</h3>

            <div>
              <label htmlFor="setting-license-plate" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">
                {t('admin.licensePlateMode')}
              </label>
              <select id="setting-license-plate" value={settings.license_plate_mode} onChange={e => update('license_plate_mode', e.target.value)} className="input">
                <option value="required">{t('admin.licensePlateLabelRequired')}</option>
                <option value="optional">{t('admin.licensePlateLabelOptional')}</option>
                <option value="disabled">{t('admin.licensePlateLabelDisabled')}</option>
              </select>
              <p className="text-xs text-surface-500 dark:text-surface-400 mt-1">
                {settings.license_plate_mode === 'required' && t('admin.licensePlateModeRequired')}
                {settings.license_plate_mode === 'optional' && t('admin.licensePlateModeOptional')}
                {settings.license_plate_mode === 'disabled' && t('admin.licensePlateModeDisabled')}
              </p>
            </div>
          </div>
        </div>
      </div>
    </motion.div>
  );
}
