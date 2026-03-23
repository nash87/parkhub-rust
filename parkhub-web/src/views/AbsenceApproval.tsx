import { useEffect, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Calendar, Check, X, Clock, Question, PaperPlaneTilt, ChatText,
} from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';
import { api } from '../api/client';

interface AbsenceRequest {
  id: string;
  user_id: string;
  user_name: string;
  absence_type: string;
  start_date: string;
  end_date: string;
  reason: string;
  status: 'pending' | 'approved' | 'rejected';
  reviewer_id?: string;
  reviewer_comment?: string;
  created_at: string;
  reviewed_at?: string;
}

const statusColors: Record<string, string> = {
  pending: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  approved: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
  rejected: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
};

const typeLabels: Record<string, string> = {
  vacation: 'absenceApproval.types.vacation',
  sick: 'absenceApproval.types.sick',
  homeoffice: 'absenceApproval.types.homeoffice',
  business_trip: 'absenceApproval.types.businessTrip',
  personal: 'absenceApproval.types.personal',
  other: 'absenceApproval.types.other',
};

function formatDate(iso: string) {
  return new Date(iso + 'T00:00:00').toLocaleDateString(undefined, { dateStyle: 'medium' });
}

// ── User Submission Form ──
function SubmitForm({ onSubmitted }: { onSubmitted: () => void }) {
  const { t } = useTranslation();
  const [absenceType, setAbsenceType] = useState('vacation');
  const [startDate, setStartDate] = useState('');
  const [endDate, setEndDate] = useState('');
  const [reason, setReason] = useState('');
  const [submitting, setSubmitting] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!startDate || !endDate || !reason.trim()) {
      toast.error(t('absenceApproval.requiredFields'));
      return;
    }
    setSubmitting(true);
    try {
      const res = await api.submitAbsenceRequest({ absence_type: absenceType, start_date: startDate, end_date: endDate, reason });
      if (res.success) {
        toast.success(t('absenceApproval.submitted'));
        setStartDate(''); setEndDate(''); setReason('');
        onSubmitted();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } catch {
      toast.error(t('common.error'));
    }
    setSubmitting(false);
  }

  return (
    <form onSubmit={handleSubmit} className="space-y-4 bg-surface-50 dark:bg-surface-800 rounded-xl p-4 border border-surface-200 dark:border-surface-700">
      <h3 className="font-semibold text-surface-900 dark:text-surface-100">{t('absenceApproval.submitTitle')}</h3>
      <div className="grid grid-cols-2 gap-3">
        <div>
          <label className="text-sm text-surface-600 dark:text-surface-400">{t('absenceApproval.type')}</label>
          <select value={absenceType} onChange={e => setAbsenceType(e.target.value)} className="w-full mt-1 px-3 py-2 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-surface-900 dark:text-surface-100">
            {Object.entries(typeLabels).map(([key, label]) => (
              <option key={key} value={key}>{t(label)}</option>
            ))}
          </select>
        </div>
        <div />
        <div>
          <label className="text-sm text-surface-600 dark:text-surface-400">{t('absenceApproval.startDate')}</label>
          <input type="date" value={startDate} onChange={e => setStartDate(e.target.value)} className="w-full mt-1 px-3 py-2 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-surface-900 dark:text-surface-100" />
        </div>
        <div>
          <label className="text-sm text-surface-600 dark:text-surface-400">{t('absenceApproval.endDate')}</label>
          <input type="date" value={endDate} onChange={e => setEndDate(e.target.value)} className="w-full mt-1 px-3 py-2 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-surface-900 dark:text-surface-100" />
        </div>
      </div>
      <div>
        <label className="text-sm text-surface-600 dark:text-surface-400">{t('absenceApproval.reason')}</label>
        <textarea value={reason} onChange={e => setReason(e.target.value)} placeholder={t('absenceApproval.reasonPlaceholder')} rows={2} className="w-full mt-1 px-3 py-2 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-surface-900 dark:text-surface-100 resize-none" />
      </div>
      <button type="submit" disabled={submitting} className="w-full py-2 rounded-lg bg-primary-600 text-white font-medium hover:bg-primary-700 disabled:opacity-50 transition-colors flex items-center justify-center gap-2">
        <PaperPlaneTilt size={18} />
        {submitting ? t('absenceApproval.submitting') : t('absenceApproval.submitBtn')}
      </button>
    </form>
  );
}

