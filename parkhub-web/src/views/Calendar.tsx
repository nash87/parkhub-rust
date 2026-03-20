import { useEffect, useState, useMemo } from 'react';
import { motion } from 'framer-motion';
import { CaretLeft, CaretRight, CalendarBlank } from '@phosphor-icons/react';
import { api, type CalendarEvent } from '../api/client';
import { useTranslation } from 'react-i18next';

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

  useEffect(() => {
    loadEvents();
  }, [currentMonth]);

  async function loadEvents() {
    setLoading(true);
    const year = currentMonth.getFullYear();
    const month = currentMonth.getMonth();
    const start = formatDate(new Date(year, month, 1));
    const end = formatDate(new Date(year, month + 1, 0));
    try {
      const res = await api.calendarEvents(start, end);
      if (res.success && res.data) setEvents(res.data);
    } catch { /* ignore */ }
    finally { setLoading(false); }
  }

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
          <button onClick={prevMonth} className="p-2 rounded-xl hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors min-w-[44px] min-h-[44px] flex items-center justify-center">
            <CaretLeft weight="bold" className="w-5 h-5 text-surface-600 dark:text-surface-400" />
          </button>
          <span className="text-sm font-medium text-surface-700 dark:text-surface-300 min-w-[140px] text-center">{monthLabel}</span>
          <button onClick={nextMonth} className="p-2 rounded-xl hover:bg-surface-100 dark:hover:bg-surface-800 transition-colors min-w-[44px] min-h-[44px] flex items-center justify-center">
            <CaretRight weight="bold" className="w-5 h-5 text-surface-600 dark:text-surface-400" />
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
            return (
              <button key={idx} onClick={() => setSelectedDate(day)}
                className={`min-h-[44px] sm:min-h-[80px] p-1 border-b border-r border-surface-100 dark:border-surface-800 text-left transition-colors ${
                  !inMonth ? 'opacity-30' : ''
                } ${selected ? 'bg-primary-50 dark:bg-primary-900/20' : 'hover:bg-surface-50 dark:hover:bg-surface-800/50'}`}
              >
                <span className={`inline-flex items-center justify-center w-6 h-6 text-xs font-medium rounded-full ${
                  today ? 'bg-primary-600 text-white' : 'text-surface-700 dark:text-surface-300'
                }`}>
                  {day.getDate()}
                </span>
                <div className="mt-0.5 space-y-0.5">
                  {dayEvents.slice(0, 3).map(e => (
                    <div key={e.id} className={`h-1.5 rounded-full ${statusColors[e.status] || 'bg-surface-300'}`} />
                  ))}
                  {dayEvents.length > 3 && <span className="text-[10px] text-surface-400">+{dayEvents.length - 3}</span>}
                </div>
              </button>
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
          <p className="text-sm text-surface-400 dark:text-surface-500">{t('calendar.selectDay', 'Klicke auf einen Tag, um Eintr\u00e4ge zu sehen')}</p>
        </div>
      )}
    </motion.div>
  );
}
