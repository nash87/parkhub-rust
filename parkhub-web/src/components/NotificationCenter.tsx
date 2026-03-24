import { useState, useEffect, useCallback, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  Bell, X, CheckCircle, XCircle, Clock, Queue, Wrench, Megaphone,
  CurrencyDollar, UserPlus, Check, Trash, Question, FunnelSimple,
} from '@phosphor-icons/react';
import toast from 'react-hot-toast';

interface CenterNotification {
  id: string;
  notification_type: string;
  title: string;
  message: string;
  read: boolean;
  action_url: string | null;
  icon: string;
  severity: string;
  type_label: string;
  created_at: string;
  date_group: string;
}

interface PaginatedResponse {
  items: CenterNotification[];
  total: number;
  page: number;
  per_page: number;
  unread_count: number;
}

const TYPE_ICONS: Record<string, typeof Bell> = {
  'check-circle': CheckCircle,
  'x-circle': XCircle,
  'clock': Clock,
  'queue': Queue,
  'wrench': Wrench,
  'megaphone': Megaphone,
  'currency-dollar': CurrencyDollar,
  'user-plus': UserPlus,
};

const SEVERITY_COLORS: Record<string, string> = {
  success: 'text-emerald-500 bg-emerald-100 dark:bg-emerald-900/30',
  warning: 'text-amber-500 bg-amber-100 dark:bg-amber-900/30',
  info: 'text-primary-500 bg-primary-100 dark:bg-primary-900/30',
  neutral: 'text-surface-500 bg-surface-100 dark:bg-surface-800',
};

function timeAgo(dateStr: string, t: (key: string, opts?: Record<string, unknown>) => string): string {
  const diff = Date.now() - new Date(dateStr).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return t('timeAgo.justNow');
  if (mins < 60) return t('timeAgo.minutesAgo', { count: mins });
  const hours = Math.floor(mins / 60);
  if (hours < 24) return t('timeAgo.hoursAgo', { count: hours });
  const days = Math.floor(hours / 24);
  return t('timeAgo.daysAgo', { count: days });
}

