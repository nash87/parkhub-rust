import { useState, useEffect } from 'react';
import { ShieldCheck, SpinnerGap, Lock, X, Check, Warning } from '@phosphor-icons/react';
import { api, type TwoFactorSetup } from '../api/client';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

export function TwoFactorSetupComponent() {
  const { t } = useTranslation();
  const [enabled, setEnabled] = useState(false);
  const [loading, setLoading] = useState(true);
  const [setupData, setSetupData] = useState<TwoFactorSetup | null>(null);
  const [code, setCode] = useState('');
  const [verifying, setVerifying] = useState(false);
  const [disablePassword, setDisablePassword] = useState('');
  const [showDisable, setShowDisable] = useState(false);
  const [disabling, setDisabling] = useState(false);

  useEffect(() => {
    api.get2FAStatus().then(res => {
      if (res.success && res.data) setEnabled(res.data.enabled);
      setLoading(false);
    }).catch(() => setLoading(false));
  }, []);

  async function handleSetup() {
    setLoading(true);
    const res = await api.setup2FA();
    if (res.success && res.data) {
      setSetupData(res.data);
    } else {
      toast.error(res.error?.message || 'Failed to set up 2FA');
    }
    setLoading(false);
  }

  async function handleVerify() {
    if (code.length !== 6) {
      toast.error('Enter a 6-digit code');
      return;
    }
    setVerifying(true);
    const res = await api.verify2FA(code);
    if (res.success && res.data?.enabled) {
      setEnabled(true);
      setSetupData(null);
      setCode('');
      toast.success('Two-factor authentication enabled!');
    } else {
      toast.error(res.error?.message || 'Invalid code');
    }
    setVerifying(false);
  }

  async function handleDisable() {
    if (!disablePassword) {
      toast.error('Enter your current password');
      return;
    }
    setDisabling(true);
    const res = await api.disable2FA(disablePassword);
    if (res.success) {
      setEnabled(false);
      setShowDisable(false);
      setDisablePassword('');
      toast.success('Two-factor authentication disabled');
    } else {
      toast.error(res.error?.message || 'Failed to disable 2FA');
    }
    setDisabling(false);
  }

  if (loading) {
    return (
      <div className="flex items-center gap-2 p-4">
        <SpinnerGap className="animate-spin" size={20} />
        <span>Loading 2FA status...</span>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <ShieldCheck size={24} weight="duotone" className={enabled ? 'text-green-500' : 'text-gray-400'} />
          <div>
            <h3 className="font-semibold">Two-Factor Authentication</h3>
            <p className="text-sm text-gray-500">
              {enabled ? 'Enabled — your account is protected' : 'Add an extra layer of security'}
            </p>
          </div>
        </div>
        {enabled ? (
          <button
            onClick={() => setShowDisable(!showDisable)}
            className="px-3 py-1.5 text-sm rounded-lg bg-red-50 text-red-600 hover:bg-red-100 transition"
          >
            Disable
          </button>
        ) : (
          <button
            onClick={handleSetup}
            className="px-3 py-1.5 text-sm rounded-lg bg-primary-50 text-primary-600 hover:bg-primary-100 transition"
          >
            Enable
          </button>
        )}
      </div>

      {/* Setup flow */}
      {setupData && !enabled && (
        <div className="border rounded-xl p-4 space-y-4 bg-gray-50 dark:bg-gray-800">
          <p className="text-sm">Scan this QR code with your authenticator app (Google Authenticator, Authy, etc.):</p>
          <div className="flex justify-center">
            <img
              src={`data:image/png;base64,${setupData.qr_code_base64}`}
              alt="2FA QR Code"
              className="w-48 h-48 rounded-lg"
            />
          </div>
          <div className="text-xs bg-gray-100 dark:bg-gray-700 p-2 rounded font-mono break-all">
            <span className="text-gray-500">Manual entry: </span>{setupData.secret}
          </div>
          <div className="flex gap-2">
            <input
              type="text"
              value={code}
              onChange={e => setCode(e.target.value.replace(/\D/g, '').slice(0, 6))}
              placeholder="Enter 6-digit code"
              className="flex-1 px-3 py-2 border rounded-lg text-center text-lg tracking-widest font-mono"
              maxLength={6}
            />
            <button
              onClick={handleVerify}
              disabled={verifying || code.length !== 6}
              className="px-4 py-2 bg-primary-600 text-white rounded-lg hover:bg-primary-700 disabled:opacity-50 flex items-center gap-1"
            >
              {verifying ? <SpinnerGap className="animate-spin" size={16} /> : <Check size={16} />}
              Verify
            </button>
          </div>
        </div>
      )}

      {/* Disable flow */}
      {showDisable && enabled && (
        <div className="border border-red-200 rounded-xl p-4 space-y-3 bg-red-50 dark:bg-red-900/20">
          <div className="flex items-center gap-2 text-red-600">
            <Warning size={20} />
            <span className="font-medium">Disable 2FA</span>
          </div>
          <p className="text-sm text-red-600">Enter your password to confirm:</p>
          <div className="flex gap-2">
            <input
              type="password"
              value={disablePassword}
              onChange={e => setDisablePassword(e.target.value)}
              placeholder="Current password"
              className="flex-1 px-3 py-2 border rounded-lg"
            />
            <button
              onClick={handleDisable}
              disabled={disabling || !disablePassword}
              className="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 disabled:opacity-50 flex items-center gap-1"
            >
              {disabling ? <SpinnerGap className="animate-spin" size={16} /> : <Lock size={16} />}
              Confirm
            </button>
            <button
              onClick={() => { setShowDisable(false); setDisablePassword(''); }}
              className="px-3 py-2 text-gray-500 hover:text-gray-700"
            >
              <X size={16} />
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
