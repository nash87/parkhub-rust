import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Megaphone, Plus, PencilSimple, Trash, SpinnerGap, Check, X,
  Info, Warning, WarningCircle, CheckCircle, Clock,
} from '@phosphor-icons/react';
import { api, type Announcement } from '../api/client';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';

type Severity = 'info' | 'warning' | 'error' | 'success';

interface AnnouncementForm {
  title: string;
  message: string;
  severity: Severity;
  active: boolean;
  expires_at: string;
}

const emptyForm: AnnouncementForm = {
  title: '',
  message: '',
  severity: 'info',
  active: true,
  expires_at: '',
};

const severityIcons: Record<Severity, { color: string; bg: string; icon: typeof Info }> = {
  info:    { color: 'text-blue-600 dark:text-blue-400',   bg: 'bg-blue-100 dark:bg-blue-900/30',   icon: Info },
  warning: { color: 'text-amber-600 dark:text-amber-400', bg: 'bg-amber-100 dark:bg-amber-900/30', icon: Warning },
  error:   { color: 'text-red-600 dark:text-red-400',     bg: 'bg-red-100 dark:bg-red-900/30',     icon: WarningCircle },
  success: { color: 'text-green-600 dark:text-green-400', bg: 'bg-green-100 dark:bg-green-900/30', icon: CheckCircle },
};

const severityLabelKeys: Record<Severity, string> = {
  info: 'admin.severityInfo',
  warning: 'admin.severityWarning',
  error: 'admin.severityError',
  success: 'admin.severitySuccess',
};

