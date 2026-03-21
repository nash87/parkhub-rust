import { useState, useEffect, useMemo, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import {
  Translate, MagnifyingGlass, CaretDown, ThumbsUp, ThumbsDown,
  SpinnerGap, PaperPlaneTilt, X, Check, Clock,
  ChatCircleDots,
} from '@phosphor-icons/react';
import { api, type TranslationProposal, type ProposalStatus } from '../api/client';
import toast from 'react-hot-toast';
import { useAuth } from '../context/AuthContext';

// Flatten nested translation object into dot-notation keys
function flattenKeys(obj: Record<string, unknown>, prefix = ''): Record<string, string> {
  const result: Record<string, string> = {};
  for (const [key, value] of Object.entries(obj)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      Object.assign(result, flattenKeys(value as Record<string, unknown>, path));
    } else {
      result[path] = String(value);
    }
  }
  return result;
}

const LANGUAGES = [
  { code: 'en', label: 'English', flag: '🇬🇧' },
  { code: 'de', label: 'Deutsch', flag: '🇩🇪' },
  { code: 'es', label: 'Español', flag: '🇪🇸' },
  { code: 'fr', label: 'Français', flag: '🇫🇷' },
  { code: 'it', label: 'Italiano', flag: '🇮🇹' },
  { code: 'ja', label: '日本語', flag: '🇯🇵' },
  { code: 'pl', label: 'Polski', flag: '🇵🇱' },
  { code: 'pt', label: 'Português', flag: '🇵🇹' },
  { code: 'tr', label: 'Türkçe', flag: '🇹🇷' },
  { code: 'zh', label: '中文', flag: '🇨🇳' },
];

function StatusBadge({ status, t }: { status: ProposalStatus; t: (key: string) => string }) {
  const colors = {
    pending: 'badge-warning',
    approved: 'badge-success',
    rejected: 'badge-error',
  };
  const icons = {
    pending: <Clock weight="bold" className="w-3 h-3" />,
    approved: <Check weight="bold" className="w-3 h-3" />,
    rejected: <X weight="bold" className="w-3 h-3" />,
  };
  const labels = {
    pending: t('translations.statusPending'),
    approved: t('translations.statusApproved'),
    rejected: t('translations.statusRejected'),
  };
  return (
    <span className={`badge ${colors[status]}`} aria-label={labels[status]}>
      {icons[status]} {labels[status]}
    </span>
  );
}

