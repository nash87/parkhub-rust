import { useEffect, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Buildings, Plus, X, PencilSimple } from '@phosphor-icons/react';
import { api, type TenantInfo, type CreateTenantRequest } from '../api/client';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

export function AdminTenantsPage() {
  const { t } = useTranslation();
  const [tenants, setTenants] = useState<TenantInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [showModal, setShowModal] = useState(false);
  const [editing, setEditing] = useState<TenantInfo | null>(null);
  const [formName, setFormName] = useState('');
  const [formDomain, setFormDomain] = useState('');
  const [formColor, setFormColor] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => { loadTenants(); }, []);

  async function loadTenants() {
    setLoading(true);
    try {
      const res = await api.listTenants();
      if (res.success && res.data) setTenants(res.data);
    } catch {
      toast.error(t('common.error'));
    } finally {
      setLoading(false);
    }
  }

  const openCreate = useCallback(() => {
    setEditing(null);
    setFormName('');
    setFormDomain('');
    setFormColor('');
    setShowModal(true);
  }, []);

  const openEdit = useCallback((tenant: TenantInfo) => {
    setEditing(tenant);
    setFormName(tenant.name);
    setFormDomain(tenant.domain || '');
    setFormColor(tenant.branding?.primary_color || '');
    setShowModal(true);
  }, []);

  const handleSave = useCallback(async () => {
    if (!formName.trim()) return;
    setSaving(true);
    const data: CreateTenantRequest = {
      name: formName.trim(),
      domain: formDomain.trim() || undefined,
      branding: formColor.trim() ? { primary_color: formColor.trim() } : undefined,
    };
    try {
      if (editing) {
        const res = await api.updateTenant(editing.id, data);
        if (res.success) {
          toast.success(t('tenants.updated', 'Tenant updated'));
          setShowModal(false);
          loadTenants();
        }
      } else {
        const res = await api.createTenant(data);
        if (res.success) {
          toast.success(t('tenants.created', 'Tenant created'));
          setShowModal(false);
          loadTenants();
        }
      }
    } catch {
      toast.error(t('common.error'));
    } finally {
      setSaving(false);
    }
  }, [editing, formName, formDomain, formColor, t]);

  if (loading) return (
    <div className="space-y-4">
      <div className="h-8 w-48 skeleton rounded-lg" />
      <div className="space-y-3">
        {[1, 2, 3].map(i => <div key={i} className="h-20 skeleton rounded-2xl" />)}
      </div>
    </div>
  );

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-bold text-surface-900 dark:text-white">
          {t('tenants.title', 'Tenants')}
        </h2>
        <button
          onClick={openCreate}
          className="flex items-center gap-1.5 px-3 py-2 text-sm font-medium rounded-xl bg-primary-600 text-white hover:bg-primary-700 transition-colors"
        >
          <Plus weight="bold" className="w-4 h-4" />
          {t('tenants.create', 'Create Tenant')}
        </button>
      </div>

      {tenants.length === 0 ? (
        <div className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-8 text-center">
          <Buildings weight="light" className="w-10 h-10 text-surface-300 dark:text-surface-600 mx-auto mb-2" />
          <p className="text-sm text-surface-500 dark:text-surface-400">
            {t('tenants.empty', 'No tenants configured. Create one to enable multi-tenant isolation.')}
          </p>
        </div>
      ) : (
        <div className="space-y-3" data-testid="tenants-list">
          {tenants.map(tenant => (
            <div key={tenant.id} className="flex items-center justify-between p-4 bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800" data-testid={`tenant-${tenant.id}`}>
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-xl flex items-center justify-center" style={{ backgroundColor: tenant.branding?.primary_color || '#6366f1' }}>
                  <Buildings weight="bold" className="w-5 h-5 text-white" />
                </div>
                <div>
                  <h3 className="text-sm font-semibold text-surface-900 dark:text-white">{tenant.name}</h3>
                  {tenant.domain && <p className="text-xs text-surface-500 dark:text-surface-400">{tenant.domain}</p>}
                </div>
              </div>
              <div className="flex items-center gap-4">
                <div className="text-xs text-surface-500 dark:text-surface-400 text-right">
                  <span>{tenant.user_count} {t('tenants.users', 'users')}</span>
                  <span className="mx-1">/</span>
                  <span>{tenant.lot_count} {t('tenants.lots', 'lots')}</span>
                </div>
                <button onClick={() => openEdit(tenant)} aria-label={t('common.edit', 'Edit')} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors">
                  <PencilSimple weight="bold" className="w-4 h-4 text-surface-500" />
                </button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Create/Edit modal */}
      <AnimatePresence>
        {showModal && (
          <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={() => setShowModal(false)}>
            <motion.div
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.95 }}
              onClick={(e) => e.stopPropagation()}
              className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-6 max-w-md w-full mx-4 shadow-xl"
            >
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-semibold text-surface-900 dark:text-white">
                  {editing ? t('tenants.editTitle', 'Edit Tenant') : t('tenants.create', 'Create Tenant')}
                </h3>
                <button onClick={() => setShowModal(false)} className="p-1 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors">
                  <X weight="bold" className="w-5 h-5 text-surface-500" />
                </button>
              </div>

              <div className="space-y-4">
                <div>
                  <label className="block text-xs font-medium text-surface-700 dark:text-surface-300 mb-1">
                    {t('tenants.name', 'Name')}
                  </label>
                  <input
                    type="text"
                    value={formName}
                    onChange={(e) => setFormName(e.target.value)}
                    className="w-full px-3 py-2 text-sm rounded-xl border border-surface-200 dark:border-surface-700 bg-white dark:bg-surface-800 text-surface-900 dark:text-white"
                    data-testid="tenant-name-input"
                  />
                </div>
                <div>
                  <label className="block text-xs font-medium text-surface-700 dark:text-surface-300 mb-1">
                    {t('tenants.domain', 'Domain')}
                  </label>
                  <input
                    type="text"
                    value={formDomain}
                    onChange={(e) => setFormDomain(e.target.value)}
                    placeholder="example.com"
                    className="w-full px-3 py-2 text-sm rounded-xl border border-surface-200 dark:border-surface-700 bg-white dark:bg-surface-800 text-surface-900 dark:text-white"
                    data-testid="tenant-domain-input"
                  />
                </div>
                <div>
                  <label className="block text-xs font-medium text-surface-700 dark:text-surface-300 mb-1">
                    {t('tenants.brandColor', 'Brand Color')}
                  </label>
                  <input
                    type="color"
                    value={formColor || '#6366f1'}
                    onChange={(e) => setFormColor(e.target.value)}
                    className="w-full h-10 rounded-xl border border-surface-200 dark:border-surface-700 cursor-pointer"
                  />
                </div>
              </div>

              <div className="flex justify-end gap-2 mt-6">
                <button onClick={() => setShowModal(false)} className="px-4 py-2 text-sm rounded-xl text-surface-700 dark:text-surface-300 hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors">
                  {t('common.cancel', 'Cancel')}
                </button>
                <button
                  onClick={handleSave}
                  disabled={saving || !formName.trim()}
                  className="px-4 py-2 text-sm font-medium rounded-xl bg-primary-600 text-white hover:bg-primary-700 disabled:opacity-50 transition-colors"
                >
                  {saving ? t('common.saving', 'Saving...') : t('common.save', 'Save')}
                </button>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}

export default AdminTenantsPage;
