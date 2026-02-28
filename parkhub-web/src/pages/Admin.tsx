import { useEffect, useState } from 'react';
import { Routes, Route, Link, useLocation } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import {
  ChartBar,
  Buildings,
  Users,
  ListChecks,
  Plus,
  CheckCircle,
  TrendUp,
  CaretRight,
  SpinnerGap,
  MagnifyingGlass,
  CaretLeft,
  PencilSimple,
  Trash,
  X,
  Warning,
  Funnel,
  DownloadSimple,
  ArrowClockwise,
  ProhibitInset,
  CheckFat,
} from '@phosphor-icons/react';
import { api, ParkingLot, ParkingLotDetailed, AdminUser, AdminBooking, CreateLotData } from '../api/client';
import { LotLayoutEditor } from '../components/LotLayoutEditor';
import { format } from 'date-fns';
import { de } from 'date-fns/locale';
import toast from 'react-hot-toast';

const tabs = [
  { name: 'Übersicht', path: '/admin', icon: ChartBar },
  { name: 'Parkplätze', path: '/admin/lots', icon: Buildings },
  { name: 'Benutzer', path: '/admin/users', icon: Users },
  { name: 'Buchungen', path: '/admin/bookings', icon: ListChecks },
];

function AdminNav() {
  const location = useLocation();

  return (
    <div className="border-b border-gray-200 dark:border-gray-800 mb-8">
      <nav className="flex gap-1 overflow-x-auto">
        {tabs.map((tab) => {
          const Icon = tab.icon;
          const isActive = location.pathname === tab.path;
          
          return (
            <Link
              key={tab.path}
              to={tab.path}
              className={`flex items-center gap-2 px-4 py-3 text-sm font-medium whitespace-nowrap border-b-2 transition-colors ${
                isActive
                  ? 'border-primary-600 text-primary-600 dark:text-primary-400'
                  : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
              }`}
            >
              <Icon weight={isActive ? 'fill' : 'regular'} className="w-5 h-5" />
              {tab.name}
            </Link>
          );
        })}
      </nav>
    </div>
  );
}

function AdminOverview() {
  const [stats, setStats] = useState({
    totalLots: 0,
    totalSlots: 0,
    availableSlots: 0,
  });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadStats();
  }, []);

  async function loadStats() {
    try {
      const lotsRes = await api.getLots();
      if (lotsRes.success && lotsRes.data) {
        const lots = lotsRes.data;
        setStats({
          totalLots: lots.length,
          totalSlots: lots.reduce((sum, l) => sum + l.total_slots, 0),
          availableSlots: lots.reduce((sum, l) => sum + l.available_slots, 0),
        });
      }
    } finally {
      setLoading(false);
    }
  }

  const occupancyRate = stats.totalSlots > 0
    ? Math.round((1 - stats.availableSlots / stats.totalSlots) * 100)
    : 0;

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" />
      </div>
    );
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="space-y-8"
    >
      <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
        System-Übersicht
      </h2>
      
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <div className="stat-card">
          <div className="flex items-center justify-between">
            <div>
              <p className="stat-label">Parkplätze</p>
              <p className="stat-value text-gray-900 dark:text-white">{stats.totalLots}</p>
            </div>
            <Buildings weight="fill" className="w-8 h-8 text-gray-300 dark:text-gray-700" />
          </div>
        </div>
        
        <div className="stat-card">
          <div className="flex items-center justify-between">
            <div>
              <p className="stat-label">Stellplätze gesamt</p>
              <p className="stat-value text-gray-900 dark:text-white">{stats.totalSlots}</p>
            </div>
            <ListChecks weight="fill" className="w-8 h-8 text-gray-300 dark:text-gray-700" />
          </div>
        </div>
        
        <div className="stat-card">
          <div className="flex items-center justify-between">
            <div>
              <p className="stat-label">Verfügbar</p>
              <p className="stat-value text-emerald-600">{stats.availableSlots}</p>
            </div>
            <CheckCircle weight="fill" className="w-8 h-8 text-emerald-200 dark:text-emerald-900" />
          </div>
        </div>
        
        <div className="stat-card">
          <div className="flex items-center justify-between">
            <div>
              <p className="stat-label">Auslastung</p>
              <p className="stat-value text-primary-600">{occupancyRate}%</p>
            </div>
            <TrendUp weight="fill" className="w-8 h-8 text-primary-200 dark:text-primary-900" />
          </div>
        </div>
      </div>

      <div className="card p-6">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Schnellaktionen
        </h3>
        <div className="flex flex-wrap gap-3">
          <Link to="/admin/lots" className="btn btn-primary">
            <Plus weight="bold" className="w-4 h-4" />
            Parkplatz anlegen
          </Link>
          <Link to="/admin/users" className="btn btn-secondary">
            <Users weight="regular" className="w-4 h-4" />
            Benutzer verwalten
          </Link>
        </div>
      </div>
    </motion.div>
  );
}