function ProposalCard({
  proposal,
  onVote,
  currentUserId,
}: {
  proposal: TranslationProposal;
  onVote: (id: string, vote: 'up' | 'down') => void;
  currentUserId?: string;
}) {
  const { t } = useTranslation();
  const isOwn = proposal.proposed_by === currentUserId;

  return (
    <motion.div
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      className="card p-5 space-y-3"
    >
      {/* Header */}
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2 flex-wrap">
            <code className="text-xs font-mono bg-surface-100 dark:bg-surface-800 px-2 py-0.5 rounded text-primary-600 dark:text-primary-400">
              {proposal.key}
            </code>
            <span className="text-xs text-surface-400">
              {LANGUAGES.find(l => l.code === proposal.language)?.flag} {proposal.language.toUpperCase()}
            </span>
            <StatusBadge status={proposal.status} t={t} />
          </div>
          <p className="text-xs text-surface-400 mt-1">
            {t('translations.proposedBy')} <strong className="text-surface-600 dark:text-surface-300">{proposal.proposed_by_name}</strong>
            {' · '}
            {new Date(proposal.created_at).toLocaleDateString()}
          </p>
        </div>
      </div>

      {/* Diff view */}
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        <div className="rounded-lg bg-red-50 dark:bg-red-900/10 border border-red-200/40 dark:border-red-800/30 p-3">
          <p className="text-[10px] font-semibold text-red-500 dark:text-red-400 uppercase tracking-wider mb-1">{t('translations.current')}</p>
          <p className="text-sm text-red-800 dark:text-red-200 break-words">{proposal.current_value || <em className="text-surface-400">{t('translations.empty')}</em>}</p>
        </div>
        <div className="rounded-lg bg-emerald-50 dark:bg-emerald-900/10 border border-emerald-200/40 dark:border-emerald-800/30 p-3">
          <p className="text-[10px] font-semibold text-emerald-500 dark:text-emerald-400 uppercase tracking-wider mb-1">{t('translations.proposed')}</p>
          <p className="text-sm text-emerald-800 dark:text-emerald-200 break-words">{proposal.proposed_value}</p>
        </div>
      </div>

      {/* Context */}
      {proposal.context && (
        <p className="text-xs text-surface-500 dark:text-surface-400 italic">
          <ChatCircleDots weight="bold" className="w-3 h-3 inline mr-1" />
          {proposal.context}
        </p>
      )}

      {/* Review comment */}
      {proposal.review_comment && (
        <div className="text-xs bg-surface-50 dark:bg-surface-800/50 rounded-lg p-2 border-l-2 border-primary-500">
          <strong>{proposal.reviewer_name}:</strong> {proposal.review_comment}
        </div>
      )}

      {/* Voting */}
      {proposal.status === 'pending' && (
        <div className="flex items-center gap-3 pt-1">
          <button
            onClick={() => onVote(proposal.id, 'up')}
            disabled={isOwn}
            className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
              proposal.user_vote === 'up'
                ? 'bg-emerald-100 dark:bg-emerald-900/30 text-emerald-600 dark:text-emerald-400'
                : 'bg-surface-100 dark:bg-surface-800 text-surface-500 hover:bg-emerald-50 dark:hover:bg-emerald-900/20 hover:text-emerald-600'
            } ${isOwn ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
            aria-label={t('translations.voteFor')}
          >
            <ThumbsUp weight={proposal.user_vote === 'up' ? 'fill' : 'bold'} className="w-3.5 h-3.5" />
            <span className="tabular-nums">{proposal.votes_for}</span>
          </button>
          <button
            onClick={() => onVote(proposal.id, 'down')}
            disabled={isOwn}
            className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
              proposal.user_vote === 'down'
                ? 'bg-red-100 dark:bg-red-900/30 text-red-600 dark:text-red-400'
                : 'bg-surface-100 dark:bg-surface-800 text-surface-500 hover:bg-red-50 dark:hover:bg-red-900/20 hover:text-red-600'
            } ${isOwn ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
            aria-label={t('translations.voteAgainst')}
          >
            <ThumbsDown weight={proposal.user_vote === 'down' ? 'fill' : 'bold'} className="w-3.5 h-3.5" />
            <span className="tabular-nums">{proposal.votes_against}</span>
          </button>
          <span className="text-xs text-surface-400 ml-auto">
            {proposal.votes_for - proposal.votes_against > 0 ? '+' : ''}{proposal.votes_for - proposal.votes_against} {t('translations.score')}
          </span>
        </div>
      )}
    </motion.div>
  );
}

