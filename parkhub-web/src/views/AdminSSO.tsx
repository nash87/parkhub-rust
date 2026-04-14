import { useEffect, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ShieldCheck, Plus, Trash, Pencil, Question, ToggleLeft, ToggleRight } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

interface SsoProvider {
  slug: string;
  display_name: string;
  entity_id: string;
  metadata_url: string;
  sso_url: string;
  certificate: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export function AdminSSOPage() {
  const { t } = useTranslation();
  const [providers, setProviders] = useState<SsoProvider[]>([]);
  const [loading, setLoading] = useState(true);
  const [showHelp, setShowHelp] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [editSlug, setEditSlug] = useState<string | null>(null);
  const [formSlug, setFormSlug] = useState('');
  const [formDisplayName, setFormDisplayName] = useState('');
  const [formEntityId, setFormEntityId] = useState('');
  const [formMetadataUrl, setFormMetadataUrl] = useState('');
  const [formSsoUrl, setFormSsoUrl] = useState('');
  const [formCertificate, setFormCertificate] = useState('');

  const loadProviders = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch('/api/v1/auth/sso/providers').then(r => r.json());
      if (res.success) {
        setProviders(res.data?.providers || []);
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => {
    loadProviders();
  }, [loadProviders]);

  function resetForm() {
    setFormSlug('');
    setFormDisplayName('');
    setFormEntityId('');
    setFormMetadataUrl('');
    setFormSsoUrl('');
    setFormCertificate('');
    setEditSlug(null);
    setShowForm(false);
  }

  function openEdit(p: SsoProvider) {
    setFormSlug(p.slug);
    setFormDisplayName(p.display_name);
    setFormEntityId(p.entity_id);
    setFormMetadataUrl(p.metadata_url);
    setFormSsoUrl(p.sso_url);
    setFormCertificate(p.certificate);
    setEditSlug(p.slug);
    setShowForm(true);
  }

  async function handleSave() {
    if (!formSlug.trim() || !formDisplayName.trim() || !formEntityId.trim() || !formSsoUrl.trim() || !formCertificate.trim()) {
      toast.error(t('sso.requiredFields'));
      return;
    }

    const slug = editSlug || formSlug.trim().toLowerCase().replace(/[^a-z0-9-]/g, '-');
    try {
      const res = await fetch(`/api/v1/admin/sso/${slug}`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          display_name: formDisplayName.trim(),
          entity_id: formEntityId.trim(),
          metadata_url: formMetadataUrl.trim(),
          sso_url: formSsoUrl.trim(),
          certificate: formCertificate.trim(),
          enabled: true,
        }),
      }).then(r => r.json());

      if (res.success) {
        toast.success(editSlug ? t('sso.updated') : t('sso.created'));
        resetForm();
        loadProviders();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
  }