function SeverityBadge({ severity, t }: { severity: Severity; t: (key: string) => string }) {
  const cfg = severityIcons[severity] || severityIcons.info;
  const Icon = cfg.icon;
  return (
    <span className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${cfg.bg} ${cfg.color}`}>
      <Icon weight="fill" className="w-3.5 h-3.5" />
      {t(severityLabelKeys[severity])}
    </span>
  );
}

function StatusBadge({ active, expiresAt, t }: { active: boolean; expiresAt?: string; t: (key: string) => string }) {
  const isExpired = expiresAt && new Date(expiresAt) < new Date();
  if (!active || isExpired) {
    return (
      <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-surface-100 dark:bg-surface-800 text-surface-500 dark:text-surface-400">
        <Clock weight="fill" className="w-3.5 h-3.5" />
        {isExpired ? t('admin.expired') : t('admin.inactive')}
      </span>
    );
  }
  return (
    <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400">
      <CheckCircle weight="fill" className="w-3.5 h-3.5" />
      {t('admin.active')}
    </span>
  );
}

export function AdminAnnouncementsPage() {
  const { t } = useTranslation();
  const [announcements, setAnnouncements] = useState<Announcement[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [form, setForm] = useState<AnnouncementForm>({ ...emptyForm });
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [confirmState, setConfirmState] = useState<{open: boolean, action: () => void}>({open: false, action: () => {}});

  useEffect(() => { load(); }, []);

  async function load() {
    try {
      const res = await api.adminListAnnouncements();
      if (res.success && res.data) setAnnouncements(res.data);
    } finally {
      setLoading(false);
    }
  }

  function openCreate() {
    setEditingId(null);
    setForm({ ...emptyForm });
    setShowForm(true);
  }

  function openEdit(a: Announcement) {
    setEditingId(a.id);
    setForm({
      title: a.title,
      message: a.message,
      severity: a.severity as Severity,
      active: a.active,
      expires_at: a.expires_at ? a.expires_at.slice(0, 16) : '',
    });
    setShowForm(true);
  }

  function closeForm() {
    setShowForm(false);
    setEditingId(null);
    setForm({ ...emptyForm });
  }

  async function handleSave() {
    if (!form.title.trim() || !form.message.trim()) {
      toast.error(t('admin.announcementTitleRequired'));
      return;
    }
    setSaving(true);
    try {
      const payload = {
        title: form.title.trim(),
        message: form.message.trim(),
        severity: form.severity,
        active: form.active,
        expires_at: form.expires_at || undefined,
      };
      const res = editingId
        ? await api.adminUpdateAnnouncement(editingId, payload)
        : await api.adminCreateAnnouncement(payload);
      if (res.success) {
        toast.success(editingId ? t('admin.announcementUpdated') : t('admin.announcementCreated'));
        closeForm();
        await load();
      } else {
        toast.error(res.error?.message || t('admin.announcementSaveFailed'));
      }
    } finally {
      setSaving(false);
    }
  }

  function handleDelete(id: string) {
    setConfirmState({
      open: true,
      action: async () => {
        setConfirmState({open: false, action: () => {}});
        setDeletingId(id);
        try {
          const res = await api.adminDeleteAnnouncement(id);
          if (res.success) {
            setAnnouncements(prev => prev.filter(a => a.id !== id));
            toast.success(t('admin.announcementDeleted'));
            if (editingId === id) closeForm();
          } else {
            toast.error(res.error?.message || t('admin.announcementDeleteFailed'));
          }
        } finally {
          setDeletingId(null);
        }
      },
    });
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64" role="status" aria-label={t('common.loading')}>
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" aria-hidden="true" />
      </div>
    );
  }

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <Megaphone weight="fill" className="w-6 h-6 text-primary-600" />
          <h2 className="text-xl font-semibold text-surface-900 dark:text-white">{t('admin.announcements')}</h2>
        </div>
        <button onClick={openCreate} className="btn btn-primary self-start sm:self-auto">
          <Plus weight="bold" className="w-4 h-4" />
          {t('admin.newAnnouncement')}
        </button>
      </div>

      {/* Form (Create / Edit) */}
      <AnimatePresence>
        {showForm && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="overflow-hidden"
          >
            <div className="card p-6 space-y-5">
              <div className="flex items-center justify-between">
                <h3 className="text-lg font-semibold text-surface-900 dark:text-white">
                  {editingId ? t('admin.editAnnouncement') : t('admin.newAnnouncement')}
                </h3>
                <button onClick={closeForm} className="p-1.5 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors" aria-label={t('common.close')}>
                  <X weight="bold" className="w-5 h-5 text-surface-400" aria-hidden="true" />
                </button>
              </div>

              {/* Title */}
              <div>
                <label htmlFor="announcement-title" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">{t('admin.announcementTitle')}</label>
                <input
                  id="announcement-title"
                  type="text"
                  value={form.title}
                  onChange={e => setForm(prev => ({ ...prev, title: e.target.value }))}
                  className="input"
                  placeholder={t('admin.announcementTitle')}
                />
              </div>

              {/* Message */}
              <div>
                <label htmlFor="announcement-message" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">{t('admin.announcementMessage')}</label>
                <textarea
                  id="announcement-message"
                  value={form.message}
                  onChange={e => setForm(prev => ({ ...prev, message: e.target.value }))}
                  className="input h-28 resize-y"
                  placeholder={t('admin.announcementMessage')}
                />
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-3 gap-5">
                {/* Severity */}
                <div>
                  <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">{t('admin.severity')}</label>
                  <div className="flex flex-wrap gap-2">
                    {(Object.keys(severityIcons) as Severity[]).map(sev => {
                      const cfg = severityIcons[sev];
                      const Icon = cfg.icon;
                      const isSelected = form.severity === sev;
                      return (
                        <button
                          key={sev}
                          type="button"
                          onClick={() => setForm(prev => ({ ...prev, severity: sev }))}
                          className={`flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-sm font-medium border-2 transition-all ${
                            isSelected
                              ? `${cfg.bg} ${cfg.color} border-current`
                              : 'border-surface-200 dark:border-surface-700 text-surface-500 dark:text-surface-400 hover:border-surface-300 dark:hover:border-surface-600'
                          }`}
                        >
                          <Icon weight={isSelected ? 'fill' : 'regular'} className="w-4 h-4" />
                          {t(severityLabelKeys[sev])}
                        </button>
                      );
                    })}
                  </div>
                </div>

                {/* Active toggle */}
                <div>
                  <label className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">{t('admin.status')}</label>
                  <button
                    type="button"
                    onClick={() => setForm(prev => ({ ...prev, active: !prev.active }))}
                    className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium border-2 transition-all ${
                      form.active
                        ? 'bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400 border-green-300 dark:border-green-700'
                        : 'bg-surface-100 dark:bg-surface-800 text-surface-500 dark:text-surface-400 border-surface-200 dark:border-surface-700'
                    }`}
                  >
                    {form.active ? (
                      <><CheckCircle weight="fill" className="w-4 h-4" />{t('admin.active')}</>
                    ) : (
                      <><Clock weight="fill" className="w-4 h-4" />{t('admin.inactive')}</>
                    )}
                  </button>
                </div>

                {/* Expires at */}
                <div>
                  <label htmlFor="announcement-expires" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-2">{t('admin.expiresAt')}</label>
                  <input
                    id="announcement-expires"
                    type="datetime-local"
                    value={form.expires_at}
                    onChange={e => setForm(prev => ({ ...prev, expires_at: e.target.value }))}
                    className="input"
                  />
                </div>
              </div>

              {/* Actions */}
              <div className="flex gap-3 pt-2">
                <button onClick={handleSave} disabled={saving} className="btn btn-primary">
                  {saving
                    ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" />
                    : <Check weight="bold" className="w-4 h-4" />}
                  {editingId ? t('common.save') : t('admin.create')}
                </button>
                <button onClick={closeForm} className="btn btn-secondary">{t('common.cancel')}</button>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Announcements List */}
      {announcements.length === 0 && !showForm ? (
        <div className="p-12 text-center">
          <Megaphone weight="light" className="w-16 h-16 text-surface-200 dark:text-surface-700 mx-auto mb-4" />
          <p className="text-surface-500 dark:text-surface-400">{t('admin.noAnnouncements')}</p>
        </div>
      ) : (
        <div className="space-y-3">
          {announcements.map((a, i) => (
            <motion.div
              key={a.id}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.04 }}
              className="card hover:shadow-md transition-shadow p-5"
            >
              <div className="flex items-start justify-between gap-4">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-3 mb-2 flex-wrap">
                    <h3 className="text-base font-semibold text-surface-900 dark:text-white truncate">
                      {a.title}
                    </h3>
                    <SeverityBadge severity={a.severity as Severity} t={t} />
                    <StatusBadge active={a.active} expiresAt={a.expires_at} t={t} />
                  </div>
                  <p className="text-sm text-surface-600 dark:text-surface-400 line-clamp-2 mb-2">
                    {a.message}
                  </p>
                  <div className="flex items-center gap-4 text-xs text-surface-500 dark:text-surface-400">
                    <span>{t('admin.announcementCreatedAt')} {new Date(a.created_at).toLocaleDateString()}</span>
                    {a.expires_at && <span>{t('admin.announcementExpiresAt')} {new Date(a.expires_at).toLocaleDateString()}</span>}
                  </div>
                </div>

                <div className="flex items-center gap-1.5 shrink-0">
                  <button
                    onClick={() => openEdit(a)}
                    className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors text-surface-400 hover:text-primary-600"
                    aria-label={`${t('common.edit')} ${a.title}`}
                  >
                    <PencilSimple weight="bold" className="w-4.5 h-4.5" aria-hidden="true" />
                  </button>
                  <button
                    onClick={() => handleDelete(a.id)}
                    disabled={deletingId === a.id}
                    className="p-2 rounded-lg hover:bg-red-50 dark:hover:bg-red-900/20 transition-colors text-surface-400 hover:text-red-600 disabled:opacity-50"
                    aria-label={`${t('common.delete')} ${a.title}`}
                  >
                    {deletingId === a.id
                      ? <SpinnerGap weight="bold" className="w-4.5 h-4.5 animate-spin" />
                      : <Trash weight="bold" className="w-4.5 h-4.5" aria-hidden="true" />}
                  </button>
                </div>
              </div>
            </motion.div>
          ))}
        </div>
      )}
      <ConfirmDialog
        open={confirmState.open}
        title={t('common.delete')}
        message={t('admin.announcementDeleteConfirm')}
        variant="danger"
        onConfirm={confirmState.action}
        onCancel={() => setConfirmState({open: false, action: () => {}})}
      />
    </motion.div>
  );
}