export function NotificationCenter() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const [notifications, setNotifications] = useState<CenterNotification[]>([]);
  const [unreadCount, setUnreadCount] = useState(0);
  const [loading, setLoading] = useState(false);
  const [filter, setFilter] = useState<'all' | 'unread' | 'read'>('all');
  const panelRef = useRef<HTMLDivElement>(null);

  const fetchUnreadCount = useCallback(async () => {
    try {
      const token = localStorage.getItem('parkhub_token');
      const res = await fetch('/api/v1/notifications/unread-count', {
        headers: { Authorization: `Bearer ${token}` },
      });
      if (res.ok) {
        const data = await res.json();
        if (data.success) setUnreadCount(data.data.count);
      }
    } catch { /* silent */ }
  }, []);

  const fetchNotifications = useCallback(async () => {
    setLoading(true);
    try {
      const token = localStorage.getItem('parkhub_token');
      const res = await fetch(`/api/v1/notifications/center?filter=${filter}&per_page=50`, {
        headers: { Authorization: `Bearer ${token}` },
      });
      if (res.ok) {
        const data = await res.json();
        if (data.success) {
          setNotifications(data.data.items);
          setUnreadCount(data.data.unread_count);
        }
      }
    } catch {
      toast.error(t('common.error'));
    } finally { setLoading(false); }
  }, [filter, t]);

  useEffect(() => { fetchUnreadCount(); const iv = setInterval(fetchUnreadCount, 30000); return () => clearInterval(iv); }, [fetchUnreadCount]);
  useEffect(() => { if (open) fetchNotifications(); }, [open, fetchNotifications]);

  // Close on click outside
  useEffect(() => {
    function handler(e: MouseEvent) {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) setOpen(false);
    }
    if (open) document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  async function markAllRead() {
    try {
      const token = localStorage.getItem('parkhub_token');
      await fetch('/api/v1/notifications/center/read-all', {
        method: 'PUT', headers: { Authorization: `Bearer ${token}` },
      });
      setNotifications(prev => prev.map(n => ({ ...n, read: true })));
      setUnreadCount(0);
    } catch { toast.error(t('common.error')); }
  }

  async function markRead(id: string) {
    try {
      const token = localStorage.getItem('parkhub_token');
      await fetch(`/api/v1/notifications/${id}/read`, {
        method: 'PUT', headers: { Authorization: `Bearer ${token}` },
      });
      setNotifications(prev => prev.map(n => n.id === id ? { ...n, read: true } : n));
      setUnreadCount(prev => Math.max(0, prev - 1));
    } catch { /* silent */ }
  }

  async function deleteNotification(id: string) {
    try {
      const token = localStorage.getItem('parkhub_token');
      await fetch(`/api/v1/notifications/center/${id}`, {
        method: 'DELETE', headers: { Authorization: `Bearer ${token}` },
      });
      setNotifications(prev => prev.filter(n => n.id !== id));
      toast.success(t('notificationCenter.deleted'));
    } catch { toast.error(t('common.error')); }
  }

  function handleClick(n: CenterNotification) {
    if (!n.read) markRead(n.id);
    if (n.action_url) { setOpen(false); navigate(n.action_url); }
  }

  // Group by date
  const grouped = notifications.reduce<Record<string, CenterNotification[]>>((acc, n) => {
    const key = n.date_group;
    (acc[key] ??= []).push(n);
    return acc;
  }, {});

  const groupLabel = (key: string) => {
    if (key === 'today') return t('notificationCenter.today');
    if (key === 'yesterday') return t('notificationCenter.yesterday');
    return key;
  };

  return (
    <div ref={panelRef} className="relative">
      {/* Bell icon button */}
      <button
        onClick={() => setOpen(o => !o)}
        className="relative p-2 rounded-lg text-surface-600 dark:text-surface-400 hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors"
        aria-label={t('notificationCenter.title')}
        title={t('notificationCenter.bellTooltip')}
      >
        <Bell weight={unreadCount > 0 ? 'fill' : 'regular'} className="w-5 h-5" />
        {unreadCount > 0 && (
          <span className="absolute -top-0.5 -right-0.5 min-w-[18px] h-[18px] flex items-center justify-center text-[10px] font-bold text-white bg-red-500 rounded-full px-1">
            {unreadCount > 99 ? '99+' : unreadCount}
          </span>
        )}
      </button>

      {/* Slide-out panel */}
      <AnimatePresence>
        {open && (
          <motion.div
            initial={{ opacity: 0, x: 20, scale: 0.95 }}
            animate={{ opacity: 1, x: 0, scale: 1 }}
            exit={{ opacity: 0, x: 20, scale: 0.95 }}
            transition={{ duration: 0.2 }}
            className="absolute right-0 top-12 w-96 max-h-[80vh] bg-white dark:bg-surface-900 border border-surface-200 dark:border-surface-700 rounded-xl shadow-2xl overflow-hidden z-50 flex flex-col"
          >
            {/* Header */}
            <div className="flex items-center justify-between px-4 py-3 border-b border-surface-200 dark:border-surface-700">
              <div className="flex items-center gap-2">
                <h3 className="font-semibold text-surface-900 dark:text-white">{t('notificationCenter.title')}</h3>
                <button onClick={() => setShowHelp(h => !h)} className="text-surface-400 hover:text-surface-600 dark:hover:text-surface-300" title={t('notificationCenter.helpLabel')}>
                  <Question weight="bold" className="w-4 h-4" />
                </button>
              </div>
              <div className="flex items-center gap-2">
                {unreadCount > 0 && (
                  <button onClick={markAllRead} className="text-xs text-primary-600 dark:text-primary-400 hover:underline">
                    {t('notificationCenter.markAllRead')}
                  </button>
                )}
                <button onClick={() => setOpen(false)} className="text-surface-400 hover:text-surface-600 dark:hover:text-surface-300">
                  <X weight="bold" className="w-4 h-4" />
                </button>
              </div>
            </div>

            {/* Help tooltip */}
            <AnimatePresence>
              {showHelp && (
                <motion.div initial={{ height: 0, opacity: 0 }} animate={{ height: 'auto', opacity: 1 }} exit={{ height: 0, opacity: 0 }} className="px-4 py-2 bg-primary-50 dark:bg-primary-950/20 text-sm text-primary-700 dark:text-primary-300 border-b border-surface-200 dark:border-surface-700">
                  {t('notificationCenter.help')}
                </motion.div>
              )}
            </AnimatePresence>

            {/* Filter bar */}
            <div className="flex items-center gap-1 px-4 py-2 border-b border-surface-100 dark:border-surface-800">
              <FunnelSimple weight="bold" className="w-4 h-4 text-surface-400 mr-1" />
              {(['all', 'unread', 'read'] as const).map(f => (
                <button
                  key={f}
                  onClick={() => setFilter(f)}
                  className={`px-2.5 py-1 text-xs rounded-full font-medium transition-colors ${
                    filter === f
                      ? 'bg-primary-100 dark:bg-primary-900/30 text-primary-700 dark:text-primary-300'
                      : 'text-surface-500 hover:bg-surface-100 dark:hover:bg-surface-800'
                  }`}
                >
                  {t(`notificationCenter.filter.${f}`)}
                </button>
              ))}
            </div>

            {/* Notification list */}
            <div className="flex-1 overflow-y-auto">
              {loading ? (
                <div className="flex items-center justify-center py-12">
                  <div className="w-6 h-6 border-2 border-primary-500 border-t-transparent rounded-full animate-spin" />
                </div>
              ) : notifications.length === 0 ? (
                <div className="flex flex-col items-center justify-center py-12 text-surface-400">
                  <Bell className="w-10 h-10 mb-2" />
                  <p className="text-sm">{t('notificationCenter.empty')}</p>
                </div>
              ) : (
                Object.entries(grouped).map(([group, items]) => (
                  <div key={group}>
                    <div className="px-4 py-1.5 text-xs font-semibold text-surface-400 uppercase tracking-wider bg-surface-50 dark:bg-surface-800/50">
                      {groupLabel(group)}
                    </div>
                    <AnimatePresence>
                      {items.map(n => {
                        const IconComp = TYPE_ICONS[n.icon] || Bell;
                        const colors = SEVERITY_COLORS[n.severity] || SEVERITY_COLORS.neutral;
                        return (
                          <motion.div
                            key={n.id}
                            layout
                            initial={{ opacity: 0, x: 20 }}
                            animate={{ opacity: 1, x: 0 }}
                            exit={{ opacity: 0, x: -100, height: 0 }}
                            className={`flex items-start gap-3 px-4 py-3 border-b border-surface-100 dark:border-surface-800 cursor-pointer hover:bg-surface-50 dark:hover:bg-surface-800/30 transition-colors ${
                              !n.read ? 'bg-primary-50/30 dark:bg-primary-950/10' : ''
                            }`}
                            onClick={() => handleClick(n)}
                          >
                            <div className={`flex-shrink-0 w-8 h-8 rounded-full flex items-center justify-center ${colors}`}>
                              <IconComp weight="fill" className="w-4 h-4" />
                            </div>
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2">
                                <p className={`text-sm font-medium truncate ${!n.read ? 'text-surface-900 dark:text-white' : 'text-surface-600 dark:text-surface-400'}`}>
                                  {n.title}
                                </p>
                                {!n.read && <span className="w-2 h-2 rounded-full bg-primary-500 flex-shrink-0" />}
                              </div>
                              <p className="text-xs text-surface-500 dark:text-surface-400 line-clamp-2 mt-0.5">{n.message}</p>
                              <p className="text-[10px] text-surface-400 mt-1">{timeAgo(n.created_at, t)}</p>
                            </div>
                            <div className="flex flex-col gap-1 flex-shrink-0">
                              {!n.read && (
                                <button onClick={e => { e.stopPropagation(); markRead(n.id); }} className="p-1 rounded text-surface-400 hover:text-emerald-500 hover:bg-emerald-50 dark:hover:bg-emerald-900/20" title={t('notificationCenter.markRead')}>
                                  <Check weight="bold" className="w-3.5 h-3.5" />
                                </button>
                              )}
                              <button onClick={e => { e.stopPropagation(); deleteNotification(n.id); }} className="p-1 rounded text-surface-400 hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20" title={t('notificationCenter.deleteOne')}>
                                <Trash weight="bold" className="w-3.5 h-3.5" />
                              </button>
                            </div>
                          </motion.div>
                        );
                      })}
                    </AnimatePresence>
                  </div>
                ))
              )}
            </div>

            {/* Footer */}
            <div className="border-t border-surface-200 dark:border-surface-700 px-4 py-2">
              <button
                onClick={() => { setOpen(false); navigate('/notifications'); }}
                className="text-sm text-primary-600 dark:text-primary-400 hover:underline w-full text-center"
              >
                {t('notificationCenter.viewAll')}
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
