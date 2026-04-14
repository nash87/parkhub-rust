import { useEffect, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ShieldCheck, Plus, Trash, Pencil, Question, UserCircle } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

const ALL_PERMISSIONS = [
  'manage_users',
  'manage_lots',
  'manage_bookings',
  'view_reports',
  'manage_settings',
  'manage_plugins',
];

interface RbacRole {
  id: string;
  name: string;
  description: string | null;
  permissions: string[];
  built_in: boolean;
  created_at: string;
  updated_at: string;
}

interface UserRoleAssignment {
  user_id: string;
  roles: { id: string; name: string; permissions: string[] }[];
}

export function AdminRolesPage() {
  const { t } = useTranslation();
  const [roles, setRoles] = useState<RbacRole[]>([]);
  const [loading, setLoading] = useState(true);
  const [showHelp, setShowHelp] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);
  const [formName, setFormName] = useState('');
  const [formDesc, setFormDesc] = useState('');
  const [formPerms, setFormPerms] = useState<string[]>([]);

  const loadRoles = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch('/api/v1/admin/roles').then(r => r.json());
      if (res.success) {
        setRoles(res.data || []);
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => { loadRoles(); }, [loadRoles]);

  function resetForm() {
    setShowForm(false);
    setEditId(null);
    setFormName('');
    setFormDesc('');
    setFormPerms([]);
  }

  function startEdit(role: RbacRole) {
    setEditId(role.id);
    setFormName(role.name);
    setFormDesc(role.description || '');
    setFormPerms([...role.permissions]);
    setShowForm(true);
  }

  function togglePerm(perm: string) {
    setFormPerms(prev =>
      prev.includes(perm) ? prev.filter(p => p !== perm) : [...prev, perm]
    );
  }

  async function handleSave() {
    if (!formName.trim()) {
      toast.error(t('rbac.nameRequired'));
      return;
    }

    try {
      const url = editId
        ? `/api/v1/admin/roles/${editId}`
        : '/api/v1/admin/roles';
      const method = editId ? 'PUT' : 'POST';
      const body = editId
        ? { name: formName, description: formDesc || null, permissions: formPerms }
        : { name: formName, description: formDesc || null, permissions: formPerms };

      const res = await fetch(url, {
        method,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      }).then(r => r.json());

      if (res.success) {
        toast.success(editId ? t('rbac.updated') : t('rbac.created'));
        resetForm();
        loadRoles();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
  }

  async function handleDelete(role: RbacRole) {
    if (role.built_in) return;
    if (!confirm(t('rbac.deleteConfirm'))) return;

    try {
      const res = await fetch(`/api/v1/admin/roles/${role.id}`, { method: 'DELETE' }).then(r => r.json());
      if (res.success) {
        toast.success(t('rbac.deleted'));
        loadRoles();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-xl font-bold text-surface-900 dark:text-white flex items-center gap-2">
            <ShieldCheck weight="duotone" className="w-6 h-6 text-primary-500" />
            {t('rbac.title')}
          </h2>
          <p className="text-sm text-surface-500 dark:text-surface-400 mt-1">
            {t('rbac.subtitle')}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setShowHelp(h => !h)}
            className="p-2 rounded-lg text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-800"
            title={t('rbac.helpLabel')}
          >
            <Question className="w-5 h-5" />
          </button>
          <button
            onClick={() => { resetForm(); setShowForm(true); }}
            className="flex items-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-xl hover:bg-primary-700 transition-colors"
          >
            <Plus className="w-4 h-4" />
            {t('rbac.createRole')}
          </button>
        </div>
      </div>

      {/* Help tooltip */}
      <AnimatePresence>
        {showHelp && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-xl p-4 text-sm text-blue-700 dark:text-blue-300"
          >
            {t('rbac.help')}
          </motion.div>
        )}
      </AnimatePresence>

      {/* Create/Edit form */}
      <AnimatePresence>
        {showForm && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            className="bg-white dark:bg-surface-800 rounded-2xl border border-surface-200 dark:border-surface-700 p-6 space-y-4"
          >
            <h3 className="font-semibold text-surface-900 dark:text-white">
              {editId ? t('rbac.editRole') : t('rbac.newRole')}
            </h3>

            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                {t('rbac.name')}
              </label>
              <input
                type="text"
                value={formName}
                onChange={e => setFormName(e.target.value)}
                className="w-full px-3 py-2 rounded-xl border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-surface-900 dark:text-white"
                placeholder={t('rbac.namePlaceholder')}
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1">
                {t('rbac.description')}
              </label>
              <input
                type="text"
                value={formDesc}
                onChange={e => setFormDesc(e.target.value)}
                className="w-full px-3 py-2 rounded-xl border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-surface-900 dark:text-white"
                placeholder={t('rbac.descriptionPlaceholder')}
              />
            </div>

            <div>
              <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">
                {t('rbac.permissions')}
              </label>
              <div className="grid grid-cols-2 sm:grid-cols-3 gap-2">
                {ALL_PERMISSIONS.map(perm => (
                  <label
                    key={perm}
                    className={`flex items-center gap-2 p-2 rounded-lg border cursor-pointer transition-colors ${
                      formPerms.includes(perm)
                        ? 'bg-primary-50 dark:bg-primary-900/20 border-primary-300 dark:border-primary-700'
                        : 'bg-surface-50 dark:bg-surface-900 border-surface-200 dark:border-surface-700'
                    }`}
                  >
                    <input
                      type="checkbox"
                      checked={formPerms.includes(perm)}
                      onChange={() => togglePerm(perm)}
                      className="rounded"
                    />
                    <span className="text-sm text-surface-700 dark:text-surface-300">
                      {t(`rbac.perm.${perm}`)}
                    </span>
                  </label>
                ))}
              </div>
            </div>

            <div className="flex gap-2 pt-2">
              <button
                onClick={handleSave}
                className="px-4 py-2 bg-primary-600 text-white rounded-xl hover:bg-primary-700 transition-colors"
              >
                {t('rbac.save')}
              </button>
              <button
                onClick={resetForm}
                className="px-4 py-2 bg-surface-100 dark:bg-surface-700 text-surface-700 dark:text-surface-300 rounded-xl hover:bg-surface-200 dark:hover:bg-surface-600 transition-colors"
              >
                {t('common.cancel')}
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Roles list */}
      {loading ? (
        <div className="text-center py-12 text-surface-400">{t('common.loading')}</div>
      ) : roles.length === 0 ? (
        <div className="text-center py-12 text-surface-400">{t('rbac.empty')}</div>
      ) : (
        <div className="grid gap-4">
          {roles.map(role => (
            <motion.div
              key={role.id}
              layout
              className="bg-white dark:bg-surface-800 rounded-2xl border border-surface-200 dark:border-surface-700 p-5"
            >
              <div className="flex items-start justify-between">
                <div>
                  <div className="flex items-center gap-2">
                    <h3 className="font-semibold text-surface-900 dark:text-white">
                      {role.name}
                    </h3>
                    {role.built_in && (
                      <span className="px-2 py-0.5 text-xs rounded-full bg-amber-100 dark:bg-amber-900/30 text-amber-700 dark:text-amber-400">
                        {t('rbac.builtIn')}
                      </span>
                    )}
                  </div>
                  {role.description && (
                    <p className="text-sm text-surface-500 dark:text-surface-400 mt-1">
                      {role.description}
                    </p>
                  )}
                </div>
                <div className="flex items-center gap-1">
                  <button
                    onClick={() => startEdit(role)}
                    className="p-2 rounded-lg text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-700"
                    title={t('rbac.edit')}
                  >
                    <Pencil className="w-4 h-4" />
                  </button>
                  {!role.built_in && (
                    <button
                      onClick={() => handleDelete(role)}
                      className="p-2 rounded-lg text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20"
                      title={t('rbac.delete')}
                    >
                      <Trash className="w-4 h-4" />
                    </button>
                  )}
                </div>
              </div>

              {/* Permission badges */}
              <div className="flex flex-wrap gap-1.5 mt-3">
                {role.permissions.length === 0 ? (
                  <span className="text-xs text-surface-400 italic">{t('rbac.noPermissions')}</span>
                ) : (
                  role.permissions.map(perm => (
                    <span
                      key={perm}
                      className="px-2 py-0.5 text-xs rounded-full bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-400"
                    >
                      {t(`rbac.perm.${perm}`)}
                    </span>
                  ))
                )}
              </div>
            </motion.div>
          ))}
        </div>
      )}
    </div>
  );
}
