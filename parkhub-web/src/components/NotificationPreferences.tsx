import { useState, useEffect } from 'react';
import { Bell, SpinnerGap, FloppyDisk, EnvelopeSimple, DeviceMobile, ChatCircleDots, Phone } from '@phosphor-icons/react';
import { api, type NotificationPreferences } from '../api/client';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <label className="flex items-center justify-between cursor-pointer group">
      <span className="text-sm">{label}</span>
      <button
        role="switch"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={`relative inline-flex h-6 w-11 shrink-0 rounded-full transition-colors ${checked ? 'bg-primary-600' : 'bg-gray-300 dark:bg-gray-600'}`}
      >
        <span
          className={`inline-block h-5 w-5 rounded-full bg-white shadow transition-transform mt-0.5 ${checked ? 'translate-x-5 ml-0.5' : 'translate-x-0.5'}`}
        />
      </button>
    </label>
  );
}

export function NotificationPreferencesComponent() {
  const { t } = useTranslation();
  const [prefs, setPrefs] = useState<NotificationPreferences>({
    email_booking_confirm: true,
    email_booking_reminder: true,
    email_swap_request: true,
    push_enabled: true,
    sms_booking_confirm: false,
    sms_booking_reminder: false,
    sms_booking_cancelled: false,
    whatsapp_booking_confirm: false,
    whatsapp_booking_reminder: false,
    whatsapp_booking_cancelled: false,
    phone_number: undefined,
  });
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    api.getNotificationPreferences().then(res => {
      if (res.success && res.data) setPrefs(res.data);
      setLoading(false);
    }).catch(() => setLoading(false));
  }, []);

  function update(key: keyof NotificationPreferences, value: boolean | string) {
    setPrefs(prev => ({ ...prev, [key]: value }));
    setDirty(true);
  }

  async function handleSave() {
    setSaving(true);
    const res = await api.updateNotificationPreferences(prefs);
    if (res.success) {
      toast.success('Notification preferences saved');
      setDirty(false);
    } else {
      toast.error(res.error?.message || 'Failed to save');
    }
    setSaving(false);
  }

  if (loading) {
    return (
      <div className="flex items-center gap-2 p-4">
        <SpinnerGap className="animate-spin" size={20} />
        <span>Loading preferences...</span>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2 mb-3">
        <Bell size={24} weight="duotone" className="text-primary-500" />
        <h3 className="font-semibold">Notification Preferences</h3>
      </div>

      <div className="space-y-1">
        <div className="flex items-center gap-2 text-sm font-medium text-gray-500 mb-2">
          <EnvelopeSimple size={16} />
          <span>Email Notifications</span>
        </div>
        <div className="space-y-3 pl-6">
          <Toggle
            checked={prefs.email_booking_confirm}
            onChange={v => update('email_booking_confirm', v)}
            label="Booking confirmations"
          />
          <Toggle
            checked={prefs.email_booking_reminder}
            onChange={v => update('email_booking_reminder', v)}
            label="Booking reminders"
          />
          <Toggle
            checked={prefs.email_swap_request}
            onChange={v => update('email_swap_request', v)}
            label="Swap request notifications"
          />
        </div>
      </div>

      <div className="space-y-1">
        <div className="flex items-center gap-2 text-sm font-medium text-gray-500 mb-2">
          <DeviceMobile size={16} />
          <span>{t('notifications.pushTitle', 'Push Notifications')}</span>
        </div>
        <div className="pl-6">
          <Toggle
            checked={prefs.push_enabled}
            onChange={v => update('push_enabled', v)}
            label={t('notifications.pushEnabled', 'Enable push notifications')}
          />
        </div>
      </div>

      {/* Phone number for SMS/WhatsApp */}
      <div className="space-y-2">
        <div className="flex items-center gap-2 text-sm font-medium text-gray-500 mb-1">
          <Phone size={16} />
          <span>{t('notifications.phoneNumber', 'Phone Number')}</span>
        </div>
        <div className="pl-6">
          <input
            type="tel"
            value={prefs.phone_number || ''}
            onChange={e => update('phone_number', e.target.value || '')}
            placeholder="+49 123 4567890"
            className="input text-sm w-full max-w-xs"
            aria-label={t('notifications.phoneNumber', 'Phone Number')}
          />
          <p className="text-xs text-gray-400 mt-1">{t('notifications.phoneHint', 'Required for SMS and WhatsApp notifications')}</p>
        </div>
      </div>

      {/* SMS Channel */}
      <div className="space-y-1">
        <div className="flex items-center gap-2 text-sm font-medium text-gray-500 mb-2">
          <DeviceMobile size={16} />
          <span>{t('notifications.smsTitle', 'SMS Notifications')}</span>
          <span className="text-[10px] bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-300 px-1.5 py-0.5 rounded-full font-medium">{t('notifications.stub', 'Coming soon')}</span>
        </div>
        <div className="space-y-3 pl-6">
          <Toggle
            checked={prefs.sms_booking_confirm}
            onChange={v => update('sms_booking_confirm', v)}
            label={t('notifications.bookingConfirm', 'Booking confirmations')}
          />
          <Toggle
            checked={prefs.sms_booking_reminder}
            onChange={v => update('sms_booking_reminder', v)}
            label={t('notifications.bookingReminder', 'Booking reminders')}
          />
          <Toggle
            checked={prefs.sms_booking_cancelled}
            onChange={v => update('sms_booking_cancelled', v)}
            label={t('notifications.bookingCancelled', 'Booking cancellations')}
          />
        </div>
      </div>

      {/* WhatsApp Channel */}
      <div className="space-y-1">
        <div className="flex items-center gap-2 text-sm font-medium text-gray-500 mb-2">
          <ChatCircleDots size={16} />
          <span>{t('notifications.whatsappTitle', 'WhatsApp Notifications')}</span>
          <span className="text-[10px] bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-300 px-1.5 py-0.5 rounded-full font-medium">{t('notifications.stub', 'Coming soon')}</span>
        </div>
        <div className="space-y-3 pl-6">
          <Toggle
            checked={prefs.whatsapp_booking_confirm}
            onChange={v => update('whatsapp_booking_confirm', v)}
            label={t('notifications.bookingConfirm', 'Booking confirmations')}
          />
          <Toggle
            checked={prefs.whatsapp_booking_reminder}
            onChange={v => update('whatsapp_booking_reminder', v)}
            label={t('notifications.bookingReminder', 'Booking reminders')}
          />
          <Toggle
            checked={prefs.whatsapp_booking_cancelled}
            onChange={v => update('whatsapp_booking_cancelled', v)}
            label={t('notifications.bookingCancelled', 'Booking cancellations')}
          />
        </div>
      </div>

      {dirty && (
        <button
          onClick={handleSave}
          disabled={saving}
          className="flex items-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50 transition"
        >
          {saving ? <SpinnerGap className="animate-spin" size={16} /> : <FloppyDisk size={16} />}
          Save Preferences
        </button>
      )}
    </div>
  );
}
