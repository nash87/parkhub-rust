import { useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ShareNetwork, Link as LinkIcon, Copy, Envelope, Question, Trash, CheckCircle, X, SpinnerGap } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

interface ShareLink {
  id: string;
  booking_id: string;
  code: string;
  url: string;
  status: 'active' | 'revoked' | 'expired';
  message: string | null;
  created_at: string;
  expires_at: string | null;
  view_count: number;
}

interface InviteResponse {
  invite_id: string;
  booking_id: string;
  email: string;
  sent_at: string;
  share_url: string;
}

interface BookingSharingProps {
  bookingId: string;
  bookingLabel?: string;
  onClose?: () => void;
}

export function BookingSharingModal({ bookingId, bookingLabel, onClose }: BookingSharingProps) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<'link' | 'invite'>('link');
  const [shareLink, setShareLink] = useState<ShareLink | null>(null);
  const [creating, setCreating] = useState(false);
  const [revoking, setRevoking] = useState(false);
  const [inviting, setInviting] = useState(false);
  const [copied, setCopied] = useState(false);
  const [inviteEmail, setInviteEmail] = useState('');
  const [inviteMessage, setInviteMessage] = useState('');
  const [expiryHours, setExpiryHours] = useState(168);
  const [showHelp, setShowHelp] = useState(false);

  const createShareLink = useCallback(async () => {
    setCreating(true);
    try {
      const res = await fetch(`/api/v1/bookings/${bookingId}/share`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ expires_in_hours: expiryHours }),
      }).then(r => r.json());
      if (res.success) {
        setShareLink(res.data);
        toast.success(t('sharing.linkCreated'));
      } else {
        toast.error(res.error || t('common.error'));
      }
    } catch {
      toast.error(t('common.error'));
    } finally {
      setCreating(false);
    }
  }, [bookingId, expiryHours, t]);

  const revokeLink = useCallback(async () => {
    setRevoking(true);
    try {
      const res = await fetch(`/api/v1/bookings/${bookingId}/share`, {
        method: 'DELETE',
      }).then(r => r.json());
      if (res.success) {
        setShareLink(null);
        toast.success(t('sharing.linkRevoked'));
      }
    } catch {
      toast.error(t('common.error'));
    } finally {
      setRevoking(false);
    }
  }, [bookingId, t]);

  const copyLink = useCallback(() => {
    if (!shareLink) return;
    const fullUrl = `${window.location.origin}${shareLink.url}`;
    navigator.clipboard.writeText(fullUrl).then(() => {
      setCopied(true);
      toast.success(t('sharing.copied'));
      setTimeout(() => setCopied(false), 2000);
    });
  }, [shareLink, t]);

  const sendInvite = useCallback(async () => {
    if (!inviteEmail || !inviteEmail.includes('@')) {
      toast.error(t('sharing.invalidEmail'));
      return;
    }
    setInviting(true);
    try {
      const res = await fetch(`/api/v1/bookings/${bookingId}/invite`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: inviteEmail, message: inviteMessage || null }),
      }).then(r => r.json());
      if (res.success) {
        toast.success(t('sharing.inviteSent', { email: inviteEmail }));
        setInviteEmail('');
        setInviteMessage('');
      } else {
        toast.error(res.error || t('common.error'));
      }
    } catch {
      toast.error(t('common.error'));
    } finally {
      setInviting(false);
    }
  }, [bookingId, inviteEmail, inviteMessage, t]);

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ opacity: 1, scale: 1 }}
      exit={{ opacity: 0, scale: 0.95 }}
      className="bg-white dark:bg-surface-800 rounded-2xl shadow-xl border border-surface-200 dark:border-surface-700 w-full max-w-md overflow-hidden"
      data-testid="sharing-modal"
    >
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-surface-200 dark:border-surface-700">
        <div className="flex items-center gap-2">
          <ShareNetwork size={20} className="text-primary-500" />
          <h2 className="font-semibold text-surface-900 dark:text-white">
            {t('sharing.title')}
          </h2>
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={() => setShowHelp(!showHelp)}
            className="p-1.5 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700"
            aria-label={t('sharing.helpLabel')}
            data-testid="sharing-help-btn"
          >
            <Question size={16} />
          </button>
          {onClose && (
            <button
              onClick={onClose}
              className="p-1.5 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700"
              data-testid="sharing-close-btn"
            >
              <X size={16} />
            </button>
          )}
        </div>
      </div>

      {/* Help */}
      <AnimatePresence>
        {showHelp && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="bg-blue-50 dark:bg-blue-900/20 border-b border-blue-200 dark:border-blue-800 px-4 py-3"
            data-testid="sharing-help"
          >
            <p className="text-sm text-blue-700 dark:text-blue-300">
              {t('sharing.help')}
            </p>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Tabs */}
      <div className="flex border-b border-surface-200 dark:border-surface-700">
        <button
          onClick={() => setActiveTab('link')}
          className={`flex-1 py-2.5 text-sm font-medium text-center transition ${
            activeTab === 'link'
              ? 'text-primary-500 border-b-2 border-primary-500'
              : 'text-surface-500 hover:text-surface-700'
          }`}
          data-testid="tab-link"
        >
          <LinkIcon size={14} className="inline mr-1" />
          {t('sharing.tabLink')}
        </button>
        <button
          onClick={() => setActiveTab('invite')}
          className={`flex-1 py-2.5 text-sm font-medium text-center transition ${
            activeTab === 'invite'
              ? 'text-primary-500 border-b-2 border-primary-500'
              : 'text-surface-500 hover:text-surface-700'
          }`}
          data-testid="tab-invite"
        >
          <Envelope size={14} className="inline mr-1" />
          {t('sharing.tabInvite')}
        </button>
      </div>

      {/* Tab Content */}
      <div className="p-4 space-y-4">
        {activeTab === 'link' && (
          <div data-testid="link-panel">
            {!shareLink ? (
              <div className="space-y-3">
                <div>
                  <label className="text-sm text-surface-600 dark:text-surface-400">
                    {t('sharing.expiryLabel')}
                  </label>
                  <select
                    value={expiryHours}
                    onChange={e => setExpiryHours(Number(e.target.value))}
                    className="mt-1 w-full rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-700 px-3 py-2 text-sm"
                    data-testid="expiry-select"
                  >
                    <option value={24}>{t('sharing.expiry24h')}</option>
                    <option value={72}>{t('sharing.expiry3d')}</option>
                    <option value={168}>{t('sharing.expiry7d')}</option>
                    <option value={720}>{t('sharing.expiry30d')}</option>
                    <option value={0}>{t('sharing.expiryNever')}</option>
                  </select>
                </div>
                <button
                  onClick={createShareLink}
                  disabled={creating}
                  className="w-full flex items-center justify-center gap-2 py-2.5 rounded-lg bg-primary-500 text-white hover:bg-primary-600 disabled:opacity-50"
                  data-testid="create-link-btn"
                >
                  {creating ? <SpinnerGap size={16} className="animate-spin" /> : <LinkIcon size={16} />}
                  {creating ? t('sharing.creating') : t('sharing.createLink')}
                </button>
              </div>
            ) : (
              <div className="space-y-3">
                <div className="flex items-center gap-2 bg-surface-50 dark:bg-surface-700 rounded-lg p-3">
                  <input
                    readOnly
                    value={`${window.location.origin}${shareLink.url}`}
                    className="flex-1 bg-transparent text-sm text-surface-900 dark:text-white outline-none"
                    data-testid="share-url-input"
                  />
                  <button
                    onClick={copyLink}
                    className="p-2 rounded-lg hover:bg-surface-200 dark:hover:bg-surface-600"
                    data-testid="copy-link-btn"
                  >
                    {copied ? <CheckCircle size={16} className="text-green-500" /> : <Copy size={16} />}
                  </button>
                </div>
                {shareLink.expires_at && (
                  <p className="text-xs text-surface-500">
                    {t('sharing.expiresAt', { date: new Date(shareLink.expires_at).toLocaleDateString() })}
                  </p>
                )}
                <button
                  onClick={revokeLink}
                  disabled={revoking}
                  className="w-full flex items-center justify-center gap-2 py-2 rounded-lg text-red-600 border border-red-200 hover:bg-red-50 dark:hover:bg-red-900/20"
                  data-testid="revoke-link-btn"
                >
                  {revoking ? <SpinnerGap size={16} className="animate-spin" /> : <Trash size={16} />}
                  {t('sharing.revokeLink')}
                </button>
              </div>
            )}
          </div>
        )}

        {activeTab === 'invite' && (
          <div className="space-y-3" data-testid="invite-panel">
            <div>
              <label className="text-sm text-surface-600 dark:text-surface-400">
                {t('sharing.guestEmail')}
              </label>
              <input
                type="email"
                value={inviteEmail}
                onChange={e => setInviteEmail(e.target.value)}
                placeholder={t('sharing.emailPlaceholder')}
                className="mt-1 w-full rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-700 px-3 py-2 text-sm"
                data-testid="invite-email-input"
              />
            </div>
            <div>
              <label className="text-sm text-surface-600 dark:text-surface-400">
                {t('sharing.messageLabel')}
              </label>
              <textarea
                value={inviteMessage}
                onChange={e => setInviteMessage(e.target.value)}
                placeholder={t('sharing.messagePlaceholder')}
                className="mt-1 w-full rounded-lg border border-surface-300 dark:border-surface-600 bg-white dark:bg-surface-700 px-3 py-2 text-sm h-20 resize-none"
                data-testid="invite-message-input"
              />
            </div>
            <button
              onClick={sendInvite}
              disabled={inviting || !inviteEmail}
              className="w-full flex items-center justify-center gap-2 py-2.5 rounded-lg bg-primary-500 text-white hover:bg-primary-600 disabled:opacity-50"
              data-testid="send-invite-btn"
            >
              {inviting ? <SpinnerGap size={16} className="animate-spin" /> : <Envelope size={16} />}
              {inviting ? t('sharing.sending') : t('sharing.sendInvite')}
            </button>
          </div>
        )}
      </div>
    </motion.div>
  );
}