  async function handleDelete(slug: string) {
    try {
      const res = await fetch(`/api/v1/admin/sso/${slug}`, { method: 'DELETE' }).then(r => r.json());
      if (res.success) {
        toast.success(t('sso.deleted'));
        loadProviders();
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
  }

  return (
    <div className="space-y-6 max-w-4xl">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-semibold text-surface-900 dark:text-surface-100 flex items-center gap-2">
            <ShieldCheck size={24} weight="bold" className="text-primary-500" />
            {t('sso.title')}
          </h2>
          <p className="text-sm text-surface-500 mt-1">{t('sso.subtitle')}</p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setShowHelp(!showHelp)}
            className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-500"
            aria-label={t('sso.helpLabel')}
          >
            <Question size={20} />
          </button>
          <button
            onClick={() => { resetForm(); setShowForm(true); }}
            className="flex items-center gap-2 px-4 py-2 rounded-lg bg-primary-500 text-white hover:bg-primary-600 transition-colors text-sm font-medium"
          >
            <Plus size={16} weight="bold" />
            {t('sso.addProvider')}
          </button>
        </div>
      </div>

      {/* Help */}
      <AnimatePresence>
        {showHelp && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="bg-primary-50 dark:bg-primary-950/30 border border-primary-200 dark:border-primary-800 rounded-lg p-4 text-sm text-surface-700 dark:text-surface-300"
          >
            {t('sso.help')}
          </motion.div>
        )}
      </AnimatePresence>

      {/* Form */}
      <AnimatePresence>
        {showForm && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            className="bg-white dark:bg-surface-800 rounded-xl border border-surface-200 dark:border-surface-700 p-6 space-y-4"
          >
            <h3 className="font-semibold text-surface-900 dark:text-surface-100">
              {editSlug ? t('sso.editProvider') : t('sso.newProvider')}
            </h3>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-xs font-medium text-surface-600 dark:text-surface-400 mb-1">{t('sso.slug')}</label>
                <input
                  type="text"
                  value={formSlug}
                  onChange={e => setFormSlug(e.target.value)}
                  disabled={!!editSlug}
                  placeholder="e.g. okta, azure-ad"
                  className="w-full px-3 py-2 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-sm"
                />
              </div>
              <div>
                <label className="block text-xs font-medium text-surface-600 dark:text-surface-400 mb-1">{t('sso.displayName')}</label>
                <input
                  type="text"
                  value={formDisplayName}
                  onChange={e => setFormDisplayName(e.target.value)}
                  placeholder="e.g. Okta, Azure AD"
                  className="w-full px-3 py-2 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-sm"
                />
              </div>
              <div>
                <label className="block text-xs font-medium text-surface-600 dark:text-surface-400 mb-1">{t('sso.entityId')}</label>
                <input
                  type="text"
                  value={formEntityId}
                  onChange={e => setFormEntityId(e.target.value)}
                  placeholder="https://idp.example.com/entity"
                  className="w-full px-3 py-2 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-sm"
                />
              </div>
              <div>
                <label className="block text-xs font-medium text-surface-600 dark:text-surface-400 mb-1">{t('sso.ssoUrl')}</label>
                <input
                  type="text"
                  value={formSsoUrl}
                  onChange={e => setFormSsoUrl(e.target.value)}
                  placeholder="https://idp.example.com/sso"
                  className="w-full px-3 py-2 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-sm"
                />
              </div>
              <div className="md:col-span-2">
                <label className="block text-xs font-medium text-surface-600 dark:text-surface-400 mb-1">{t('sso.metadataUrl')}</label>
                <input
                  type="text"
                  value={formMetadataUrl}
                  onChange={e => setFormMetadataUrl(e.target.value)}
                  placeholder="https://idp.example.com/metadata"
                  className="w-full px-3 py-2 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-sm"
                />
              </div>
              <div className="md:col-span-2">
                <label className="block text-xs font-medium text-surface-600 dark:text-surface-400 mb-1">{t('sso.certificate')}</label>
                <textarea
                  value={formCertificate}
                  onChange={e => setFormCertificate(e.target.value)}
                  placeholder="Base64-encoded X.509 certificate"
                  rows={3}
                  className="w-full px-3 py-2 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-sm font-mono"
                />
              </div>
            </div>

            <div className="flex justify-end gap-2 pt-2">
              <button onClick={resetForm} className="px-4 py-2 rounded-lg text-sm text-surface-600 hover:bg-surface-100 dark:hover:bg-surface-700">
                {t('common.cancel')}
              </button>
              <button onClick={handleSave} className="px-4 py-2 rounded-lg text-sm font-medium bg-primary-500 text-white hover:bg-primary-600">
                {t('sso.save')}
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Provider list */}
      {loading ? (
        <div className="text-center py-8 text-surface-400">{t('common.loading')}</div>
      ) : providers.length === 0 ? (
        <div className="text-center py-12 text-surface-400">
          <ShieldCheck size={48} className="mx-auto mb-3 opacity-30" />
          <p>{t('sso.empty')}</p>
        </div>
      ) : (
        <div className="space-y-3">
          {providers.map(p => (
            <motion.div
              key={p.slug}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              className="bg-white dark:bg-surface-800 rounded-xl border border-surface-200 dark:border-surface-700 p-4 flex items-center justify-between"
            >
              <div className="flex items-center gap-3">
                <ShieldCheck size={24} weight="bold" className="text-primary-500" />
                <div>
                  <p className="font-medium text-surface-900 dark:text-surface-100">{p.display_name}</p>
                  <p className="text-xs text-surface-500">{p.slug} &middot; {p.entity_id}</p>
                </div>
              </div>
              <div className="flex items-center gap-2">
                {p.enabled ? (
                  <ToggleRight size={24} className="text-green-500" />
                ) : (
                  <ToggleLeft size={24} className="text-surface-400" />
                )}
                <button
                  onClick={() => openEdit(p)}
                  className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700 text-surface-500"
                  aria-label={t('sso.edit')}
                >
                  <Pencil size={16} />
                </button>
                <button
                  onClick={() => handleDelete(p.slug)}
                  className="p-2 rounded-lg hover:bg-red-50 dark:hover:bg-red-950/30 text-red-500"
                  aria-label={t('sso.delete')}
                >
                  <Trash size={16} />
                </button>
              </div>
            </motion.div>
          ))}
        </div>
      )}
    </div>
  );
}
