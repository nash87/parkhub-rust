import { useState, useEffect, useRef, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { createColumnHelper } from '@tanstack/react-table';
import {
  Users, SpinnerGap, MagnifyingGlass, Coins,
  PencilSimple, X, Check, Gauge, UserMinus, UserPlus,
} from '@phosphor-icons/react';
import { api, type User } from '../api/client';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';
import { DataTable } from '../components/ui/DataTable';

const columnHelper = createColumnHelper<User>();

export function AdminUsersPage() {
  const { t } = useTranslation();
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [debouncedSearch, setDebouncedSearch] = useState('');
  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editRole, setEditRole] = useState('');
  const [creditUserId, setCreditUserId] = useState<string | null>(null);
  const [creditAmount, setCreditAmount] = useState('');
  const [creditDesc, setCreditDesc] = useState('');
  const [savingRole, setSavingRole] = useState(false);
  const [grantingCredits, setGrantingCredits] = useState(false);
  const [editingQuotaId, setEditingQuotaId] = useState<string | null>(null);
  const [editQuota, setEditQuota] = useState('');
  const [savingQuota, setSavingQuota] = useState(false);

  useEffect(() => { loadUsers(); }, []);

  useEffect(() => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => setDebouncedSearch(search), 200);
    return () => { if (debounceRef.current) clearTimeout(debounceRef.current); };
  }, [search]);

  async function loadUsers() {
    try {
      const res = await api.adminUsers();
      if (res.success && res.data) setUsers(res.data);
    } finally {
      setLoading(false);
    }
  }

  function startEditRole(user: User) {
    setEditingId(user.id);
    setEditRole(user.role);
  }

  async function saveRole(userId: string) {
    setSavingRole(true);
    try {
      const res = await api.adminUpdateUserRole(userId, editRole);
      if (res.success) {
        setUsers(prev => prev.map(u => u.id === userId ? { ...u, role: editRole as User['role'] } : u));
        toast.success(t('admin.roleUpdated'));
        setEditingId(null);
      } else {
        toast.error(res.error?.message || t('admin.roleUpdateFailed'));
      }
    } finally {
      setSavingRole(false);
    }
  }

  async function toggleActive(user: User) {
    const res = await api.adminUpdateUser(user.id, { is_active: !user.is_active });
    if (res.success) {
      setUsers(prev => prev.map(u => u.id === user.id ? { ...u, is_active: !u.is_active } : u));
      toast.success(user.is_active ? t('admin.userDeactivated') : t('admin.userActivated'));
    } else {
      toast.error(res.error?.message || t('admin.userUpdateFailed'));
    }
  }

  async function handleGrantCredits() {
    if (!creditUserId || !creditAmount) return;
    setGrantingCredits(true);
    try {
      const res = await api.adminGrantCredits(creditUserId, Number(creditAmount), creditDesc || undefined);
      if (res.success) {
        toast.success(t('admin.creditsGranted'));
        setCreditUserId(null);
        setCreditAmount('');
        setCreditDesc('');
        await loadUsers();
      } else {
        toast.error(res.error?.message || t('admin.creditsGrantFailed'));
      }
    } finally {
      setGrantingCredits(false);
    }
  }

  function startEditQuota(user: User) {
    setEditingQuotaId(user.id);
    setEditQuota(String(user.credits_monthly_quota));
  }

  async function saveQuota(userId: string) {
    const quota = Number(editQuota);
    if (isNaN(quota) || quota < 0 || quota > 999) {
      toast.error(t('admin.quotaRange'));
      return;
    }
    setSavingQuota(true);
    try {
      const res = await api.adminUpdateUserQuota(userId, quota);
      if (res.success) {
        setUsers(prev => prev.map(u => u.id === userId ? { ...u, credits_monthly_quota: quota } : u));
        toast.success(t('admin.quotaUpdated'));
        setEditingQuotaId(null);
      } else {
        toast.error(res.error?.message || t('admin.quotaUpdateFailed'));
      }
    } finally {
      setSavingQuota(false);
    }
  }

  function roleBadge(role: string) {
    const colors: Record<string, string> = {
      superadmin: 'bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400',
      admin: 'bg-amber-100 dark:bg-amber-900/30 text-amber-600 dark:text-amber-400',
      user: 'bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400',
    };
    return (
      <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${colors[role] || colors.user}`}>
        {role}
      </span>
    );
  }

  const columns = useMemo(() => [
    columnHelper.accessor('name', {
      header: () => t('admin.users'),
      cell: info => (
        <div className="min-w-0">
          <p className="text-sm font-medium text-surface-900 dark:text-white truncate">{info.getValue()}</p>
          <p className="text-xs text-surface-500 dark:text-surface-400 truncate">{info.row.original.email}</p>
        </div>
      ),
      enableSorting: true,
    }),
    columnHelper.accessor('role', {
      header: () => t('admin.editRole'),
      cell: info => {
        const user = info.row.original;
        if (editingId === user.id) {
          return (
            <div className="flex items-center gap-2">
              <select value={editRole} onChange={e => setEditRole(e.target.value)} className="input text-xs py-1 px-2 w-28">
                <option value="user">user</option>
                <option value="admin">admin</option>
                <option value="superadmin">superadmin</option>
              </select>
              <button onClick={() => saveRole(user.id)} disabled={savingRole} className="p-1 rounded hover:bg-emerald-100 dark:hover:bg-emerald-900/30 text-emerald-600" aria-label={t('common.save')}>
                {savingRole ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> : <Check weight="bold" className="w-4 h-4" />}
              </button>
              <button onClick={() => setEditingId(null)} className="p-1 rounded hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-400" aria-label={t('common.cancel')}>
                <X weight="bold" className="w-4 h-4" />
              </button>
            </div>
          );
        }
        return roleBadge(user.role);
      },
      enableSorting: true,
    }),
    columnHelper.accessor('credits_balance', {
      header: () => t('admin.credits'),
      cell: info => <span className="text-sm font-semibold text-surface-900 dark:text-white tabular-nums">{info.getValue()}</span>,
      enableSorting: true,
    }),
    columnHelper.accessor('credits_monthly_quota', {
      header: () => t('admin.monthlyQuota'),
      cell: info => {
        const user = info.row.original;
        if (editingQuotaId === user.id) {
          return (
            <div className="flex items-center gap-2">
              <input type="number" min={0} max={999} value={editQuota} onChange={e => setEditQuota(e.target.value)} className="input text-xs py-1 px-2 w-20" aria-label={t('admin.monthlyQuota')} />
              <button onClick={() => saveQuota(user.id)} disabled={savingQuota} className="p-1 rounded hover:bg-emerald-100 dark:hover:bg-emerald-900/30 text-emerald-600" aria-label={t('admin.saveQuota')}>
                {savingQuota ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> : <Check weight="bold" className="w-4 h-4" />}
              </button>
              <button onClick={() => setEditingQuotaId(null)} className="p-1 rounded hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-400" aria-label={t('admin.cancelEditQuota')}>
                <X weight="bold" className="w-4 h-4" />
              </button>
            </div>
          );
        }
        return (
          <button
            onClick={() => startEditQuota(user)}
            className="inline-flex items-center gap-1.5 text-sm text-surface-700 dark:text-surface-300 hover:text-primary-600 dark:hover:text-primary-400 transition-colors"
            aria-label={t('admin.editQuota', { name: user.name })}
          >
            <span className="font-semibold tabular-nums">{user.credits_monthly_quota}</span>
          </button>
        );
      },
      enableSorting: true,
    }),
    columnHelper.accessor('is_active', {
      header: () => t('admin.status'),
      cell: info => (
        <span className={`inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium ${
          info.getValue()
            ? 'bg-emerald-100 dark:bg-emerald-900/30 text-emerald-600 dark:text-emerald-400'
            : 'bg-surface-100 dark:bg-surface-800 text-surface-500 dark:text-surface-400'
        }`}>
          {info.getValue() ? t('admin.active') : t('admin.inactive')}
        </span>
      ),
      enableSorting: true,
    }),
    columnHelper.display({
      id: 'actions',
      header: '',
      cell: info => {
        const user = info.row.original;
        return (
          <div className="flex items-center justify-end gap-1">
            <button
              onClick={() => startEditRole(user)}
              className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors text-surface-400 hover:text-primary-600"
              title={t('admin.editRole')}
              aria-label={`${t('admin.editRole')} ${user.name}`}
            >
              <PencilSimple weight="bold" className="w-4 h-4" />
            </button>
            <button
              onClick={() => { setCreditUserId(user.id); setCreditAmount(''); setCreditDesc(''); }}
              className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors text-surface-400 hover:text-accent-600"
              title={t('admin.grantCreditsFor')}
              aria-label={`${t('admin.grantCredits')} ${user.name}`}
            >
              <Coins weight="bold" className="w-4 h-4" />
            </button>
            <button
              onClick={() => toggleActive(user)}
              className={`p-2 rounded-lg transition-colors ${
                user.is_active
                  ? 'hover:bg-red-50 dark:hover:bg-red-900/20 text-surface-400 hover:text-red-600'
                  : 'hover:bg-emerald-50 dark:hover:bg-emerald-900/20 text-surface-400 hover:text-emerald-600'
              }`}
              title={user.is_active ? t('admin.deactivate') : t('admin.activate')}
              aria-label={`${user.is_active ? t('admin.deactivate') : t('admin.activate')} ${user.name}`}
            >
              {user.is_active ? <UserMinus weight="bold" className="w-4 h-4" /> : <UserPlus weight="bold" className="w-4 h-4" />}
            </button>
          </div>
        );
      },
    }),
  ], [editingId, editRole, savingRole, editingQuotaId, editQuota, savingQuota, t]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64" role="status" aria-label={t('common.loading')}>
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" />
      </div>
    );
  }

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <h2 className="text-xl font-semibold text-surface-900 dark:text-white">{t('admin.users')}</h2>
          <span className="text-sm text-surface-400 tabular-nums">({users.length})</span>
        </div>

        <div className="relative">
          <MagnifyingGlass weight="bold" className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
          <input
            type="text"
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder={t('admin.searchUsers')}
            className="input pl-9 pr-8 w-full sm:w-64"
            aria-label={t('admin.searchUsers')}
          />
          {search && (
            <button
              type="button"
              onClick={() => setSearch('')}
              aria-label={t('admin.clearSearch')}
              className="absolute right-2 top-1/2 -translate-y-1/2 p-0.5 rounded text-surface-400 hover:text-surface-600 dark:hover:text-surface-200 transition-colors"
            >
              <X weight="bold" className="w-3.5 h-3.5" />
            </button>
          )}
        </div>
      </div>

      {/* Grant Credits Modal */}
      <AnimatePresence>
        {creditUserId && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="overflow-hidden"
          >
            <div className="card p-6 space-y-4">
              <div className="flex items-center justify-between">
                <h3 className="text-sm font-semibold text-surface-900 dark:text-white uppercase tracking-wide">{t('admin.grantCredits')}</h3>
                <button onClick={() => setCreditUserId(null)} className="p-1.5 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors" aria-label={t('common.close')}>
                  <X weight="bold" className="w-5 h-5 text-surface-400" />
                </button>
              </div>
              <p className="text-sm text-surface-500 dark:text-surface-400">
                {t('admin.grantingTo')} <strong className="text-surface-900 dark:text-white">{users.find(u => u.id === creditUserId)?.name}</strong>
              </p>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div>
                  <label htmlFor="credit-amount" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">{t('admin.amount')}</label>
                  <input id="credit-amount" type="number" min={1} value={creditAmount} onChange={e => setCreditAmount(e.target.value)} className="input" placeholder="10" />
                </div>
                <div>
                  <label htmlFor="credit-description" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">{t('admin.description')}</label>
                  <input id="credit-description" type="text" value={creditDesc} onChange={e => setCreditDesc(e.target.value)} className="input" />
                </div>
              </div>
              <div className="flex gap-3">
                <button onClick={handleGrantCredits} disabled={grantingCredits || !creditAmount} className="btn btn-primary">
                  {grantingCredits ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> : <Check weight="bold" className="w-4 h-4" />}
                  {t('admin.grant')}
                </button>
                <button onClick={() => setCreditUserId(null)} className="btn btn-secondary">{t('common.cancel')}</button>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Users Table — TanStack Table with sorting */}
      <DataTable
        data={users}
        columns={columns}
        searchValue={debouncedSearch}
        emptyMessage={search ? t('admin.noUsersMatch') : t('admin.noUsersFound')}
      />
    </motion.div>
  );
}
