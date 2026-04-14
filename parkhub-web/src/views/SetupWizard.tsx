import { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';

interface WizardStep {
  step: number;
  name: string;
  completed: boolean;
}

const THEMES = [
  { id: 'classic', label: 'Classic', color: 'bg-blue-600' },
  { id: 'glass', label: 'Glass', color: 'bg-sky-400/60' },
  { id: 'bento', label: 'Bento', color: 'bg-amber-500' },
  { id: 'brutalist', label: 'Brutalist', color: 'bg-gray-800' },
  { id: 'neon', label: 'Neon', color: 'bg-fuchsia-500' },
  { id: 'warm', label: 'Warm', color: 'bg-orange-400' },
  { id: 'liquid', label: 'Liquid', color: 'bg-cyan-500' },
  { id: 'mono', label: 'Mono', color: 'bg-neutral-600' },
  { id: 'ocean', label: 'Ocean', color: 'bg-teal-500' },
  { id: 'forest', label: 'Forest', color: 'bg-emerald-600' },
  { id: 'synthwave', label: 'Synthwave', color: 'bg-purple-600' },
  { id: 'zen', label: 'Zen', color: 'bg-stone-500' },
  { id: 'aurora', label: 'Aurora', color: 'bg-violet-500' },
  { id: 'material', label: 'Material You', color: 'bg-indigo-500' },
  { id: 'midnight', label: 'Midnight', color: 'bg-black' },
  { id: 'sakura', label: 'Sakura', color: 'bg-pink-400' },
];

const TIMEZONES = [
  'UTC', 'Europe/Berlin', 'Europe/London', 'Europe/Paris', 'Europe/Vienna',
  'Europe/Zurich', 'America/New_York', 'America/Chicago', 'America/Los_Angeles',
  'Asia/Tokyo', 'Asia/Shanghai', 'Australia/Sydney',
];

export function SetupWizardPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [currentStep, setCurrentStep] = useState(1);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Step 1 fields
  const [companyName, setCompanyName] = useState('');
  const [timezone, setTimezone] = useState('Europe/Berlin');
  const [logoBase64, setLogoBase64] = useState<string | null>(null);

  // Step 2 fields
  const [lotName, setLotName] = useState('');
  const [floorCount, setFloorCount] = useState(1);
  const [slotsPerFloor, setSlotsPerFloor] = useState(10);

  // Step 3 fields
  const [inviteEmails, setInviteEmails] = useState('');

  // Step 4 fields
  const [selectedTheme, setSelectedTheme] = useState('classic');

  // Check wizard status on mount
  useEffect(() => {
    fetch('/api/v1/setup/wizard/status')
      .then(r => r.json())
      .then(res => {
        if (res.success && res.data?.completed) {
          navigate('/', { replace: true });
        }
      })
      .catch(() => {});
  }, [navigate]);

  const handleLogoUpload = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => setLogoBase64(reader.result as string);
    reader.readAsDataURL(file);
  }, []);

  const submitStep = useCallback(async () => {
    setLoading(true);
    setError(null);

    let body: Record<string, unknown> = { step: currentStep };

    switch (currentStep) {
      case 1:
        if (!companyName.trim()) {
          setError(t('setup.companyNameRequired', 'Company name is required'));
          setLoading(false);
          return;
        }
        body = { ...body, company_name: companyName, timezone, logo_base64: logoBase64 };
        break;
      case 2:
        if (!lotName.trim()) {
          setError(t('setup.lotNameRequired', 'Lot name is required'));
          setLoading(false);
          return;
        }
        body = { ...body, lot_name: lotName, floor_count: floorCount, slots_per_floor: slotsPerFloor };
        break;
      case 3:
        body = { ...body, invite_emails: inviteEmails.split(',').map(e => e.trim()).filter(e => e.includes('@')) };
        break;
      case 4:
        body = { ...body, theme: selectedTheme };
        break;
    }

    try {
      const res = await fetch('/api/v1/setup/wizard', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      });
      const json = await res.json();

      if (json.success) {
        if (currentStep < 4) {
          setCurrentStep(currentStep + 1);
        } else {
          navigate('/', { replace: true });
        }
      } else {
        setError(json.error?.message || 'Failed to save');
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      setError(t('setup.networkError', 'Network error'));
    } finally {
      setLoading(false);
    }
  }, [currentStep, companyName, timezone, logoBase64, lotName, floorCount, slotsPerFloor, inviteEmails, selectedTheme, navigate, t]);

  const stepLabels = [
    t('setup.step1', 'Company Info'),
    t('setup.step2', 'Create Lot'),
    t('setup.step3', 'User Setup'),
    t('setup.step4', 'Choose Theme'),
  ];

  return (
    <div className="min-h-dvh bg-gray-50 dark:bg-gray-950 flex flex-col items-center justify-center p-6" data-testid="setup-wizard">
      <div className="w-full max-w-xl">
        <h1 className="text-3xl font-bold text-center mb-2 text-gray-900 dark:text-white">
          {t('setup.title', 'Setup Wizard')}
        </h1>

        {/* Progress bar */}
        <div className="flex items-center gap-2 mb-8" data-testid="wizard-progress">
          {stepLabels.map((label, i) => (
            <div key={i} className="flex-1 flex flex-col items-center">
              <div
                className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold mb-1 transition-colors ${
                  i + 1 <= currentStep
                    ? 'bg-primary-600 text-white'
                    : 'bg-gray-200 dark:bg-gray-700 text-gray-500'
                }`}
              >
                {i + 1}
              </div>
              <span className="text-xs text-gray-500 text-center">{label}</span>
              {i < 3 && <div className={`h-0.5 w-full mt-1 ${i + 1 < currentStep ? 'bg-primary-600' : 'bg-gray-200 dark:bg-gray-700'}`} />}
            </div>
          ))}
        </div>

        {/* Step content */}
        <div className="bg-white dark:bg-gray-900 rounded-2xl shadow-lg p-6 mb-4" data-testid="wizard-step-content">
          {currentStep === 1 && (
            <div className="space-y-4" data-testid="wizard-step-1">
              <h2 className="text-xl font-semibold text-gray-900 dark:text-white">{t('setup.step1', 'Company Info')}</h2>
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  {t('setup.companyName', 'Company Name')}
                </label>
                <input
                  type="text"
                  value={companyName}
                  onChange={e => setCompanyName(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                  placeholder="ParkCorp GmbH"
                  data-testid="input-company-name"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  {t('setup.timezone', 'Timezone')}
                </label>
                <select
                  value={timezone}
                  onChange={e => setTimezone(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                  data-testid="select-timezone"
                >
                  {TIMEZONES.map(tz => <option key={tz} value={tz}>{tz}</option>)}
                </select>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  Logo (optional)
                </label>
                <input
                  type="file"
                  accept="image/*"
                  onChange={handleLogoUpload}
                  className="text-sm text-gray-500"
                  data-testid="input-logo"
                />
              </div>
            </div>
          )}

          {currentStep === 2 && (
            <div className="space-y-4" data-testid="wizard-step-2">
              <h2 className="text-xl font-semibold text-gray-900 dark:text-white">{t('setup.step2', 'Create Lot')}</h2>
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  {t('setup.lotName', 'Lot Name')}
                </label>
                <input
                  type="text"
                  value={lotName}
                  onChange={e => setLotName(e.target.value)}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                  placeholder="Main Garage"
                  data-testid="input-lot-name"
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    {t('setup.floors', 'Floors')}
                  </label>
                  <input
                    type="number"
                    min={1}
                    max={20}
                    value={floorCount}
                    onChange={e => setFloorCount(Number(e.target.value))}
                    className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                    data-testid="input-floors"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                    {t('setup.slotsPerFloor', 'Slots per Floor')}
                  </label>
                  <input
                    type="number"
                    min={1}
                    max={500}
                    value={slotsPerFloor}
                    onChange={e => setSlotsPerFloor(Number(e.target.value))}
                    className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                    data-testid="input-slots-per-floor"
                  />
                </div>
              </div>
              <p className="text-sm text-gray-500">
                Total: {floorCount * slotsPerFloor} slots
              </p>
            </div>
          )}

          {currentStep === 3 && (
            <div className="space-y-4" data-testid="wizard-step-3">
              <h2 className="text-xl font-semibold text-gray-900 dark:text-white">{t('setup.step3', 'User Setup')}</h2>
              <p className="text-sm text-gray-500">{t('setup.inviteDesc', 'Your admin account is already set up. Optionally invite more users.')}</p>
              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  {t('setup.inviteUsers', 'Invite Users (comma-separated emails)')}
                </label>
                <textarea
                  value={inviteEmails}
                  onChange={e => setInviteEmails(e.target.value)}
                  rows={3}
                  className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                  placeholder="alice@company.com, bob@company.com"
                  data-testid="input-invite-emails"
                />
              </div>
            </div>
          )}

          {currentStep === 4 && (
            <div className="space-y-4" data-testid="wizard-step-4">
              <h2 className="text-xl font-semibold text-gray-900 dark:text-white">{t('setup.step4', 'Choose Theme')}</h2>
              <div className="grid grid-cols-3 sm:grid-cols-4 gap-3" data-testid="theme-grid">
                {THEMES.map(theme => (
                  <button
                    key={theme.id}
                    onClick={() => setSelectedTheme(theme.id)}
                    className={`flex flex-col items-center p-3 rounded-xl border-2 transition-all ${
                      selectedTheme === theme.id
                        ? 'border-primary-600 ring-2 ring-primary-600/30'
                        : 'border-gray-200 dark:border-gray-700 hover:border-gray-400'
                    }`}
                    data-testid={`theme-${theme.id}`}
                  >
                    <div className={`w-10 h-10 rounded-lg ${theme.color} mb-2`} />
                    <span className="text-xs font-medium text-gray-700 dark:text-gray-300">{theme.label}</span>
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Error */}
        {error && (
          <div className="text-red-500 text-sm text-center mb-4" data-testid="wizard-error">{error}</div>
        )}

        {/* Navigation */}
        <div className="flex justify-between">
          <button
            onClick={() => setCurrentStep(Math.max(1, currentStep - 1))}
            disabled={currentStep === 1}
            className="px-4 py-2 text-sm font-medium text-gray-600 dark:text-gray-400 disabled:opacity-30"
            data-testid="wizard-back"
          >
            {t('common.back', 'Back')}
          </button>
          <button
            onClick={submitStep}
            disabled={loading}
            className="px-6 py-2 bg-primary-600 text-white rounded-lg font-medium hover:bg-primary-700 disabled:opacity-50 transition-colors"
            data-testid="wizard-next"
          >
            {loading
              ? '...'
              : currentStep === 4
                ? t('setup.complete', 'Complete Setup')
                : t('common.next', 'Next')}
          </button>
        </div>
      </div>
    </div>
  );
}
