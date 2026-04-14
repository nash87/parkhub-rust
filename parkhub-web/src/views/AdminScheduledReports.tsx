import { useEffect, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Clock, Plus, Trash, PaperPlaneTilt, Question, Pencil, ToggleLeft, ToggleRight } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

interface ReportSchedule {
  id: string;
  name: string;
  report_type: 'occupancy_summary' | 'revenue_report' | 'user_activity' | 'booking_trends';
  frequency: 'daily' | 'weekly' | 'monthly';
  recipients: string[];
  enabled: boolean;
  last_sent_at: string | null;
  next_run_at: string;
  created_at: string;
  updated_at: string;
}

const reportTypeLabels: Record<string, string> = {
  occupancy_summary: 'Occupancy Summary',
  revenue_report: 'Revenue Report',
  user_activity: 'User Activity',
  booking_trends: 'Booking Trends',
};

const frequencyLabels: Record<string, string> = {
  daily: 'Daily',
  weekly: 'Weekly',
  monthly: 'Monthly',
};

const frequencyCron: Record<string, string> = {
  daily: '0 8 * * *',
  weekly: '0 8 * * MON',
  monthly: '0 8 1 * *',
};

export function AdminScheduledReportsPage() {
  const { t } = useTranslation();
  const [schedules, setSchedules] = useState<ReportSchedule[]>([]);
  const [loading, setLoading] = useState(true);
  const [showHelp, setShowHelp] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [editId, setEditId] = useState<string | null>(null);
  const [formName, setFormName] = useState('');
  const [formType, setFormType] = useState<string>('occupancy_summary');
  const [formFrequency, setFormFrequency] = useState<string>('daily');
  const [formRecipients, setFormRecipients] = useState('');

  const loadSchedules = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch('/api/v1/admin/reports/schedules').then(r => r.json());
      if (res.success) {
        setSchedules(res.data.schedules);
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => { loadSchedules(); }, [loadSchedules]);

  const resetForm = () => {
    setFormName('');
    setFormType('occupancy_summary');
    setFormFrequency('daily');
    setFormRecipients('');
    setEditId(null);
    setShowForm(false);
  };

  const handleSave = async () => {
    if (!formName.trim()) {
      toast.error(t('scheduledReports.nameRequired'));
      return;
    }
    const recipients = formRecipients.split(',').map(e => e.trim()).filter(Boolean);
    if (recipients.length === 0) {
      toast.error(t('scheduledReports.recipientsRequired'));
      return;
    }

    try {
      const method = editId ? 'PUT' : 'POST';
      const url = editId
        ? `/api/v1/admin/reports/schedules/${editId}`
        : '/api/v1/admin/reports/schedules';
      const res = await fetch(url, {
        method,
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          name: formName,
          report_type: formType,
          frequency: formFrequency,
          recipients,
        }),
      }).then(r => r.json());
      if (res.success) {
        toast.success(editId ? t('scheduledReports.updated') : t('scheduledReports.created'));
        resetForm();
        loadSchedules();
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await fetch(`/api/v1/admin/reports/schedules/${id}`, { method: 'DELETE' });
      toast.success(t('scheduledReports.deleted'));
      setSchedules(prev => prev.filter(s => s.id !== id));
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
  };

  const handleSendNow = async (id: string) => {
    try {
      const res = await fetch(`/api/v1/admin/reports/schedules/${id}/send-now`, {
        method: 'POST',
      }).then(r => r.json());
      if (res.success) {
        toast.success(t('scheduledReports.sentNow'));
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
  };

  const startEdit = (schedule: ReportSchedule) => {
    setEditId(schedule.id);
    setFormName(schedule.name);
    setFormType(schedule.report_type);
    setFormFrequency(schedule.frequency);
    setFormRecipients(schedule.recipients.join(', '));
    setShowForm(true);
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500" />
      </div>
    );
  }

  return (
    <div className="space-y-6" data-testid="scheduled-reports-page">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white">
            {t('scheduledReports.title')}
          </h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">
            {t('scheduledReports.subtitle')}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setShowHelp(!showHelp)}
            className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700"
            aria-label={t('scheduledReports.helpLabel')}
            data-testid="reports-help-btn"
          >
            <Question size={20} />
          </button>
          <button
            onClick={() => { resetForm(); setShowForm(true); }}
            className="flex items-center gap-2 px-4 py-2 rounded-lg bg-primary-500 text-white hover:bg-primary-600"
            data-testid="create-schedule-btn"
          >
            <Plus size={16} />
            {t('scheduledReports.create')}
          </button>
        </div>
      </div>

      {/* Help */}
      <AnimatePresence>
        {showHelp && (
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -10 }}
            className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4"
            data-testid="reports-help"
          >
            <p className="text-sm text-blue-700 dark:text-blue-300">
              {t('scheduledReports.help')}
            </p>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Create/Edit Form */}
      <AnimatePresence>
        {showForm && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="bg-white dark:bg-surface-800 rounded-xl p-6 shadow-sm border border-surface-200 dark:border-surface-700 space-y-4"
            data-testid="schedule-form"
          >
            <h2 className="text-lg font-semibold text-surface-900 dark:text-white">
              {editId ? t('scheduledReports.editSchedule') : t('scheduledReports.newSchedule')}
            </h2>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label className="text-sm text-surface-600 dark:text-surface-400">{t('scheduledReports.name')}</label>
                <input
                  value={formName}
                  onChange={e => setFormName(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-700 px-3 py-2 text-sm"
                  data-testid="form-name"
                />
              </div>
              <div>
                <label className="text-sm text-surface-600 dark:text-surface-400">{t('scheduledReports.reportType')}</label>
                <select
                  value={formType}
                  onChange={e => setFormType(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-700 px-3 py-2 text-sm"
                  data-testid="form-type"
                >
                  {Object.entries(reportTypeLabels).map(([k, v]) => (
                    <option key={k} value={k}>{v}</option>
                  ))}
                </select>
              </div>
              <div>
                <label className="text-sm text-surface-600 dark:text-surface-400">{t('scheduledReports.frequency')}</label>
                <select
                  value={formFrequency}
                  onChange={e => setFormFrequency(e.target.value)}
                  className="mt-1 w-full rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-700 px-3 py-2 text-sm"
                  data-testid="form-frequency"
                >
                  {Object.entries(frequencyLabels).map(([k, v]) => (
                    <option key={k} value={k}>{v}</option>
                  ))}
                </select>
              </div>
              <div>
                <label className="text-sm text-surface-600 dark:text-surface-400">{t('scheduledReports.recipients')}</label>
                <input
                  value={formRecipients}
                  onChange={e => setFormRecipients(e.target.value)}
                  placeholder={t('scheduledReports.recipientsPlaceholder')}
                  className="mt-1 w-full rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-700 px-3 py-2 text-sm"
                  data-testid="form-recipients"
                />
              </div>
            </div>
            <div className="flex gap-2">
              <button
                onClick={handleSave}
                className="px-4 py-2 rounded-lg bg-primary-500 text-white hover:bg-primary-600"
                data-testid="form-save-btn"
              >
                {editId ? t('scheduledReports.save') : t('scheduledReports.create')}
              </button>
              <button
                onClick={resetForm}
                className="px-4 py-2 rounded-lg bg-surface-100 dark:bg-surface-700 text-surface-700 dark:text-surface-300"
                data-testid="form-cancel-btn"
              >
                {t('common.cancel')}
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Schedule List */}
      {schedules.length === 0 ? (
        <div className="text-center py-12 text-surface-500" data-testid="schedules-empty">
          {t('scheduledReports.empty')}
        </div>
      ) : (
        <div className="space-y-3" data-testid="schedules-list">
          {schedules.map(schedule => (
            <motion.div
              key={schedule.id}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              className="bg-white dark:bg-surface-800 rounded-xl p-4 shadow-sm border border-surface-200 dark:border-surface-700"
              data-testid="schedule-card"
            >
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <h3 className="font-medium text-surface-900 dark:text-white">{schedule.name}</h3>
                    {schedule.enabled ? (
                      <ToggleRight size={20} weight="fill" className="text-green-500" data-testid="enabled-icon" />
                    ) : (
                      <ToggleLeft size={20} className="text-surface-400" data-testid="disabled-icon" />
                    )}
                  </div>
                  <div className="flex flex-wrap gap-2 mt-1">
                    <span className="text-xs px-2 py-0.5 rounded-full bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-400">
                      {reportTypeLabels[schedule.report_type]}
                    </span>
                    <span className="text-xs px-2 py-0.5 rounded-full bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-400">
                      {frequencyLabels[schedule.frequency]}
                    </span>
                    <span className="text-xs text-surface-500" title={frequencyCron[schedule.frequency]}>
                      <Clock size={12} className="inline mr-0.5" />
                      {frequencyCron[schedule.frequency]}
                    </span>
                  </div>
                  <p className="text-xs text-surface-500 mt-1">
                    {t('scheduledReports.recipientsLabel')}: {schedule.recipients.join(', ')}
                  </p>
                  {schedule.last_sent_at && (
                    <p className="text-xs text-surface-400 mt-0.5">
                      {t('scheduledReports.lastSent')}: {new Date(schedule.last_sent_at).toLocaleString()}
                    </p>
                  )}
                </div>
                <div className="flex items-center gap-1">
                  <button
                    onClick={() => handleSendNow(schedule.id)}
                    className="p-1.5 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700 text-primary-500"
                    title={t('scheduledReports.sendNow')}
                    data-testid="send-now-btn"
                  >
                    <PaperPlaneTilt size={16} />
                  </button>
                  <button
                    onClick={() => startEdit(schedule)}
                    className="p-1.5 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700"
                    title={t('scheduledReports.edit')}
                    data-testid="edit-btn"
                  >
                    <Pencil size={16} />
                  </button>
                  <button
                    onClick={() => handleDelete(schedule.id)}
                    className="p-1.5 rounded-lg hover:bg-red-50 dark:hover:bg-red-900/20 text-red-500"
                    title={t('scheduledReports.delete')}
                    data-testid="delete-btn"
                  >
                    <Trash size={16} />
                  </button>
                </div>
              </div>
            </motion.div>
          ))}
        </div>
      )}
    </div>
  );
}
