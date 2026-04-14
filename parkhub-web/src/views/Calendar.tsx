import { useEffect, useRef, useState, useMemo, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { CaretLeft, CaretRight, CalendarBlank, LinkSimple, X, Copy, Check, Question, ArrowsClockwise } from '@phosphor-icons/react';
import { api, type CalendarEvent } from '../api/client';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

const statusColors: Record<string, string> = {
  confirmed: 'bg-emerald-500',
  active: 'bg-emerald-500',
  pending: 'bg-amber-500',
  cancelled: 'bg-surface-400',
  completed: 'bg-primary-500',
};

function isSameDay(a: Date, b: Date) {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

function isSameMonth(a: Date, b: Date) {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth();
}

function isToday(d: Date) {
  return isSameDay(d, new Date());
}

function formatDate(d: Date) {
  return d.toISOString().slice(0, 10);
}

export function CalendarPage() {
  const { t } = useTranslation();
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [currentMonth, setCurrentMonth] = useState(new Date());
  const [selectedDate, setSelectedDate] = useState<Date | null>(null);
  const [loading, setLoading] = useState(true);
  const [showSubscribeModal, setShowSubscribeModal] = useState(false);
  const [subscriptionUrl, setSubscriptionUrl] = useState('');
  const [copied, setCopied] = useState(false);
  const [generatingToken, setGeneratingToken] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const [dragEvent, setDragEvent] = useState<CalendarEvent | null>(null);
  const [dropTarget, setDropTarget] = useState<Date | null>(null);
  const [showRescheduleConfirm, setShowRescheduleConfirm] = useState(false);
  const [rescheduling, setRescheduling] = useState(false);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(() => {
    loadEvents();
    return () => { abortRef.current?.abort(); };
  }, [currentMonth]);

  async function loadEvents() {
    abortRef.current?.abort();
    const controller = new AbortController();
    abortRef.current = controller;
    setLoading(true);
    const year = currentMonth.getFullYear();
    const month = currentMonth.getMonth();
    const start = formatDate(new Date(year, month, 1));
    const end = formatDate(new Date(year, month + 1, 0));
    try {
      const res = await api.calendarEvents(start, end);
      if (controller.signal.aborted) return;
      if (res.success && res.data) setEvents(res.data);
    /* istanbul ignore next -- network failure path */
    } catch {
      if (!controller.signal.aborted) toast.error(t('common.error'));
    } finally {
      if (!controller.signal.aborted) setLoading(false);
    }
  }

  const handleSubscribe = useCallback(async () => {
    setGeneratingToken(true);
    try {
      const res = await api.generateCalendarToken();
      if (res.success && res.data) {
        setSubscriptionUrl(res.data.url);
        setShowSubscribeModal(true);
      } else {
        toast.error(t('common.error'));
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    } finally {
      setGeneratingToken(false);
    }
  }, [t]);

  const handleCopyLink = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(subscriptionUrl);
      setCopied(true);
      toast.success(t('calendar.linkCopied', 'Link copied'));
      setTimeout(() => setCopied(false), 2000);
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
  }, [subscriptionUrl, t]);

  // Build calendar grid (Monday start)
  const days = useMemo(() => {
    const year = currentMonth.getFullYear();
    const month = currentMonth.getMonth();
    const firstDay = new Date(year, month, 1);
    const lastDay = new Date(year, month + 1, 0);
    const startDow = firstDay.getDay() === 0 ? 6 : firstDay.getDay() - 1;

    const result: Date[] = [];
    // Previous month padding
    for (let i = 0; i < startDow; i++) {
      result.push(new Date(year, month, 1 - startDow + i));
    }
    // Current month
    for (let d = 1; d <= lastDay.getDate(); d++) {
      result.push(new Date(year, month, d));
    }
    // Next month padding
    while (result.length % 7 !== 0) {
      result.push(new Date(year, month + 1, result.length - startDow - lastDay.getDate() + 1));
    }
    return result;
  }, [currentMonth]);

  // Pre-index events by date string for O(1) lookup per day
  const eventsByDay = useMemo(() => {
    const map = new Map<string, CalendarEvent[]>();
    for (const e of events) {
      const key = new Date(e.start).toISOString().slice(0, 10);
      const list = map.get(key);
      if (list) list.push(e);
      else map.set(key, [e]);
    }
    return map;
  }, [events]);

  const eventsForDay = (day: Date) =>
    eventsByDay.get(formatDate(day)) ?? [];

  const selectedEvents = selectedDate ? eventsForDay(selectedDate) : [];

  const monthLabel = currentMonth.toLocaleDateString(undefined, { month: 'long', year: 'numeric' });
  // Use i18n-aware weekday abbreviations (Monday-start)
  const WEEKDAYS = useMemo(() => {
    const base = new Date(2026, 0, 5); // Monday
    return Array.from({ length: 7 }, (_, i) => {
      const d = new Date(base);
      d.setDate(base.getDate() + i);
      return d.toLocaleDateString(undefined, { weekday: 'short' });
    });
  }, []);

  // ── Drag-to-Reschedule handlers ──
  function handleDragStart(e: React.DragEvent, event: CalendarEvent) {
    if (event.type !== 'booking') return;
    setDragEvent(event);
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', event.id);
  }

  function handleDragOver(e: React.DragEvent, day: Date) {
    if (!dragEvent) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    setDropTarget(day);
  }

  function handleDragLeave() {
    setDropTarget(null);
  }

  function handleDrop(e: React.DragEvent, day: Date) {
    e.preventDefault();
    if (!dragEvent) return;
    setDropTarget(day);
    setShowRescheduleConfirm(true);
  }

  function handleDragEnd() {
    if (!showRescheduleConfirm) {
      setDragEvent(null);
      setDropTarget(null);
    }
  }

  async function confirmReschedule() {
    if (!dragEvent || !dropTarget) return;
    setRescheduling(true);
    try {
      const oldStart = new Date(dragEvent.start);
      const oldEnd = new Date(dragEvent.end);
      const duration = oldEnd.getTime() - oldStart.getTime();
      const newStart = new Date(dropTarget.getFullYear(), dropTarget.getMonth(), dropTarget.getDate(), oldStart.getHours(), oldStart.getMinutes());
      const newEnd = new Date(newStart.getTime() + duration);
      const res = await api.rescheduleBooking(dragEvent.id, newStart.toISOString(), newEnd.toISOString());
      if (res.success && res.data?.success) {
        toast.success(t('calendarDrag.rescheduled'));
        loadEvents();
      } else {
        toast.error(res.data?.message || res.error?.message || t('common.error'));
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
    setRescheduling(false);
    setShowRescheduleConfirm(false);
    setDragEvent(null);
    setDropTarget(null);
  }

  function cancelReschedule() {
    setShowRescheduleConfirm(false);
    setDragEvent(null);
    setDropTarget(null);
  }

  function prevMonth() {
    setCurrentMonth(d => new Date(d.getFullYear(), d.getMonth() - 1, 1));
  }
  function nextMonth() {
    setCurrentMonth(d => new Date(d.getFullYear(), d.getMonth() + 1, 1));
  }

  if (loading) return (
    <div className="space-y-4">
      <div className="h-8 w-48 skeleton rounded-lg" />
      <div className="h-96 skeleton rounded-2xl" />
    </div>
  );

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
        <h1 className="text-2xl font-bold text-surface-900 dark:text-white">{t('calendar.title', 'Kalender')}</h1>
        <div className="flex items-center gap-2 self-start sm:self-auto">
          <button onClick={() => setShowHelp(!showHelp)} className="p-2 rounded-xl hover:bg-surface-100 dark:hover:bg-surface-800 text-surface-500 min-w-[44px] min-h-[44px] flex items-center justify-center" title={t('calendarDrag.helpLabel')}>
            <Question size={20} />
          </button>
          <button
            onClick={handleSubscribe}
            disabled={generatingToken}
            aria-label={t('calendar.subscribe', 'Subscribe to Calendar')}
            className="flex items-center gap-1.5 px-3 py-2 text-xs font-medium rounded-xl bg-primary-600 text-white hover:bg-primary-700 transition-colors disabled:opacity-50 min-h-[44px]"
          >
            <LinkSimple weight="bold" className="w-4 h-4" aria-hidden="true" />
            {t('calendar.subscribe', 'Subscribe')}
          </button>
          <button onClick={prevMonth} aria-label={t('calendar.previousMonth', 'Previous month')} className="p-2 rounded-xl hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors min-w-[44px] min-h-[44px] flex items-center justify-center">
            <CaretLeft weight="bold" className="w-5 h-5 text-surface-600 dark:text-surface-400" aria-hidden="true" />
          </button>
          <span aria-live="polite" className="text-sm font-medium text-surface-700 dark:text-surface-300 min-w-[140px] text-center">{monthLabel}</span>
          <button onClick={nextMonth} aria-label={t('calendar.nextMonth', 'Next month')} className="p-2 rounded-xl hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors min-w-[44px] min-h-[44px] flex items-center justify-center">
            <CaretRight weight="bold" className="w-5 h-5 text-surface-600 dark:text-surface-400" aria-hidden="true" />
          </button>
        </div>
      </div>

      {/* Calendar grid */}
      <div className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 overflow-hidden">
        <div className="grid grid-cols-7 border-b border-surface-200 dark:border-surface-800">
          {WEEKDAYS.map(day => (
            <div key={day} className="p-2 text-center text-xs font-medium text-surface-500 dark:text-surface-400">{day}</div>
          ))}
        </div>
        <div className="grid grid-cols-7">
          {days.map((day, idx) => {
            const dayEvents = eventsForDay(day);
            const inMonth = isSameMonth(day, currentMonth);
            const today = isToday(day);
            const selected = selectedDate && isSameDay(day, selectedDate);
            const isDropTarget = dropTarget && isSameDay(day, dropTarget);
            return (
              <div key={idx}
                onClick={() => setSelectedDate(day)}
                onDragOver={(e) => handleDragOver(e, day)}
                onDragLeave={handleDragLeave}
                onDrop={(e) => handleDrop(e, day)}
                role="button"
                tabIndex={0}
                aria-label={`${day.toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long' })}${dayEvents.length > 0 ? `, ${dayEvents.length} ${t('calendar.events', 'events')}` : ''}`}
                aria-pressed={!!selected}
                className={`min-h-[44px] sm:min-h-[80px] p-1 border-b border-r border-surface-100 dark:border-surface-800 text-left transition-colors cursor-pointer ${
                  !inMonth ? 'opacity-30' : ''
                } ${selected ? 'bg-primary-50 dark:bg-primary-900/20' : 'hover:bg-surface-50 dark:hover:bg-surface-800/50'} ${
                  isDropTarget ? 'ring-2 ring-primary-500 bg-primary-50/50 dark:bg-primary-900/30' : ''
                }`}
              >
                <span className={`inline-flex items-center justify-center w-6 h-6 text-xs font-medium rounded-full ${
                  today ? 'bg-primary-600 text-white' : 'text-surface-700 dark:text-surface-300'
                }`}>
                  {day.getDate()}
                </span>
                <div className="mt-0.5 space-y-0.5">
                  {dayEvents.slice(0, 3).map(e => (
                    <div key={e.id}
                      draggable={e.type === 'booking'}
                      onDragStart={(ev) => handleDragStart(ev, e)}
                      onDragEnd={handleDragEnd}
                      className={`h-1.5 rounded-full ${statusColors[e.status] || 'bg-surface-300'} ${e.type === 'booking' ? 'cursor-grab active:cursor-grabbing' : ''}`}
                    />
                  ))}
                  {dayEvents.length > 3 && <span className="text-[10px] text-surface-400">+{dayEvents.length - 3}</span>}
                </div>
              </div>
            );
          })}
        </div>
      </div>

      {/* Selected day detail */}
      {selectedDate ? (
        <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} className="space-y-3">
          <h2 className="text-lg font-semibold text-surface-900 dark:text-white">
            {selectedDate.toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long' })}
          </h2>
          {selectedEvents.length === 0 ? (
            <div className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-8 text-center">
              <CalendarBlank weight="light" className="w-10 h-10 text-surface-300 dark:text-surface-600 mx-auto mb-2" />
              <p className="text-sm text-surface-500 dark:text-surface-400">{t('calendar.noBookings', 'Keine Eintr\u00e4ge an diesem Tag')}</p>
            </div>
          ) : (
            <div className="space-y-2">
              {selectedEvents.map(e => (
                <div key={e.id} className="flex items-center gap-3 p-3 rounded-xl bg-white dark:bg-surface-800 border border-surface-200 dark:border-surface-700">
                  <div className={`w-2 h-8 rounded-full ${statusColors[e.status] || 'bg-surface-300'}`} />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium text-surface-900 dark:text-white truncate">{e.title}</p>
                    <p className="text-xs text-surface-500 dark:text-surface-400">
                      {new Date(e.start).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })} - {new Date(e.end).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' })}
                      {e.lot_name && ` \u00b7 ${e.lot_name}`}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </motion.div>
      ) : (
        <div className="text-center py-4">
          <p className="text-sm text-surface-500 dark:text-surface-400">{t('calendar.selectDay', 'Klicke auf einen Tag, um Eintr\u00e4ge zu sehen')}</p>
        </div>
      )}

      {/* Drag help tooltip */}
      <AnimatePresence>
        {showHelp && (
          <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} exit={{ opacity: 0, height: 0 }}
            className="p-3 rounded-lg bg-primary-50 dark:bg-primary-900/20 border border-primary-200 dark:border-primary-800 text-sm text-primary-800 dark:text-primary-300 flex items-start gap-2">
            <ArrowsClockwise size={18} className="mt-0.5 shrink-0" />
            <span>{t('calendarDrag.help')}</span>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Reschedule confirmation dialog */}
      <AnimatePresence>
        {showRescheduleConfirm && dragEvent && dropTarget && (
          <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" data-testid="reschedule-confirm">
            <motion.div
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.95 }}
              className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-6 max-w-sm w-full mx-4 shadow-xl"
            >
              <h3 className="text-lg font-semibold text-surface-900 dark:text-white mb-3">
                {t('calendarDrag.confirmTitle')}
              </h3>
              <p className="text-sm text-surface-600 dark:text-surface-400 mb-2">
                {t('calendarDrag.confirmDesc')}
              </p>
              <div className="text-sm text-surface-700 dark:text-surface-300 mb-4 space-y-1">
                <p><strong>{dragEvent.title}</strong></p>
                <p>{t('calendarDrag.from')}: {new Date(dragEvent.start).toLocaleDateString()}</p>
                <p>{t('calendarDrag.to')}: {dropTarget.toLocaleDateString()}</p>
              </div>
              <div className="flex gap-2 justify-end">
                <button onClick={cancelReschedule}
                  className="px-4 py-2 rounded-lg text-sm font-medium bg-surface-100 dark:bg-surface-800 text-surface-700 dark:text-surface-300 hover:bg-surface-200 dark:hover:bg-surface-700">
                  {t('common.cancel', 'Cancel')}
                </button>
                <button onClick={confirmReschedule} disabled={rescheduling}
                  className="px-4 py-2 rounded-lg text-sm font-medium bg-primary-600 text-white hover:bg-primary-700 disabled:opacity-50 flex items-center gap-1">
                  <ArrowsClockwise size={14} />
                  {rescheduling ? t('calendarDrag.rescheduling') : t('calendarDrag.confirmBtn')}
                </button>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>

      {/* Subscribe modal */}
      <AnimatePresence>
        {showSubscribeModal && (
          <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={() => setShowSubscribeModal(false)}>
            <motion.div
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.95 }}
              onClick={(e) => e.stopPropagation()}
              className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-6 max-w-lg w-full mx-4 shadow-xl"
            >
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-lg font-semibold text-surface-900 dark:text-white">
                  {t('calendar.subscribeTitle', 'Subscribe to Calendar')}
                </h3>
                <button
                  onClick={() => setShowSubscribeModal(false)}
                  aria-label={t('common.close', 'Close')}
                  className="p-1 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors"
                >
                  <X weight="bold" className="w-5 h-5 text-surface-500" />
                </button>
              </div>

              <p className="text-sm text-surface-600 dark:text-surface-400 mb-4">
                {t('calendar.subscribeDesc', 'Use this URL to subscribe to your parking calendar from any calendar app.')}
              </p>

              {/* URL field with copy button */}
              <div className="flex items-center gap-2 mb-6">
                <input
                  type="text"
                  readOnly
                  value={subscriptionUrl}
                  className="flex-1 px-3 py-2 text-sm rounded-xl border border-surface-200 dark:border-surface-700 bg-surface-50 dark:bg-surface-800 text-surface-900 dark:text-white font-mono truncate"
                  data-testid="subscription-url"
                />
                <button
                  onClick={handleCopyLink}
                  aria-label={t('calendar.copyLink', 'Copy link')}
                  className="flex items-center gap-1.5 px-3 py-2 text-sm font-medium rounded-xl bg-primary-600 text-white hover:bg-primary-700 transition-colors min-h-[40px]"
                >
                  {copied ? <Check weight="bold" className="w-4 h-4" /> : <Copy weight="bold" className="w-4 h-4" />}
                  {t('calendar.copyLink', 'Copy')}
                </button>
              </div>

              {/* Instructions */}
              <div className="space-y-3">
                <h4 className="text-sm font-medium text-surface-900 dark:text-white">
                  {t('calendar.instructions', 'How to subscribe')}
                </h4>
                <div className="space-y-2 text-xs text-surface-600 dark:text-surface-400">
                  <div className="flex gap-2">
                    <span className="font-semibold text-surface-700 dark:text-surface-300 shrink-0">Google Calendar:</span>
                    <span>{t('calendar.instructionGoogle', 'Settings > Add calendar > From URL > paste the link')}</span>
                  </div>
                  <div className="flex gap-2">
                    <span className="font-semibold text-surface-700 dark:text-surface-300 shrink-0">Outlook:</span>
                    <span>{t('calendar.instructionOutlook', 'Add calendar > Subscribe from web > paste the link')}</span>
                  </div>
                  <div className="flex gap-2">
                    <span className="font-semibold text-surface-700 dark:text-surface-300 shrink-0">Apple Calendar:</span>
                    <span>{t('calendar.instructionApple', 'File > New Calendar Subscription > paste the link')}</span>
                  </div>
                </div>
              </div>
            </motion.div>
          </div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}
