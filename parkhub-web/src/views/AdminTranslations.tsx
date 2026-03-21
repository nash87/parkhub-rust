import { useState, useEffect, useMemo, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { createColumnHelper } from '@tanstack/react-table';
import {
  Translate, SpinnerGap, Check, X, Clock, Eye,
  ThumbsUp, ThumbsDown, ChatCircleDots, ArrowsClockwise,
  CheckCircle, XCircle, MagnifyingGlass,
} from '@phosphor-icons/react';
import { api, type TranslationProposal, type ProposalStatus } from '../api/client';
import toast from 'react-hot-toast';
import { DataTable } from '../components/ui/DataTable';
import { ConfirmDialog } from '../components/ui/ConfirmDialog';

const columnHelper = createColumnHelper<TranslationProposal>();

const STATUS_COLORS: Record<ProposalStatus, string> = {
  pending: 'badge-warning',
  approved: 'badge-success',
  rejected: 'badge-error',
};

const STATUS_ICONS: Record<ProposalStatus, React.ReactNode> = {
  pending: <Clock weight="bold" className="w-3 h-3" />,
  approved: <Check weight="bold" className="w-3 h-3" />,
  rejected: <X weight="bold" className="w-3 h-3" />,
};

export function AdminTranslationsPage() {
  const { t } = useTranslation();
  const [proposals, setProposals] = useState<TranslationProposal[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<ProposalStatus | 'all'>('pending');
  const [search, setSearch] = useState('');
  const [reviewingId, setReviewingId] = useState<string | null>(null);
  const [reviewComment, setReviewComment] = useState('');
  const [reviewAction, setReviewAction] = useState<'approved' | 'rejected' | null>(null);
  const [submittingReview, setSubmittingReview] = useState(false);
  const [confirmState, setConfirmState] = useState<{open: boolean, action: () => void}>({open: false, action: () => {}});

  const loadProposals = useCallback(async () => {
    setLoading(true);
    try {
      const status = filter === 'all' ? undefined : filter;
      const res = await api.getTranslationProposals(status);
      if (res.success && res.data) setProposals(res.data);
    } finally {
      setLoading(false);
    }
  }, [filter]);

  useEffect(() => { loadProposals(); }, [loadProposals]);

  async function handleReview(id: string, status: 'approved' | 'rejected') {
    setSubmittingReview(true);
    try {
      const res = await api.reviewProposal(id, { status, comment: reviewComment || undefined });
      if (res.success && res.data) {
        setProposals(prev => prev.map(p => p.id === id ? res.data! : p));
        toast.success(status === 'approved' ? t('translations.admin.approved') : t('translations.admin.rejected'));
        setReviewingId(null);
        setReviewComment('');
        setReviewAction(null);
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } finally {
      setSubmittingReview(false);
    }
  }

  function handleBulkAction(action: 'approved' | 'rejected') {
    const pending = proposals.filter(p => p.status === 'pending');
    if (pending.length === 0) return;

    setConfirmState({
      open: true,
      action: async () => {
        setConfirmState({open: false, action: () => {}});
        let success = 0;
        for (const p of pending) {
          const res = await api.reviewProposal(p.id, { status: action });
          if (res.success) success++;
        }
        toast.success(t('translations.admin.bulkComplete', { count: success }));
        loadProposals();
      },
    });
  }

  const filteredProposals = useMemo(() => {
    if (!search) return proposals;
    const q = search.toLowerCase();
    return proposals.filter(p =>
      p.key.toLowerCase().includes(q) ||
      p.proposed_value.toLowerCase().includes(q) ||
      p.proposed_by_name.toLowerCase().includes(q) ||
      p.language.toLowerCase().includes(q)
    );
  }, [proposals, search]);

  const pendingCount = proposals.filter(p => p.status === 'pending').length;

  const columns = useMemo(() => [
    columnHelper.accessor('key', {
      header: () => t('translations.keyLabel'),
      cell: info => (
        <div className="min-w-0">
          <code className="text-xs font-mono text-primary-600 dark:text-primary-400 bg-surface-100 dark:bg-surface-800 px-1.5 py-0.5 rounded">
            {info.getValue()}
          </code>
          <p className="text-[10px] text-surface-400 mt-0.5">{info.row.original.language.toUpperCase()}</p>
        </div>
      ),
      enableSorting: true,
    }),
    columnHelper.accessor('current_value', {
      header: () => t('translations.current'),
      cell: info => (
        <span className="text-sm text-red-600 dark:text-red-400 line-through opacity-70 truncate block max-w-[200px]">
          {info.getValue() || <em className="text-surface-400 no-underline">{t('translations.empty')}</em>}
        </span>
      ),
      enableSorting: false,
    }),
    columnHelper.accessor('proposed_value', {
      header: () => t('translations.proposed'),
      cell: info => (
        <span className="text-sm text-emerald-600 dark:text-emerald-400 font-medium truncate block max-w-[200px]">
          {info.getValue()}
        </span>
      ),
      enableSorting: false,
    }),
    columnHelper.accessor('proposed_by_name', {
      header: () => t('translations.admin.proposedBy'),
      cell: info => <span className="text-sm text-surface-600 dark:text-surface-300">{info.getValue()}</span>,
      enableSorting: true,
    }),
    columnHelper.accessor('votes_for', {
      header: () => t('translations.score'),
      cell: info => {
        const p = info.row.original;
        const net = p.votes_for - p.votes_against;
        return (
          <div className="flex items-center gap-2 tabular-nums text-sm">
            <span className="text-emerald-500"><ThumbsUp weight="bold" className="w-3.5 h-3.5 inline" /> {p.votes_for}</span>
            <span className="text-red-500"><ThumbsDown weight="bold" className="w-3.5 h-3.5 inline" /> {p.votes_against}</span>
            <span className={`font-semibold ${net > 0 ? 'text-emerald-600' : net < 0 ? 'text-red-600' : 'text-surface-400'}`}>
              {net > 0 ? '+' : ''}{net}
            </span>
          </div>
        );
      },
      enableSorting: true,
    }),
    columnHelper.accessor('status', {
      header: () => t('admin.status'),
      cell: info => (
        <span className={`badge ${STATUS_COLORS[info.getValue()]}`}>
          {STATUS_ICONS[info.getValue()]} {info.getValue()}
        </span>
      ),
      enableSorting: true,
    }),
    columnHelper.display({
      id: 'actions',
      header: '',
      cell: info => {
        const p = info.row.original;
        if (p.status !== 'pending') {
          return p.reviewer_name ? (
            <span className="text-xs text-surface-500 dark:text-surface-400">{p.reviewer_name}</span>
          ) : null;
        }
        return (
          <div className="flex items-center gap-1 justify-end">
            <button
              onClick={() => { setReviewingId(p.id); setReviewAction('approved'); setReviewComment(''); }}
              className="p-2 rounded-lg hover:bg-emerald-50 dark:hover:bg-emerald-900/20 text-surface-400 hover:text-emerald-600 transition-colors"
              title={t('translations.admin.approve')}
              aria-label={`${t('translations.admin.approve')} ${p.key}`}
            >
              <CheckCircle weight="bold" className="w-4.5 h-4.5" />
            </button>
            <button
              onClick={() => { setReviewingId(p.id); setReviewAction('rejected'); setReviewComment(''); }}
              className="p-2 rounded-lg hover:bg-red-50 dark:hover:bg-red-900/20 text-surface-400 hover:text-red-600 transition-colors"
              title={t('translations.admin.reject')}
              aria-label={`${t('translations.admin.reject')} ${p.key}`}
            >
              <XCircle weight="bold" className="w-4.5 h-4.5" />
            </button>
            <button
              onClick={() => { setReviewingId(p.id); setReviewAction(null); setReviewComment(''); }}
              className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-400 hover:text-primary-600 transition-colors"
              title={t('translations.admin.reviewDetail')}
              aria-label={`${t('translations.admin.reviewDetail')} ${p.key}`}
            >
              <Eye weight="bold" className="w-4.5 h-4.5" />
            </button>
          </div>
        );
      },
    }),
  ], [t]);

  const reviewProposal = reviewingId ? proposals.find(p => p.id === reviewingId) : null;

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <h2 className="text-xl font-semibold text-surface-900 dark:text-white flex items-center gap-2">
            <Translate weight="fill" className="w-5 h-5 text-primary-600" />
            {t('translations.admin.title')}
          </h2>
          {pendingCount > 0 && (
            <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-semibold bg-amber-100 dark:bg-amber-900/30 text-amber-600 dark:text-amber-400 tabular-nums">
              {pendingCount} {t('translations.admin.pendingReview')}
            </span>
          )}
        </div>
        <div className="flex items-center gap-2">
          {pendingCount > 0 && (
            <>
              <button onClick={() => handleBulkAction('approved')} className="btn btn-sm btn-secondary text-emerald-600">
                <CheckCircle weight="bold" className="w-4 h-4" />
                {t('translations.admin.approveAll')}
              </button>
              <button onClick={() => handleBulkAction('rejected')} className="btn btn-sm btn-secondary text-red-600">
                <XCircle weight="bold" className="w-4 h-4" />
                {t('translations.admin.rejectAll')}
              </button>
            </>
          )}
          <button onClick={loadProposals} className="btn btn-sm btn-ghost" aria-label={t('common.refresh')}>
            <ArrowsClockwise weight="bold" className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* Filters */}
      <div className="flex flex-col sm:flex-row gap-3">
        <div className="relative flex-1">
          <MagnifyingGlass weight="bold" className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
          <input
            type="text"
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder={t('translations.admin.searchProposals')}
            className="input pl-9 w-full"
            aria-label={t('translations.admin.searchProposals')}
          />
        </div>
        <select
          value={filter}
          onChange={e => setFilter(e.target.value as ProposalStatus | 'all')}
          className="input w-auto min-w-[140px]"
          aria-label={t('translations.filterStatus')}
        >
          <option value="all">{t('translations.allStatuses')}</option>
          <option value="pending">{t('translations.statusPending')}</option>
          <option value="approved">{t('translations.statusApproved')}</option>
          <option value="rejected">{t('translations.statusRejected')}</option>
        </select>
      </div>

      {/* Review Detail Panel */}
      <AnimatePresence>
        {reviewProposal && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="overflow-hidden"
          >
            <div className="card p-6 space-y-4 border-l-4 border-l-primary-500">
              <div className="flex items-start justify-between">
                <div>
                  <h3 className="text-base font-semibold text-surface-900 dark:text-white">
                    {t('translations.admin.reviewProposal')}
                  </h3>
                  <p className="text-xs text-surface-400 mt-1">
                    {t('translations.proposedBy')} <strong>{reviewProposal.proposed_by_name}</strong>
                    {' · '}{new Date(reviewProposal.created_at).toLocaleString()}
                  </p>
                </div>
                <button onClick={() => { setReviewingId(null); setReviewAction(null); }} className="btn btn-ghost btn-icon btn-sm" aria-label={t('common.close')}>
                  <X weight="bold" className="w-4 h-4" />
                </button>
              </div>

              {/* Key + language */}
              <div className="flex items-center gap-2 flex-wrap">
                <code className="text-sm font-mono bg-surface-100 dark:bg-surface-800 px-2 py-1 rounded text-primary-600 dark:text-primary-400">
                  {reviewProposal.key}
                </code>
                <span className="text-xs text-surface-400 font-medium">{reviewProposal.language.toUpperCase()}</span>
                <div className="flex items-center gap-2 ml-auto tabular-nums text-sm">
                  <span className="text-emerald-500"><ThumbsUp weight="bold" className="w-3.5 h-3.5 inline" /> {reviewProposal.votes_for}</span>
                  <span className="text-red-500"><ThumbsDown weight="bold" className="w-3.5 h-3.5 inline" /> {reviewProposal.votes_against}</span>
                </div>
              </div>

              {/* Diff */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div className="rounded-lg bg-red-50 dark:bg-red-900/10 border border-red-200/40 dark:border-red-800/30 p-3">
                  <p className="text-[10px] font-semibold text-red-500 dark:text-red-400 uppercase tracking-wider mb-1">{t('translations.current')}</p>
                  <p className="text-sm text-red-800 dark:text-red-200 break-words">{reviewProposal.current_value || <em className="text-surface-400">{t('translations.empty')}</em>}</p>
                </div>
                <div className="rounded-lg bg-emerald-50 dark:bg-emerald-900/10 border border-emerald-200/40 dark:border-emerald-800/30 p-3">
                  <p className="text-[10px] font-semibold text-emerald-500 dark:text-emerald-400 uppercase tracking-wider mb-1">{t('translations.proposed')}</p>
                  <p className="text-sm text-emerald-800 dark:text-emerald-200 break-words">{reviewProposal.proposed_value}</p>
                </div>
              </div>

              {/* Context */}
              {reviewProposal.context && (
                <p className="text-xs text-surface-500 dark:text-surface-400 italic">
                  <ChatCircleDots weight="bold" className="w-3 h-3 inline mr-1" />
                  {reviewProposal.context}
                </p>
              )}

              {/* Review form */}
              {reviewProposal.status === 'pending' && (
                <div className="space-y-3 pt-2 border-t border-surface-200 dark:border-surface-700">
                  <div>
                    <label htmlFor="review-comment" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">
                      {t('translations.admin.comment')}
                    </label>
                    <input
                      id="review-comment"
                      type="text"
                      value={reviewComment}
                      onChange={e => setReviewComment(e.target.value)}
                      className="input"
                      placeholder={t('translations.admin.commentPlaceholder')}
                    />
                  </div>
                  <div className="flex gap-3">
                    <button
                      onClick={() => handleReview(reviewProposal.id, 'approved')}
                      disabled={submittingReview}
                      className="btn btn-primary bg-emerald-600 hover:bg-emerald-700"
                    >
                      {submittingReview && reviewAction === 'approved'
                        ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" />
                        : <CheckCircle weight="bold" className="w-4 h-4" />}
                      {t('translations.admin.approve')}
                    </button>
                    <button
                      onClick={() => handleReview(reviewProposal.id, 'rejected')}
                      disabled={submittingReview}
                      className="btn bg-red-600 hover:bg-red-700 text-white"
                    >
                      {submittingReview && reviewAction === 'rejected'
                        ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" />
                        : <XCircle weight="bold" className="w-4 h-4" />}
                      {t('translations.admin.reject')}
                    </button>
                    <button onClick={() => { setReviewingId(null); setReviewAction(null); }} className="btn btn-secondary">
                      {t('common.cancel')}
                    </button>
                  </div>
                </div>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Proposals Table */}
      <DataTable
        data={filteredProposals}
        columns={columns}
        searchValue=""
        emptyMessage={t('translations.noProposals')}
      />
      <ConfirmDialog
        open={confirmState.open}
        title={t('ui.confirmAction')}
        message={t('translations.admin.confirmBulkApprove', { count: proposals.filter(p => p.status === 'pending').length })}
        variant="danger"
        onConfirm={confirmState.action}
        onCancel={() => setConfirmState({open: false, action: () => {}})}
      />
    </motion.div>
  );
}
