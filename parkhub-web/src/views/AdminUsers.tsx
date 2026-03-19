import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Users, SpinnerGap, MagnifyingGlass, Coins,
  PencilSimple, X, Check, Gauge, UserMinus, UserPlus,
} from '@phosphor-icons/react';
import { api, type User } from '../api/client';
import toast from 'react-hot-toast';

export function AdminUsersPage() {
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
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

  async function loadUsers() {
    try {
      const res = await api.adminUsers();
      if (res.success && res.data) setUsers(res.data);
    } finally {
      setLoading(false);
    }
  }

  const filtered = users.filter(u =>
    u.name.toLowerCase().includes(search.toLowerCase()) ||
    u.email.toLowerCase().includes(search.toLowerCase()) ||
    u.username.toLowerCase().includes(search.toLowerCase())
  );

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
        toast.success('Role updated');
        setEditingId(null);
      } else {
        toast.error(res.error?.message || 'Failed to update role');
      }
    } finally {
      setSavingRole(false);
    }
  }

  async function toggleActive(user: User) {
    const res = await api.adminUpdateUser(user.id, { is_active: !user.is_active });
    if (res.success) {
      setUsers(prev => prev.map(u => u.id === user.id ? { ...u, is_active: !u.is_active } : u));
      toast.success(user.is_active ? 'User deactivated' : 'User activated');
    } else {
      toast.error(res.error?.message || 'Failed to update user');
    }
  }

  async function handleGrantCredits() {
    if (!creditUserId || !creditAmount) return;
    setGrantingCredits(true);
    try {
      const res = await api.adminGrantCredits(creditUserId, Number(creditAmount), creditDesc || undefined);
      if (res.success) {
        toast.success('Credits granted');
        setCreditUserId(null);
        setCreditAmount('');
        setCreditDesc('');
        await loadUsers();
      } else {
        toast.error(res.error?.message || 'Failed to grant credits');
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
      toast.error('Quota must be 0-999');
      return;
    }
    setSavingQuota(true);
    try {
      const res = await api.adminUpdateUserQuota(userId, quota);
      if (res.success) {
        setUsers(prev => prev.map(u => u.id === userId ? { ...u, credits_monthly_quota: quota } : u));
        toast.success('Quota updated');
        setEditingQuotaId(null);
      } else {
        toast.error(res.error?.message || 'Failed to update quota');
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
      <span className={`inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-xs font-medium ${colors[role] || colors.user}`}>
        <ShieldCheck weight="fill" className="w-3.5 h-3.5" />
        {role}
      </span>
    );
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" />
      </div>
    );
  }

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <Users weight="fill" className="w-6 h-6 text-primary-600" />
          <h2 className="text-xl font-semibold text-surface-900 dark:text-white">Users</h2>
          <span className="text-sm text-surface-400">({users.length})</span>
        </div>

        <div className="relative">
          <MagnifyingGlass weight="bold" className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
          <input
            type="text"
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder="Search users..."
            className="input pl-9 w-64"
            aria-label="Search users"
          />
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
                <h3 className="text-lg font-semibold text-surface-900 dark:text-white flex items-center gap-2">
                  <Coins weight="fill" className="w-5 h-5 text-primary-600" />
                  Grant Credits
                </h3>
                <button onClick={() => setCreditUserId(null)} className="p-1.5 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors">
                  <X weight="bold" className="w-5 h-5 text-surface-400" />
                </button>
              </div>
              <p className="text-sm text-surface-500 dark:text-surface-400">
                Granting to: <strong className="text-surface-900 dark:text-white">{users.find(u => u.id === creditUserId)?.name}</strong>
              </p>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div>
                  <label htmlFor="credit-amount" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">Amount</label>
                  <input id="credit-amount" type="number" min={1} value={creditAmount} onChange={e => setCreditAmount(e.target.value)} className="input" placeholder="10" />
                </div>
                <div>
                  <label htmlFor="credit-description" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">Description (optional)</label>
                  <input id="credit-description" type="text" value={creditDesc} onChange={e => setCreditDesc(e.target.value)} className="input" placeholder="Bonus credits" />
                </div>
              </div>
              <div className="flex gap-3">
                <button onClick={handleGrantCredits} disabled={grantingCredits || !creditAmount} className="btn btn-primary">
                  {grantingCredits ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> : <Check weight="bold" className="w-4 h-4" />}
                  Grant
                </button>
                <button onClick={() => setCreditUserId(null)} className="btn btn-secondary">Cancel</button>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Users Table */}
      <div className="card overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-surface-200 dark:border-surface-700">
                <th className="text-left px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">User</th>
                <th className="text-left px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">Role</th>
                <th className="text-left px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">Credits</th>
                <th className="text-left px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">Quota/mo</th>
                <th className="text-left px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">Status</th>
                <th className="text-right px-5 py-3.5 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">Actions</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-surface-100 dark:divide-surface-800">
              {filtered.map((user, i) => (
                <motion.tr
                  key={user.id}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  transition={{ delay: i * 0.02 }}
                  className="hover:bg-surface-50 dark:hover:bg-surface-800/50 transition-colors"
                >
                  <td className="px-5 py-4">
                    <div className="flex items-center gap-3">
                      <div className="w-9 h-9 rounded-xl bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center">
                        <UserCircle weight="fill" className="w-5 h-5 text-primary-600 dark:text-primary-400" />
                      </div>
                      <div className="min-w-0">
                        <p className="text-sm font-medium text-surface-900 dark:text-white truncate">{user.name}</p>
                        <p className="text-xs text-surface-500 dark:text-surface-400 truncate">{user.email}</p>
                      </div>
                    </div>
                  </td>
                  <td className="px-5 py-4">
                    {editingId === user.id ? (
                      <div className="flex items-center gap-2">
                        <select value={editRole} onChange={e => setEditRole(e.target.value)} className="input text-xs py-1 px-2 w-28">
                          <option value="user">user</option>
                          <option value="admin">admin</option>
                          <option value="superadmin">superadmin</option>
                        </select>
                        <button onClick={() => saveRole(user.id)} disabled={savingRole} className="p-1 rounded hover:bg-emerald-100 dark:hover:bg-emerald-900/30 text-emerald-600">
                          {savingRole ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> : <Check weight="bold" className="w-4 h-4" />}
                        </button>
                        <button onClick={() => setEditingId(null)} className="p-1 rounded hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-400">
                          <X weight="bold" className="w-4 h-4" />
                        </button>
                      </div>
                    ) : (
                      roleBadge(user.role)
                    )}
                  </td>
                  <td className="px-5 py-4">
                    <span className="text-sm font-semibold text-surface-900 dark:text-white">{user.credits_balance}</span>
                  </td>
                  <td className="px-5 py-4">
                    {editingQuotaId === user.id ? (
                      <div className="flex items-center gap-2">
                        <input
                          type="number"
                          min={0}
                          max={999}
                          value={editQuota}
                          onChange={e => setEditQuota(e.target.value)}
                          className="input text-xs py-1 px-2 w-20"
                        />
                        <button onClick={() => saveQuota(user.id)} disabled={savingQuota} className="p-1 rounded hover:bg-emerald-100 dark:hover:bg-emerald-900/30 text-emerald-600">
                          {savingQuota ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> : <Check weight="bold" className="w-4 h-4" />}
                        </button>
                        <button onClick={() => setEditingQuotaId(null)} className="p-1 rounded hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-400">
                          <X weight="bold" className="w-4 h-4" />
                        </button>
                      </div>
                    ) : (
                      <button
                        onClick={() => startEditQuota(user)}
                        className="inline-flex items-center gap-1.5 text-sm text-surface-700 dark:text-surface-300 hover:text-primary-600 dark:hover:text-primary-400 transition-colors"
                        title="Edit quota"
                      >
                        <span className="font-semibold">{user.credits_monthly_quota}</span>
                        <Gauge weight="bold" className="w-3.5 h-3.5 opacity-0 group-hover:opacity-100" />
                      </button>
                    )}
                  </td>
                  <td className="px-5 py-4">
                    <span className={`inline-flex items-center px-2.5 py-1 rounded-full text-xs font-medium ${
                      user.is_active
                        ? 'bg-emerald-100 dark:bg-emerald-900/30 text-emerald-600 dark:text-emerald-400'
                        : 'bg-surface-100 dark:bg-surface-800 text-surface-500 dark:text-surface-400'
                    }`}>
                      {user.is_active ? 'Active' : 'Inactive'}
                    </span>
                  </td>
                  <td className="px-5 py-4">
                    <div className="flex items-center justify-end gap-1">
                      <button
                        onClick={() => startEditRole(user)}
                        className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors text-surface-400 hover:text-primary-600"
                        title="Edit role"
                      >
                        <PencilSimple weight="bold" className="w-4 h-4" />
                      </button>
                      <button
                        onClick={() => { setCreditUserId(user.id); setCreditAmount(''); setCreditDesc(''); }}
                        className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors text-surface-400 hover:text-accent-600"
                        title="Grant credits"
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
                        title={user.is_active ? 'Deactivate' : 'Activate'}
                      >
                        {user.is_active
                          ? <UserMinus weight="bold" className="w-4 h-4" />
                          : <UserPlus weight="bold" className="w-4 h-4" />
                        }
                      </button>
                    </div>
                  </td>
                </motion.tr>
              ))}
            </tbody>
          </table>
        </div>

        {filtered.length === 0 && (
          <div className="p-12 text-center">
            <Users weight="light" className="w-16 h-16 text-surface-200 dark:text-surface-700 mx-auto mb-4" />
            <p className="text-surface-500 dark:text-surface-400">
              {search ? 'No users match your search.' : 'No users found.'}
            </p>
          </div>
        )}
      </div>
    </motion.div>
  );
}