export function TranslationsPage() {
  const { t, i18n } = useTranslation();
  const { user } = useAuth();
  const [proposals, setProposals] = useState<TranslationProposal[]>([]);
  const [loading, setLoading] = useState(true);
  const [filter, setFilter] = useState<ProposalStatus | 'all'>('pending');
  const [search, setSearch] = useState('');
  const [selectedLang, setSelectedLang] = useState(i18n.language?.substring(0, 2) || 'en');

  // Propose new translation state
  const [showPropose, setShowPropose] = useState(false);
  const [proposeKey, setProposeKey] = useState('');
  const [proposeValue, setProposeValue] = useState('');
  const [proposeContext, setProposeContext] = useState('');
  const [submitting, setSubmitting] = useState(false);

  // Get all available translation keys from the current locale
  const allKeys = useMemo(() => {
    const resources = i18n.getResourceBundle(selectedLang, 'translation');
    if (!resources) return {};
    return flattenKeys(resources);
  }, [selectedLang, i18n]);

  const loadProposals = useCallback(async () => {
    setLoading(true);
    try {
      const status = filter === 'all' ? undefined : filter;
      const res = await api.getTranslationProposals(status);
      if (res.success && res.data) {
        setProposals(res.data);
      }
    } finally {
      setLoading(false);
    }
  }, [filter]);

  useEffect(() => { loadProposals(); }, [loadProposals]);

  async function handleVote(id: string, vote: 'up' | 'down') {
    const res = await api.voteOnProposal(id, vote);
    if (res.success && res.data) {
      setProposals(prev => prev.map(p => p.id === id ? res.data! : p));
      toast.success(t('translations.voteCast'));
    } else {
      toast.error(res.error?.message || t('common.error'));
      loadProposals(); // Re-sync on error
    }
  }

  async function handlePropose() {
    if (!proposeKey || !proposeValue) return;
    setSubmitting(true);
    try {
      const res = await api.createTranslationProposal({
        language: selectedLang,
        key: proposeKey,
        proposed_value: proposeValue,
        context: proposeContext || undefined,
      });
      if (res.success) {
        toast.success(t('translations.proposalCreated'));
        setShowPropose(false);
        setProposeKey('');
        setProposeValue('');
        setProposeContext('');
        loadProposals();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } finally {
      setSubmitting(false);
    }
  }

  const filteredProposals = proposals.filter(p => {
    if (selectedLang !== 'all' && p.language !== selectedLang) return false;
    if (!search) return true;
    const q = search.toLowerCase();
    return p.key.toLowerCase().includes(q) || p.proposed_value.toLowerCase().includes(q) || p.current_value.toLowerCase().includes(q);
  });

  const filteredKeys = useMemo(() => {
    if (!search) return Object.entries(allKeys);
    const q = search.toLowerCase();
    return Object.entries(allKeys).filter(([k, v]) => k.toLowerCase().includes(q) || v.toLowerCase().includes(q));
  }, [allKeys, search]);

  const [activeTab, setActiveTab] = useState<'browse' | 'proposals'>('proposals');

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white flex items-center gap-3">
            <Translate weight="fill" className="w-7 h-7 text-primary-600" />
            {t('translations.title')}
          </h1>
          <p className="text-surface-500 dark:text-surface-400 text-sm mt-1">{t('translations.subtitle')}</p>
        </div>
        <button onClick={() => setShowPropose(true)} className="btn btn-primary">
          <PaperPlaneTilt weight="bold" className="w-4 h-4" />
          {t('translations.propose')}
        </button>
      </div>

      {/* Tabs */}
      <div className="flex gap-1 bg-surface-100 dark:bg-surface-800 rounded-xl p-1">
        {(['proposals', 'browse'] as const).map(tab => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`flex-1 py-2 px-4 rounded-lg text-sm font-medium transition-colors ${
              activeTab === tab
                ? 'bg-white dark:bg-surface-900 text-surface-900 dark:text-white shadow-sm'
                : 'text-surface-500 dark:text-surface-400 hover:text-surface-700 dark:hover:text-surface-200'
            }`}
          >
            {tab === 'proposals' ? t('translations.proposals') : t('translations.browseKeys')}
          </button>
        ))}
      </div>

      {/* Filters */}
      <div className="flex flex-col sm:flex-row gap-3">
        <div className="relative flex-1">
          <MagnifyingGlass weight="bold" className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
          <input
            type="text"
            value={search}
            onChange={e => setSearch(e.target.value)}
            placeholder={t('translations.searchKeys')}
            className="input pl-9 w-full"
            aria-label={t('translations.searchKeys')}
          />
        </div>
        <select
          value={selectedLang}
          onChange={e => setSelectedLang(e.target.value)}
          className="input w-auto min-w-[140px]"
          aria-label={t('translations.selectLanguage')}
        >
          {LANGUAGES.map(l => (
            <option key={l.code} value={l.code}>{l.flag} {l.label}</option>
          ))}
        </select>
        {activeTab === 'proposals' && (
          <select
            value={filter}
            onChange={e => setFilter(e.target.value as ProposalStatus | 'all')}
            className="input w-auto min-w-[120px]"
            aria-label={t('translations.filterStatus')}
          >
            <option value="all">{t('translations.allStatuses')}</option>
            <option value="pending">{t('translations.statusPending')}</option>
            <option value="approved">{t('translations.statusApproved')}</option>
            <option value="rejected">{t('translations.statusRejected')}</option>
          </select>
        )}
      </div>

      {/* Propose Modal */}
      <AnimatePresence>
        {showPropose && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="overflow-hidden"
          >
            <div className="card p-6 space-y-4 border-l-4 border-l-primary-500">
              <div className="flex items-center justify-between">
                <h3 className="text-base font-semibold text-surface-900 dark:text-white">
                  {t('translations.newProposal')}
                </h3>
                <button onClick={() => setShowPropose(false)} className="btn btn-ghost btn-icon btn-sm" aria-label={t('common.close')}>
                  <X weight="bold" className="w-4 h-4" />
                </button>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div>
                  <label htmlFor="propose-key" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">
                    {t('translations.keyLabel')}
                  </label>
                  <input
                    id="propose-key"
                    list="translation-keys"
                    value={proposeKey}
                    onChange={e => setProposeKey(e.target.value)}
                    className="input font-mono text-sm"
                    placeholder="nav.dashboard"
                  />
                  <datalist id="translation-keys">
                    {Object.keys(allKeys).slice(0, 50).map(k => (
                      <option key={k} value={k} />
                    ))}
                  </datalist>
                  {proposeKey && allKeys[proposeKey] && (
                    <p className="text-xs text-surface-400 mt-1">
                      {t('translations.currentValue')}: <span className="text-surface-600 dark:text-surface-300">{allKeys[proposeKey]}</span>
                    </p>
                  )}
                </div>
                <div>
                  <label htmlFor="propose-value" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">
                    {t('translations.proposedValue')}
                  </label>
                  <input
                    id="propose-value"
                    value={proposeValue}
                    onChange={e => setProposeValue(e.target.value)}
                    className="input"
                    placeholder={t('translations.enterTranslation')}
                  />
                </div>
              </div>

              <div>
                <label htmlFor="propose-context" className="block text-sm font-medium text-surface-700 dark:text-surface-300 mb-1.5">
                  {t('translations.contextLabel')}
                </label>
                <input
                  id="propose-context"
                  value={proposeContext}
                  onChange={e => setProposeContext(e.target.value)}
                  className="input"
                  placeholder={t('translations.contextPlaceholder')}
                />
              </div>

              <div className="flex gap-3">
                <button onClick={handlePropose} disabled={submitting || !proposeKey || !proposeValue} className="btn btn-primary">
                  {submitting ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> : <PaperPlaneTilt weight="bold" className="w-4 h-4" />}
                  {t('translations.submitProposal')}
                </button>
                <button onClick={() => setShowPropose(false)} className="btn btn-secondary">{t('common.cancel')}</button>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Content */}
      {activeTab === 'proposals' ? (
        loading ? (
          <div className="flex items-center justify-center h-40" role="status">
            <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" />
          </div>
        ) : filteredProposals.length === 0 ? (
          <div className="card p-8 text-center">
            <Translate weight="duotone" className="w-12 h-12 text-surface-300 dark:text-surface-600 mx-auto mb-3" />
            <p className="text-sm text-surface-500 dark:text-surface-400">{t('translations.noProposals')}</p>
          </div>
        ) : (
          <div className="space-y-3">
            <p className="text-xs text-surface-400">{filteredProposals.length} {t('translations.proposalsCount')}</p>
            {filteredProposals.map(p => (
              <ProposalCard key={p.id} proposal={p} onVote={handleVote} currentUserId={user?.id} />
            ))}
          </div>
        )
      ) : (
        /* Browse all keys */
        <div className="card overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="border-b border-surface-200 dark:border-surface-700">
                  <th className="text-left px-5 py-3 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">{t('translations.keyLabel')}</th>
                  <th className="text-left px-5 py-3 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider">{t('translations.value')}</th>
                  <th className="text-right px-5 py-3 text-xs font-semibold text-surface-500 dark:text-surface-400 uppercase tracking-wider"></th>
                </tr>
              </thead>
              <tbody className="divide-y divide-surface-100 dark:divide-surface-800">
                {filteredKeys.slice(0, 100).map(([key, value]) => (
                  <tr key={key} className="hover:bg-surface-50 dark:hover:bg-surface-800/50 transition-colors">
                    <td className="px-5 py-3">
                      <code className="text-xs font-mono text-primary-600 dark:text-primary-400">{key}</code>
                    </td>
                    <td className="px-5 py-3 text-sm text-surface-700 dark:text-surface-300 max-w-xs truncate">
                      {value}
                    </td>
                    <td className="px-5 py-3 text-right">
                      <button
                        onClick={() => {
                          setProposeKey(key);
                          setProposeValue('');
                          setShowPropose(true);
                        }}
                        className="btn btn-ghost btn-sm text-xs"
                      >
                        {t('translations.suggestChange')}
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {filteredKeys.length > 100 && (
            <div className="p-4 text-center text-xs text-surface-400">
              {t('translations.showingFirst', { count: 100, total: filteredKeys.length })}
            </div>
          )}
        </div>
      )}
    </motion.div>
  );
}
