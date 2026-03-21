import { useEffect, useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Bell, Warning, Info, CheckCircle, Check, SpinnerGap, ArrowClockwise,
} from '@phosphor-icons/react';
import { api, type Notification } from '../api/client';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

const notifIcon: Record<string, typeof Warning> = { warning: Warning, info: Info, success: CheckCircle };
const notifColor: Record<string, string> = { warning: 'text-amber-500', info: 'text-primary-500', success: 'text-emerald-500' };
const notifBg: Record<string, string> = { warning: 'bg-amber-100 dark:bg-amber-900/30', info: 'bg-primary-100 dark:bg-primary-900/30', success: 'bg-emerald-100 dark:bg-emerald-900/30' };

function resolveType(raw: string): string {
  if (raw === 'warning') return 'warning';
  if (raw === 'success') return 'success';
  return 'info';
}

function timeAgoFn(dateStr: string, t: (key: string, opts?: Record<string, unknown>) => string): string {
  const diff = Date.now() - new Date(dateStr).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return t('timeAgo.justNow');
  if (mins < 60) return t('timeAgo.minutesAgo', { count: mins });
  const hours = Math.floor(mins / 60);
  if (hours < 24) return t('timeAgo.hoursAgo', { count: hours });
  const days = Math.floor(hours / 24);
  return t('timeAgo.daysAgo', { count: days });
}

export function NotificationsPage() {
  const { t } = useTranslation();
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [loading, setLoading] = useState(true);
  const [markingAll, setMarkingAll] = useState(false);

  useEffect(() => { loadNotifications(); }, []);

  async function loadNotifications() {
    setLoading(true);
    try {
      const res = await api.getNotifications();
      if (res.success && res.data) setNotifications(res.data);
    } catch { /* ignore */ }
    finally { setLoading(false); }
  }

  async function markAsRead(id: string) {
    setNotifications(prev => prev.map(n => n.id === id ? { ...n, read: true } : n));
    try { await api.markNotificationRead(id); } catch {
      setNotifications(prev => prev.map(n => n.id === id ? { ...n, read: false } : n));
    }
  }

  async function markAllAsRead() {
    setMarkingAll(true);
    try {
      const res = await api.markAllNotificationsRead();
      if (res.success) {
        setNotifications(prev => prev.map(n => ({ ...n, read: true })));
        toast.success(t('notifications.allMarkedRead'));
      }
    } catch { toast.error(t('common.error')); }
    finally { setMarkingAll(false); }
  }

  const unreadCount = useMemo(() => notifications.filter(n => !n.read).length, [notifications]);

  if (loading) return (
    <div className="space-y-6">
      <div className="h-8 w-64 skeleton rounded-xl" />
      {[1, 2, 3].map(i => <div key={i} className="h-20 skeleton rounded-2xl" />)}
    </div>
  );

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-8">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white">{t('notifications.title')}</h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">
            {unreadCount > 0 ? t('notifications.unreadCount', { count: unreadCount }) : t('notifications.allRead')}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={loadNotifications} className="btn btn-secondary">
            <ArrowClockwise weight="bold" className="w-4 h-4" /> {t('common.refresh')}
          </button>
          {unreadCount > 0 && (
            <button onClick={markAllAsRead} disabled={markingAll} className="btn btn-primary">
              {markingAll ? <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" /> : <Check weight="bold" className="w-4 h-4" />}
              {t('notifications.markAllRead')}
            </button>
          )}
        </div>
      </div>

      {/* List */}
      {notifications.length === 0 ? (
        <div className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-12 text-center">
          <Bell weight="light" className="w-20 h-20 text-surface-200 dark:text-surface-700 mx-auto mb-4" />
          <p className="text-surface-500 dark:text-surface-400">{t('notifications.empty')}</p>
        </div>
      ) : (
        <div className="space-y-2" role="list" aria-label={t('notifications.title')}>
          <AnimatePresence>
            {notifications.map(n => {
              const nType = resolveType(n.notification_type);
              const NIcon = notifIcon[nType] || Info;
              return (
                <motion.button
                  key={n.id}
                  role="listitem"
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, x: -50 }}
                  onClick={() => { if (!n.read) markAsRead(n.id); }}
                  aria-label={`${n.title} — ${n.read ? t('notifications.allRead') : t('notifications.unread')}`}
                  className={`w-full text-left bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-4 flex items-start gap-4 transition-all hover:shadow-md ${
                    n.read ? 'opacity-60' : 'ring-1 ring-primary-200 dark:ring-primary-800'
                  }`}
                >
                  <div className={`w-10 h-10 rounded-xl flex items-center justify-center flex-shrink-0 ${notifBg[nType]}`}>
                    <NIcon weight="fill" className={`w-5 h-5 ${notifColor[nType]}`} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-start justify-between gap-2">
                      <p className={`text-sm font-semibold ${n.read ? 'text-surface-500 dark:text-surface-400' : 'text-surface-900 dark:text-white'}`}>
                        {n.title}
                      </p>
                      <div className="flex items-center gap-2 flex-shrink-0">
                        <span className="text-xs text-surface-500 dark:text-surface-400 whitespace-nowrap">{timeAgoFn(n.created_at, t)}</span>
                        {!n.read && <span className="w-2.5 h-2.5 bg-primary-500 rounded-full flex-shrink-0" />}
                      </div>
                    </div>
                    <p className={`text-sm mt-0.5 ${n.read ? 'text-surface-400' : 'text-surface-600 dark:text-surface-300'}`}>{n.message}</p>
                  </div>
                </motion.button>
              );
            })}
          </AnimatePresence>
        </div>
      )}
    </motion.div>
  );
}