// ── Admin Pending Queue ──
function AdminPendingQueue({ requests, onAction }: { requests: AbsenceRequest[]; onAction: () => void }) {
  const { t } = useTranslation();
  const [comment, setComment] = useState<Record<string, string>>({});
  const [processing, setProcessing] = useState<string | null>(null);

  async function handleApprove(id: string) {
    setProcessing(id);
    try {
      const res = await api.approveAbsenceRequest(id, comment[id] || undefined);
      if (res.success) {
        toast.success(t('absenceApproval.approved'));
        onAction();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } catch { toast.error(t('common.error')); }
    setProcessing(null);
  }

  async function handleReject(id: string) {
    if (!comment[id]?.trim()) {
      toast.error(t('absenceApproval.rejectReasonRequired'));
      return;
    }
    setProcessing(id);
    try {
      const res = await api.rejectAbsenceRequest(id, comment[id]);
      if (res.success) {
        toast.success(t('absenceApproval.rejected'));
        onAction();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } catch { toast.error(t('common.error')); }
    setProcessing(null);
  }

  if (requests.length === 0) {
    return (
      <div className="text-center py-8 text-surface-500 dark:text-surface-400">
        <Check size={32} className="mx-auto mb-2 opacity-50" />
        <p>{t('absenceApproval.noPending')}</p>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {requests.map(req => (
        <motion.div key={req.id} initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }}
          className="bg-surface-50 dark:bg-surface-800 rounded-xl p-4 border border-surface-200 dark:border-surface-700">
          <div className="flex items-start justify-between mb-2">
            <div>
              <span className="font-semibold text-surface-900 dark:text-surface-100">{req.user_name}</span>
              <span className={`ml-2 px-2 py-0.5 rounded-full text-xs font-medium ${statusColors[req.status]}`}>
                {t(`absenceApproval.status.${req.status}`)}
              </span>
            </div>
            <span className="text-xs text-surface-500">{t(typeLabels[req.absence_type] || req.absence_type)}</span>
          </div>
          <div className="text-sm text-surface-600 dark:text-surface-400 mb-2">
            <Calendar size={14} className="inline mr-1" />
            {formatDate(req.start_date)} — {formatDate(req.end_date)}
          </div>
          <p className="text-sm text-surface-700 dark:text-surface-300 mb-3">{req.reason}</p>
          <div className="flex gap-2">
            <input
              type="text"
              placeholder={t('absenceApproval.commentPlaceholder')}
              value={comment[req.id] || ''}
              onChange={e => setComment(c => ({ ...c, [req.id]: e.target.value }))}
              className="flex-1 px-3 py-1.5 rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-900 text-sm text-surface-900 dark:text-surface-100"
            />
            <button onClick={() => handleApprove(req.id)} disabled={processing === req.id}
              className="px-3 py-1.5 rounded-lg bg-green-600 text-white text-sm font-medium hover:bg-green-700 disabled:opacity-50 flex items-center gap-1">
              <Check size={14} /> {t('absenceApproval.approveBtn')}
            </button>
            <button onClick={() => handleReject(req.id)} disabled={processing === req.id}
              className="px-3 py-1.5 rounded-lg bg-red-600 text-white text-sm font-medium hover:bg-red-700 disabled:opacity-50 flex items-center gap-1">
              <X size={14} /> {t('absenceApproval.rejectBtn')}
            </button>
          </div>
        </motion.div>
      ))}
    </div>
  );
}

// ── Main Page ──
export function AbsenceApprovalPage() {
  const { t } = useTranslation();
  const [myRequests, setMyRequests] = useState<AbsenceRequest[]>([]);
  const [pendingRequests, setPendingRequests] = useState<AbsenceRequest[]>([]);
  const [loading, setLoading] = useState(true);
  const [isAdmin, setIsAdmin] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const [tab, setTab] = useState<'my' | 'admin'>('my');

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const myRes = await api.myAbsenceRequests();
      if (myRes.success && myRes.data) setMyRequests(myRes.data);

      // Try loading pending (will fail for non-admins)
      try {
        const pendingRes = await api.pendingAbsenceRequests();
        if (pendingRes.success && pendingRes.data) {
          setPendingRequests(pendingRes.data);
          setIsAdmin(true);
        }
      } catch { /* not admin */ }
    } catch { /* ignore */ }
    setLoading(false);
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  return (
    <div className="max-w-3xl mx-auto px-4 py-6">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-surface-100">{t('absenceApproval.title')}</h1>
          <p className="text-surface-500 dark:text-surface-400 text-sm">{t('absenceApproval.subtitle')}</p>
        </div>
        <button onClick={() => setShowHelp(!showHelp)} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-500" title={t('absenceApproval.helpLabel')}>
          <Question size={20} />
        </button>
      </div>

      {/* Help tooltip */}
      <AnimatePresence>
        {showHelp && (
          <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} exit={{ opacity: 0, height: 0 }}
            className="mb-4 p-3 rounded-lg bg-primary-50 dark:bg-primary-900/20 border border-primary-200 dark:border-primary-800 text-sm text-primary-800 dark:text-primary-300 flex items-start gap-2">
            <ChatText size={18} className="mt-0.5 shrink-0" />
            <span>{t('absenceApproval.help')}</span>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Submit form */}
      <div className="mb-6">
        <SubmitForm onSubmitted={loadData} />
      </div>

      {/* Tabs */}
      {isAdmin && (
        <div className="flex gap-2 mb-4">
          <button onClick={() => setTab('my')} className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${tab === 'my' ? 'bg-primary-600 text-white' : 'bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400 hover:bg-surface-200 dark:hover:bg-surface-700'}`}>
            {t('absenceApproval.myRequests')}
          </button>
          <button onClick={() => setTab('admin')} className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors flex items-center gap-1 ${tab === 'admin' ? 'bg-primary-600 text-white' : 'bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-400 hover:bg-surface-200 dark:hover:bg-surface-700'}`}>
            {t('absenceApproval.pendingQueue')}
            {pendingRequests.length > 0 && (
              <span className="ml-1 px-1.5 py-0.5 rounded-full bg-amber-500 text-white text-xs">{pendingRequests.length}</span>
            )}
          </button>
        </div>
      )}

      {/* Content */}
      {loading ? (
        <div className="text-center py-8 text-surface-500">{t('common.loading')}</div>
      ) : tab === 'admin' && isAdmin ? (
        <AdminPendingQueue requests={pendingRequests} onAction={loadData} />
      ) : (
        <div className="space-y-3">
          {myRequests.length === 0 ? (
            <div className="text-center py-8 text-surface-500 dark:text-surface-400">
              <Clock size={32} className="mx-auto mb-2 opacity-50" />
              <p>{t('absenceApproval.noRequests')}</p>
            </div>
          ) : (
            myRequests.map(req => (
              <motion.div key={req.id} initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }}
                className="bg-surface-50 dark:bg-surface-800 rounded-xl p-4 border border-surface-200 dark:border-surface-700">
                <div className="flex items-center justify-between mb-2">
                  <span className="text-sm text-surface-600 dark:text-surface-400">{t(typeLabels[req.absence_type] || req.absence_type)}</span>
                  <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${statusColors[req.status]}`}>
                    {t(`absenceApproval.status.${req.status}`)}
                  </span>
                </div>
                <div className="text-sm text-surface-700 dark:text-surface-300 mb-1">
                  <Calendar size={14} className="inline mr-1" />
                  {formatDate(req.start_date)} — {formatDate(req.end_date)}
                </div>
                <p className="text-sm text-surface-600 dark:text-surface-400">{req.reason}</p>
                {req.reviewer_comment && (
                  <div className="mt-2 p-2 rounded-lg bg-surface-100 dark:bg-surface-700 text-sm text-surface-600 dark:text-surface-400 flex items-start gap-1.5">
                    <ChatText size={14} className="mt-0.5 shrink-0" />
                    <span>{req.reviewer_comment}</span>
                  </div>
                )}
              </motion.div>
            ))
          )}
        </div>
      )}
    </div>
  );
}