function AdminLots() {
  const [lots, setLots] = useState<ParkingLot[]>([]);
  const [loading, setLoading] = useState(true);
  const [editingLotId, setEditingLotId] = useState<string | null>(null);
  const [editingLayout, setEditingLayout] = useState<ParkingLotDetailed | null>(null);
  const [showNewEditor, setShowNewEditor] = useState(false);

  useEffect(() => {
    loadLots();
  }, []);

  async function loadLots() {
    try {
      const res = await api.getLots();
      if (res.success && res.data) setLots(res.data);
    } finally {
      setLoading(false);
    }
  }

  async function handleEdit(lot: ParkingLot) {
    if (editingLotId === lot.id) {
      setEditingLotId(null);
      setEditingLayout(null);
      return;
    }
    const res = await api.getLotDetailed(lot.id);
    if (res.success && res.data) {
      setEditingLotId(lot.id);
      setEditingLayout(res.data);
      setShowNewEditor(false);
    }
  }

  async function handleCreateLot(_layout: unknown, name: string) {
    if (!name.trim()) {
      toast.error('Bitte geben Sie einen Namen ein');
      return;
    }
    const lotData: CreateLotData = {
      name: name.trim(),
      address: '',
      total_slots: 0,
      available_slots: 0,
      latitude: 0,
      longitude: 0,
      floors: [],
      amenities: [],
      images: [],
    };
    const res = await api.createLot(lotData);
    if (res.success && res.data) {
      setLots(prev => [...prev, res.data!]);
      setShowNewEditor(false);
      toast.success(`Parkplatz "${name}" erstellt`);
    } else {
      toast.error(res.error?.message ?? 'Fehler beim Erstellen des Parkplatzes');
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" />
      </div>
    );
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="space-y-6"
    >
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
          Parkplätze verwalten
        </h2>
        <button
          onClick={() => { setShowNewEditor((p) => !p); setEditingLotId(null); }}
          className="btn btn-primary"
        >
          <Plus weight="bold" className="w-4 h-4" />
          Neuer Parkplatz
        </button>
      </div>

      {/* New lot editor */}
      <AnimatePresence>
        {showNewEditor && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="overflow-hidden"
          >
            <div className="card p-6">
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                Neuen Parkplatz anlegen
              </h3>
              <LotLayoutEditor
                onSave={handleCreateLot}
                onCancel={() => setShowNewEditor(false)}
              />
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {lots.length === 0 && !showNewEditor && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="card p-12 text-center"
        >
          <Buildings weight="light" className="w-16 h-16 text-gray-300 dark:text-gray-700 mx-auto mb-4" aria-hidden="true" />
          <p className="text-gray-700 dark:text-gray-300 font-medium mb-1">
            Noch keine Parkplätze angelegt
          </p>
          <p className="text-sm text-gray-500 dark:text-gray-400 mb-6">
            Erstellen Sie Ihren ersten Parkplatz, um das System zu nutzen.
          </p>
          <button
            onClick={() => setShowNewEditor(true)}
            className="btn btn-primary inline-flex"
          >
            <Plus weight="bold" className="w-4 h-4" aria-hidden="true" />
            Ersten Parkplatz erstellen
          </button>
        </motion.div>
      )}

      <div className="space-y-4">
        {lots.map((lot) => (
          <div key={lot.id}>
            <div className="card-hover p-6">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-4">
                  <div className="w-12 h-12 bg-gray-100 dark:bg-gray-800 rounded-xl flex items-center justify-center">
                    <Buildings weight="fill" className="w-6 h-6 text-gray-500" />
                  </div>
                  <div>
                    <p className="font-semibold text-gray-900 dark:text-white">{lot.name}</p>
                    <p className="text-sm text-gray-500 dark:text-gray-400">{lot.address}</p>
                  </div>
                </div>
                <div className="flex items-center gap-4">
                  <div className="text-right">
                    <p className="font-bold text-gray-900 dark:text-white">
                      {lot.available_slots}/{lot.total_slots}
                    </p>
                    <p className="text-xs text-gray-500">verfügbar</p>
                  </div>
                  <button
                    onClick={() => handleEdit(lot)}
                    className={`btn btn-sm ${editingLotId === lot.id ? 'btn-primary' : 'btn-secondary'}`}
                  >
                    Bearbeiten
                    <CaretRight weight="bold" className={`w-4 h-4 transition-transform ${editingLotId === lot.id ? 'rotate-90' : ''}`} />
                  </button>
                </div>
              </div>
            </div>

            {/* Inline editor */}
            <AnimatePresence>
              {editingLotId === lot.id && editingLayout && (
                <motion.div
                  initial={{ opacity: 0, height: 0 }}
                  animate={{ opacity: 1, height: 'auto' }}
                  exit={{ opacity: 0, height: 0 }}
                  className="overflow-hidden"
                >
                  <div className="card p-6 mt-2 border-l-4 border-l-primary-500">
                    <LotLayoutEditor
                      initialLayout={editingLayout.layout}
                      lotName={editingLayout.name}
                      onSave={(_layout, _name) => {
                        // Layout saved — note: the backend does not yet have a
                        // dedicated PUT /api/v1/lots/:id endpoint. The layout
                        // editor currently serves as a preview/planning tool.
                        // Dismiss the editor and show a confirmation message.
                        toast.success('Layout gespeichert (Vorschau)');
                        setEditingLotId(null);
                      }}
                      onCancel={() => setEditingLotId(null)}
                    />
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        ))}
      </div>
    </motion.div>
  );
}

function AdminUsers() {
  const [users, setUsers] = useState<AdminUser[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [page, setPage] = useState(1);
  const [editingUser, setEditingUser] = useState<AdminUser | null>(null);
  const [editRole, setEditRole] = useState<AdminUser['role']>('user');
  const [savingRole, setSavingRole] = useState(false);
  const [deletingUserId, setDeletingUserId] = useState<string | null>(null);
  const [confirmDeleteUser, setConfirmDeleteUser] = useState<AdminUser | null>(null);
  const PAGE_SIZE = 10;

  useEffect(() => {
    loadUsers();
  }, []);

  async function loadUsers() {
    setLoading(true);
    try {
      const res = await api.adminGetUsers();
      if (res.success && res.data) {
        setUsers(res.data);
      } else {
        toast.error('Benutzer konnten nicht geladen werden');
      }
    } finally {
      setLoading(false);
    }
  }

  async function handleSaveRole() {
    if (!editingUser) return;
    setSavingRole(true);
    try {
      const res = await api.adminUpdateUserRole(editingUser.id, editRole);
      if (res.success) {
        setUsers(users.map(u => u.id === editingUser.id ? { ...u, role: editRole } : u));
        toast.success('Rolle aktualisiert');
        setEditingUser(null);
      } else {
        toast.error(res.error?.message ?? 'Fehler beim Speichern');
      }
    } finally {
      setSavingRole(false);
    }
  }

  async function handleToggleStatus(user: AdminUser) {
    const newStatus = user.status === 'active' ? 'disabled' : 'active';
    const res = await api.adminUpdateUserStatus(user.id, newStatus);
    if (res.success) {
      setUsers(users.map(u => u.id === user.id ? { ...u, status: newStatus } : u));
      toast.success(newStatus === 'active' ? 'Benutzer aktiviert' : 'Benutzer deaktiviert');
    } else {
      toast.error(res.error?.message ?? 'Fehler');
    }
  }

  async function handleDeleteUser(user: AdminUser) {
    setDeletingUserId(user.id);
    try {
      const res = await api.adminDeleteUser(user.id);
      if (res.success) {
        setUsers(users.filter(u => u.id !== user.id));
        toast.success('Benutzer gelöscht');
        setConfirmDeleteUser(null);
      } else {
        toast.error(res.error?.message ?? 'Fehler beim Löschen');
      }
    } finally {
      setDeletingUserId(null);
    }
  }

  const filtered = users.filter(u =>
    search === '' ||
    u.name.toLowerCase().includes(search.toLowerCase()) ||
    u.email.toLowerCase().includes(search.toLowerCase()) ||
    u.username.toLowerCase().includes(search.toLowerCase())
  );
  const totalPages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const paginated = filtered.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE);

  function roleBadge(role: AdminUser['role']) {
    if (role === 'superadmin') return <span className="badge badge-error">Super-Admin</span>;
    if (role === 'admin') return <span className="badge badge-warning">Admin</span>;
    return <span className="badge badge-info">Benutzer</span>;
  }

  function statusBadge(status: AdminUser['status']) {
    if (status === 'active') return <span className="badge badge-success"><CheckCircle weight="fill" className="w-3 h-3" />Aktiv</span>;
    return <span className="badge badge-gray"><ProhibitInset weight="fill" className="w-3 h-3" />Deaktiviert</span>;
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" />
      </div>
    );
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="space-y-6"
    >
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
          Benutzer verwalten
          <span className="ml-2 badge badge-info">{users.length}</span>
        </h2>
        <button onClick={loadUsers} className="btn btn-secondary">
          <ArrowClockwise weight="bold" className="w-4 h-4" />
          Aktualisieren
        </button>
      </div>

      {/* Search */}
      <div className="relative">
        <MagnifyingGlass weight="bold" className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" aria-hidden="true" />
        <input
          type="search"
          placeholder="Suche nach Name, E-Mail oder Benutzername…"
          value={search}
          onChange={(e) => { setSearch(e.target.value); setPage(1); }}
          className="input pl-10"
          aria-label="Benutzer suchen"
        />
      </div>

      {/* Table */}
      <div className="card overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-100 dark:border-gray-800 bg-gray-50 dark:bg-gray-800/50">
                <th className="text-left px-4 py-3 font-semibold text-gray-700 dark:text-gray-300">Name</th>
                <th className="text-left px-4 py-3 font-semibold text-gray-700 dark:text-gray-300 hidden md:table-cell">E-Mail</th>
                <th className="text-left px-4 py-3 font-semibold text-gray-700 dark:text-gray-300">Rolle</th>
                <th className="text-left px-4 py-3 font-semibold text-gray-700 dark:text-gray-300 hidden lg:table-cell">Erstellt</th>
                <th className="text-left px-4 py-3 font-semibold text-gray-700 dark:text-gray-300">Status</th>
                <th className="text-right px-4 py-3 font-semibold text-gray-700 dark:text-gray-300">Aktionen</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100 dark:divide-gray-800">
              {paginated.length === 0 ? (
                <tr>
                  <td colSpan={6} className="px-4 py-12 text-center text-gray-500 dark:text-gray-400">
                    {search ? 'Keine Benutzer gefunden' : 'Keine Benutzer vorhanden'}
                  </td>
                </tr>
              ) : (
                paginated.map((user) => (
                  <tr key={user.id} className="hover:bg-gray-50 dark:hover:bg-gray-800/30 transition-colors">
                    <td className="px-4 py-3">
                      <div className="flex items-center gap-3">
                        <div className="w-8 h-8 rounded-full bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center shrink-0">
                          <span className="text-xs font-bold text-primary-600 dark:text-primary-400">
                            {user.name.charAt(0).toUpperCase()}
                          </span>
                        </div>
                        <div>
                          <p className="font-medium text-gray-900 dark:text-white">{user.name}</p>
                          <p className="text-xs text-gray-500 dark:text-gray-400 md:hidden">{user.email}</p>
                        </div>
                      </div>
                    </td>
                    <td className="px-4 py-3 text-gray-600 dark:text-gray-300 hidden md:table-cell">
                      {user.email}
                    </td>
                    <td className="px-4 py-3">
                      {roleBadge(user.role)}
                    </td>
                    <td className="px-4 py-3 text-gray-500 dark:text-gray-400 hidden lg:table-cell">
                      {format(new Date(user.created_at), 'd. MMM yyyy', { locale: de })}
                    </td>
                    <td className="px-4 py-3">
                      {statusBadge(user.status)}
                    </td>
                    <td className="px-4 py-3">
                      <div className="flex items-center justify-end gap-1">
                        {/* Edit Role */}
                        <button
                          onClick={() => { setEditingUser(user); setEditRole(user.role); }}
                          className="btn btn-ghost btn-icon btn-sm"
                          aria-label={`Rolle von ${user.name} bearbeiten`}
                          title="Rolle bearbeiten"
                        >
                          <PencilSimple weight="regular" className="w-4 h-4" />
                        </button>
                        {/* Toggle Status */}
                        <button
                          onClick={() => handleToggleStatus(user)}
                          className="btn btn-ghost btn-icon btn-sm"
                          aria-label={user.status === 'active' ? `${user.name} deaktivieren` : `${user.name} aktivieren`}
                          title={user.status === 'active' ? 'Deaktivieren' : 'Aktivieren'}
                        >
                          {user.status === 'active' ? (
                            <ProhibitInset weight="regular" className="w-4 h-4 text-amber-500" />
                          ) : (
                            <CheckFat weight="regular" className="w-4 h-4 text-emerald-500" />
                          )}
                        </button>
                        {/* Delete */}
                        <button
                          onClick={() => setConfirmDeleteUser(user)}
                          className="btn btn-ghost btn-icon btn-sm text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20"
                          aria-label={`${user.name} löschen`}
                          title="Löschen"
                        >
                          <Trash weight="regular" className="w-4 h-4" />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>

        {/* Pagination */}
        {totalPages > 1 && (
          <div className="px-4 py-3 border-t border-gray-100 dark:border-gray-800 flex items-center justify-between">
            <p className="text-sm text-gray-500 dark:text-gray-400">
              {(page - 1) * PAGE_SIZE + 1}–{Math.min(page * PAGE_SIZE, filtered.length)} von {filtered.length}
            </p>
            <div className="flex items-center gap-1">
              <button
                onClick={() => setPage(p => Math.max(1, p - 1))}
                disabled={page === 1}
                className="btn btn-ghost btn-icon btn-sm"
                aria-label="Vorherige Seite"
              >
                <CaretLeft weight="bold" className="w-4 h-4" />
              </button>
              <span className="text-sm text-gray-700 dark:text-gray-300 px-2">
                {page} / {totalPages}
              </span>
              <button
                onClick={() => setPage(p => Math.min(totalPages, p + 1))}
                disabled={page === totalPages}
                className="btn btn-ghost btn-icon btn-sm"
                aria-label="Nächste Seite"
              >
                <CaretRight weight="bold" className="w-4 h-4" />
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Edit Role Modal */}
      <AnimatePresence>
        {editingUser && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 flex items-center justify-center p-4"
          >
            <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={() => setEditingUser(null)} aria-hidden="true" />
            <motion.div
              initial={{ scale: 0.95, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 0.95, opacity: 0 }}
              role="dialog"
              aria-modal="true"
              aria-label="Rolle bearbeiten"
              className="relative card p-6 w-full max-w-sm shadow-2xl"
            >
              <button onClick={() => setEditingUser(null)} className="absolute top-4 right-4 btn btn-ghost btn-icon" aria-label="Schließen">
                <X weight="bold" className="w-4 h-4" />
              </button>
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                Rolle bearbeiten
              </h3>
              <p className="text-sm text-gray-600 dark:text-gray-300 mb-4">
                Benutzer: <span className="font-medium text-gray-900 dark:text-white">{editingUser.name}</span>
              </p>
              <div className="mb-6">
                <label htmlFor="role-select" className="label">Rolle</label>
                <select
                  id="role-select"
                  value={editRole}
                  onChange={(e) => setEditRole(e.target.value as AdminUser['role'])}
                  className="input"
                >
                  <option value="user">Benutzer</option>
                  <option value="admin">Admin</option>
                  <option value="superadmin">Super-Admin</option>
                </select>
              </div>
              <div className="flex gap-3">
                <button onClick={() => setEditingUser(null)} className="btn btn-secondary flex-1">
                  Abbrechen
                </button>
                <button onClick={handleSaveRole} disabled={savingRole} className="btn btn-primary flex-1">
                  {savingRole ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> : null}
                  Speichern
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Delete Confirm Modal */}
      <AnimatePresence>
        {confirmDeleteUser && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 flex items-center justify-center p-4"
          >
            <div className="absolute inset-0 bg-black/50 backdrop-blur-sm" onClick={() => setConfirmDeleteUser(null)} aria-hidden="true" />
            <motion.div
              initial={{ scale: 0.95, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 0.95, opacity: 0 }}
              role="dialog"
              aria-modal="true"
              aria-label="Benutzer löschen bestätigen"
              className="relative card p-6 w-full max-w-sm shadow-2xl"
            >
              <div className="flex items-center gap-3 mb-4">
                <div className="w-10 h-10 bg-red-100 dark:bg-red-900/30 rounded-xl flex items-center justify-center">
                  <Warning weight="fill" className="w-5 h-5 text-red-600 dark:text-red-400" />
                </div>
                <h3 className="text-lg font-semibold text-gray-900 dark:text-white">
                  Benutzer löschen?
                </h3>
              </div>
              <p className="text-sm text-gray-600 dark:text-gray-300 mb-6">
                Möchten Sie{' '}
                <span className="font-semibold text-gray-900 dark:text-white">{confirmDeleteUser.name}</span>{' '}
                wirklich löschen? Diese Aktion kann nicht rückgängig gemacht werden.
              </p>
              <div className="flex gap-3">
                <button
                  onClick={() => setConfirmDeleteUser(null)}
                  disabled={deletingUserId === confirmDeleteUser.id}
                  className="btn btn-secondary flex-1"
                >
                  Abbrechen
                </button>
                <button
                  onClick={() => handleDeleteUser(confirmDeleteUser)}
                  disabled={deletingUserId === confirmDeleteUser.id}
                  className="btn btn-danger flex-1"
                >
                  {deletingUserId === confirmDeleteUser.id ? (
                    <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" />
                  ) : (
                    <Trash weight="bold" className="w-4 h-4" />
                  )}
                  Löschen
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}

function AdminBookings() {
  const [bookings, setBookings] = useState<AdminBooking[]>([]);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(1);
  const [filterStatus, setFilterStatus] = useState<string>('');
  const [filterLot, setFilterLot] = useState<string>('');
  const [filterDateFrom, setFilterDateFrom] = useState<string>('');
  const [filterDateTo, setFilterDateTo] = useState<string>('');
  const PAGE_SIZE = 15;

  useEffect(() => {
    loadBookings();
  }, []);

  async function loadBookings() {
    setLoading(true);
    try {
      const res = await api.adminGetBookings();
      if (res.success && res.data) {
        setBookings(res.data);
      } else {
        toast.error('Buchungen konnten nicht geladen werden');
      }
    } finally {
      setLoading(false);
    }
  }

  function exportCsv() {
    const headers = ['ID', 'Benutzer', 'Parkplatz', 'Stellplatz', 'Start', 'Ende', 'Status', 'Typ'];
    const rows = filtered.map(b => [
      b.id,
      b.user_name,
      b.lot_name,
      b.slot_number,
      b.start_time,
      b.end_time,
      b.status,
      b.booking_type ?? 'one-time',
    ]);
    const csv = [headers, ...rows]
      .map(row => row.map(cell => `"${String(cell).replace(/"/g, '""')}"`).join(','))
      .join('\n');
    const blob = new Blob(['\uFEFF' + csv], { type: 'text/csv;charset=utf-8;' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `parkhub-buchungen-${new Date().toISOString().slice(0, 10)}.csv`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
    toast.success('CSV exportiert');
  }

  const lots = Array.from(new Set(bookings.map(b => b.lot_name))).sort();

  const filtered = bookings.filter(b => {
    if (filterStatus && b.status !== filterStatus) return false;
    if (filterLot && b.lot_name !== filterLot) return false;
    if (filterDateFrom && b.start_time < filterDateFrom) return false;
    if (filterDateTo && b.start_time > filterDateTo + 'T23:59:59') return false;
    return true;
  });

  const totalPages = Math.max(1, Math.ceil(filtered.length / PAGE_SIZE));
  const paginated = filtered.slice((page - 1) * PAGE_SIZE, page * PAGE_SIZE);

  function statusBadge(status: AdminBooking['status']) {
    if (status === 'active') return <span className="badge badge-info">Aktiv</span>;
    if (status === 'completed') return <span className="badge badge-success">Abgeschlossen</span>;
    return <span className="badge badge-gray">Storniert</span>;
  }

  function typeBadge(type?: string) {
    if (type === 'recurring') return <span className="badge badge-warning">Wiederkehrend</span>;
    if (type === 'guest') return <span className="badge badge-gray">Gast</span>;
    return <span className="badge badge-info">Einmalig</span>;
  }

  function resetFilters() {
    setFilterStatus('');
    setFilterLot('');
    setFilterDateFrom('');
    setFilterDateTo('');
    setPage(1);
  }

  const hasFilters = filterStatus || filterLot || filterDateFrom || filterDateTo;

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" />
      </div>
    );
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="space-y-6"
    >
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
        <div>
          <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
            Alle Buchungen
            <span className="ml-2 badge badge-info">{filtered.length}</span>
          </h2>
          {hasFilters && (
            <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
              {filtered.length} von {bookings.length} Buchungen (gefiltert)
            </p>
          )}
        </div>
        <div className="flex gap-2 flex-wrap">
          <button onClick={loadBookings} className="btn btn-secondary btn-sm">
            <ArrowClockwise weight="bold" className="w-4 h-4" />
            Aktualisieren
          </button>
          <button onClick={exportCsv} disabled={filtered.length === 0} className="btn btn-secondary btn-sm">
            <DownloadSimple weight="bold" className="w-4 h-4" />
            CSV exportieren
          </button>
        </div>
      </div>

      {/* Filters */}
      <div className="card p-4">
        <div className="flex items-center gap-2 mb-3">
          <Funnel weight="fill" className="w-4 h-4 text-gray-400" />
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">Filter</span>
          {hasFilters && (
            <button
              onClick={resetFilters}
              className="ml-auto text-xs text-primary-600 dark:text-primary-400 hover:underline flex items-center gap-1"
            >
              <X weight="bold" className="w-3 h-3" />
              Filter zurücksetzen
            </button>
          )}
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
          <div>
            <label htmlFor="filter-status" className="label text-xs">Status</label>
            <select
              id="filter-status"
              value={filterStatus}
              onChange={(e) => { setFilterStatus(e.target.value); setPage(1); }}
              className="input text-sm py-2"
            >
              <option value="">Alle Status</option>
              <option value="active">Aktiv</option>
              <option value="completed">Abgeschlossen</option>
              <option value="cancelled">Storniert</option>
            </select>
          </div>
          <div>
            <label htmlFor="filter-lot" className="label text-xs">Parkplatz</label>
            <select
              id="filter-lot"
              value={filterLot}
              onChange={(e) => { setFilterLot(e.target.value); setPage(1); }}
              className="input text-sm py-2"
            >
              <option value="">Alle Parkplätze</option>
              {lots.map(lot => (
                <option key={lot} value={lot}>{lot}</option>
              ))}
            </select>
          </div>
          <div>
            <label htmlFor="filter-date-from" className="label text-xs">Von Datum</label>
            <input
              id="filter-date-from"
              type="date"
              value={filterDateFrom}
              onChange={(e) => { setFilterDateFrom(e.target.value); setPage(1); }}
              className="input text-sm py-2"
            />
          </div>
          <div>
            <label htmlFor="filter-date-to" className="label text-xs">Bis Datum</label>
            <input
              id="filter-date-to"
              type="date"
              value={filterDateTo}
              onChange={(e) => { setFilterDateTo(e.target.value); setPage(1); }}
              className="input text-sm py-2"
            />
          </div>
        </div>
      </div>

      {/* Table */}
      <div className="card overflow-hidden">
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-gray-100 dark:border-gray-800 bg-gray-50 dark:bg-gray-800/50">
                <th className="text-left px-4 py-3 font-semibold text-gray-700 dark:text-gray-300">Benutzer</th>
                <th className="text-left px-4 py-3 font-semibold text-gray-700 dark:text-gray-300">Parkplatz</th>
                <th className="text-left px-4 py-3 font-semibold text-gray-700 dark:text-gray-300">Platz</th>
                <th className="text-left px-4 py-3 font-semibold text-gray-700 dark:text-gray-300 hidden md:table-cell">Start</th>
                <th className="text-left px-4 py-3 font-semibold text-gray-700 dark:text-gray-300 hidden lg:table-cell">Ende</th>
                <th className="text-left px-4 py-3 font-semibold text-gray-700 dark:text-gray-300">Status</th>
                <th className="text-left px-4 py-3 font-semibold text-gray-700 dark:text-gray-300 hidden xl:table-cell">Typ</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100 dark:divide-gray-800">
              {paginated.length === 0 ? (
                <tr>
                  <td colSpan={7} className="px-4 py-12 text-center text-gray-500 dark:text-gray-400">
                    {hasFilters ? 'Keine Buchungen für diese Filter' : 'Keine Buchungen vorhanden'}
                  </td>
                </tr>
              ) : (
                paginated.map((booking) => (
                  <tr key={booking.id} className="hover:bg-gray-50 dark:hover:bg-gray-800/30 transition-colors">
                    <td className="px-4 py-3">
                      <div>
                        <p className="font-medium text-gray-900 dark:text-white">{booking.user_name}</p>
                        <p className="text-xs text-gray-500 dark:text-gray-400">{booking.user_email}</p>
                      </div>
                    </td>
                    <td className="px-4 py-3 text-gray-700 dark:text-gray-300">{booking.lot_name}</td>
                    <td className="px-4 py-3">
                      <span className="inline-flex items-center justify-center w-8 h-8 bg-gray-100 dark:bg-gray-800 rounded-lg font-bold text-gray-700 dark:text-gray-300 text-xs">
                        {booking.slot_number}
                      </span>
                    </td>
                    <td className="px-4 py-3 text-gray-600 dark:text-gray-300 hidden md:table-cell">
                      {format(new Date(booking.start_time), 'd. MMM yy, HH:mm', { locale: de })}
                    </td>
                    <td className="px-4 py-3 text-gray-600 dark:text-gray-300 hidden lg:table-cell">
                      {format(new Date(booking.end_time), 'd. MMM yy, HH:mm', { locale: de })}
                    </td>
                    <td className="px-4 py-3">{statusBadge(booking.status)}</td>
                    <td className="px-4 py-3 hidden xl:table-cell">{typeBadge(booking.booking_type)}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>

        {/* Pagination */}
        {totalPages > 1 && (
          <div className="px-4 py-3 border-t border-gray-100 dark:border-gray-800 flex items-center justify-between">
            <p className="text-sm text-gray-500 dark:text-gray-400">
              {(page - 1) * PAGE_SIZE + 1}–{Math.min(page * PAGE_SIZE, filtered.length)} von {filtered.length}
            </p>
            <div className="flex items-center gap-1">
              <button
                onClick={() => setPage(p => Math.max(1, p - 1))}
                disabled={page === 1}
                className="btn btn-ghost btn-icon btn-sm"
                aria-label="Vorherige Seite"
              >
                <CaretLeft weight="bold" className="w-4 h-4" />
              </button>
              <span className="text-sm text-gray-700 dark:text-gray-300 px-2">
                {page} / {totalPages}
              </span>
              <button
                onClick={() => setPage(p => Math.min(totalPages, p + 1))}
                disabled={page === totalPages}
                className="btn btn-ghost btn-icon btn-sm"
                aria-label="Nächste Seite"
              >
                <CaretRight weight="bold" className="w-4 h-4" />
              </button>
            </div>
          </div>
        )}
      </div>
    </motion.div>
  );
}

export function AdminPage() {
  return (
    <div>
      <div className="mb-2">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Administration
        </h1>
        <p className="text-gray-500 dark:text-gray-400 mt-1">
          System- und Benutzerverwaltung
        </p>
      </div>

      <AdminNav />

      <Routes>
        <Route path="/" element={<AdminOverview />} />
        <Route path="/lots" element={<AdminLots />} />
        <Route path="/users" element={<AdminUsers />} />
        <Route path="/bookings" element={<AdminBookings />} />
      </Routes>
    </div>
  );
}
