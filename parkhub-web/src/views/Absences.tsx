import { useState, useEffect, useMemo, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  House, Calendar, CalendarCheck, Trash, Plus, CaretLeft, CaretRight,
  X,
} from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';
import { api, type AbsenceEntry, type AbsencePattern } from '../api/client';
import { ABSENCE_CONFIG, type AbsenceType } from '../constants/absenceConfig';

function isDateInRange(date: Date, start: string, end: string) {
  const d = date.toISOString().slice(0, 10);
  return d >= start && d <= end;
}

function isSameDay(a: Date, b: Date) {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

export function AbsencesPage() {
  const { t } = useTranslation();
  const [entries, setEntries] = useState<AbsenceEntry[]>([]);
  const [patterns, setPatterns] = useState<AbsencePattern[]>([]);
  const [loading, setLoading] = useState(true);
  const [showAdd, setShowAdd] = useState(false);
  const [showPattern, setShowPattern] = useState(false);

  const today = useMemo(() => new Date(), []);
  const todayStr = today.toISOString().slice(0, 10);
  const [calMonth, setCalMonth] = useState(today.getMonth());
  const [calYear, setCalYear] = useState(today.getFullYear());

  const loadData = useCallback(async () => {
    try {
      const [absRes, patRes] = await Promise.all([api.listAbsences(), api.getAbsencePattern()]);
      if (absRes.success && absRes.data) setEntries(absRes.data);
      if (patRes.success && patRes.data) setPatterns(patRes.data);
    } catch { /* ignore */ }
    setLoading(false);
  }, []);

  useEffect(() => { loadData(); }, [loadData]);

  const hoPattern = useMemo(() => patterns.find(p => p.absence_type === 'homeoffice'), [patterns]);

  // Calendar days
  const calendarDays = useMemo(() => {
    const firstDay = new Date(calYear, calMonth, 1);
    const lastDay = new Date(calYear, calMonth + 1, 0);
    const startDow = firstDay.getDay() === 0 ? 6 : firstDay.getDay() - 1;
    const days: { date: Date; inMonth: boolean; isToday: boolean; types: AbsenceType[] }[] = [];

    for (let i = 0; i < startDow; i++) {
      days.push({ date: new Date(calYear, calMonth, 1 - startDow + i), inMonth: false, isToday: false, types: [] });
    }
    for (let d = 1; d <= lastDay.getDate(); d++) {
      const date = new Date(calYear, calMonth, d);
      const dow = date.getDay() === 0 ? 6 : date.getDay() - 1;
      const types: AbsenceType[] = [];
      for (const e of entries) {
        if (isDateInRange(date, e.start_date, e.end_date)) {
          const at = e.absence_type as AbsenceType;
          if (!types.includes(at)) types.push(at);
        }
      }
      if (dow < 5 && hoPattern && hoPattern.weekdays.includes(dow) && !types.includes('homeoffice')) {
        types.push('homeoffice');
      }
      days.push({ date, inMonth: true, isToday: isSameDay(date, today), types });
    }
    while (days.length % 7 !== 0) {
      days.push({ date: new Date(calYear, calMonth + 1, days.length - startDow - lastDay.getDate() + 1), inMonth: false, isToday: false, types: [] });
    }
    return days;
  }, [entries, hoPattern, calMonth, calYear, today]);

  const calMonthLabel = new Date(calYear, calMonth, 1).toLocaleDateString(undefined, { month: 'long', year: 'numeric' });

  function prevMonth() { if (calMonth === 0) { setCalMonth(11); setCalYear(y => y - 1); } else setCalMonth(m => m - 1); }
  function nextMonth() { if (calMonth === 11) { setCalMonth(0); setCalYear(y => y + 1); } else setCalMonth(m => m + 1); }

  async function deleteEntry(id: string) {
    const res = await api.deleteAbsence(id);
    if (res.success) {
      setEntries(prev => prev.filter(e => e.id !== id));
      toast.success(t('absences.deleted', 'Abwesenheit gel\u00f6scht'));
    }
  }

  async function handleAdd(type: AbsenceType, startDate: string, endDate: string, note: string) {
    const res = await api.createAbsence(type, startDate, endDate, note || undefined);
    if (res.success && res.data) {
      setEntries(prev => [...prev, res.data!].sort((a, b) => a.start_date.localeCompare(b.start_date)));
      toast.success(t('absences.added', 'Abwesenheit eingetragen'));
      setShowAdd(false);
    } else {
      toast.error(res.error?.message || t('common.error'));
    }
  }

  async function handlePatternSave(weekdays: number[]) {
    const res = await api.setAbsencePattern('homeoffice', weekdays);
    if (res.success && res.data) {
      setPatterns(prev => [...prev.filter(p => p.absence_type !== 'homeoffice'), res.data!]);
      toast.success(t('absences.patternUpdated', 'Muster aktualisiert'));
    }
  }

  const WEEKDAYS = [
    t('homeoffice.weekdaysShort.mon'), t('homeoffice.weekdaysShort.tue'), t('homeoffice.weekdaysShort.wed'),
    t('homeoffice.weekdaysShort.thu'), t('homeoffice.weekdaysShort.fri'), t('homeoffice.weekdaysShort.sat'),
    t('homeoffice.weekdaysShort.sun'),
  ];

  if (loading) return (
    <div className="space-y-6">
      <div className="h-8 w-64 skeleton rounded-xl" />
      <div className="h-80 skeleton rounded-2xl" />
    </div>
  );

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white flex items-center gap-3">
            <Calendar weight="fill" className="w-7 h-7 text-primary-600" />
            {t('absences.title', 'Abwesenheiten')}
          </h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">{t('absences.subtitle', 'Homeoffice, Urlaub & mehr verwalten')}</p>
        </div>
        <button onClick={() => setShowAdd(true)} className="btn btn-primary">
          <Plus weight="bold" className="w-4 h-4" /> {t('absences.addAbsence', 'Eintragen')}
        </button>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-5 gap-6">
        {/* Calendar */}
        <div className="lg:col-span-3 bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-6">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold text-surface-900 dark:text-white">{calMonthLabel}</h2>
            <div className="flex items-center gap-1">
              <button onClick={prevMonth} aria-label={t('absences.previousMonth', 'Previous month')} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 min-w-[44px] min-h-[44px] flex items-center justify-center"><CaretLeft weight="bold" className="w-5 h-5 text-surface-600 dark:text-surface-400" aria-hidden="true" /></button>
              <button onClick={() => { setCalMonth(today.getMonth()); setCalYear(today.getFullYear()); }} aria-label={t('absences.goToToday')} className="px-3 py-2 text-xs font-medium text-surface-500 hover:text-surface-700 dark:hover:text-surface-300 min-h-[44px] flex items-center">{t('absences.today')}</button>
              <button onClick={nextMonth} aria-label={t('absences.nextMonth', 'Next month')} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 min-w-[44px] min-h-[44px] flex items-center justify-center"><CaretRight weight="bold" className="w-5 h-5 text-surface-600 dark:text-surface-400" aria-hidden="true" /></button>
            </div>
          </div>
          <div className="grid grid-cols-7 gap-1">
            {WEEKDAYS.map(d => <div key={d} className="text-center text-xs font-semibold text-surface-500 dark:text-surface-400 py-2">{d}</div>)}
            {calendarDays.map((day, i) => {
              const mainType = day.types[0];
              const cfg = mainType ? ABSENCE_CONFIG[mainType] : null;
              const isWeekend = day.date.getDay() === 0 || day.date.getDay() === 6;
              return (
                <div key={i} className={`relative flex flex-col items-center justify-center rounded-lg text-sm font-medium min-h-[40px] transition-all ${
                  !day.inMonth ? 'text-surface-300 dark:text-surface-700' :
                  day.isToday ? 'ring-2 ring-primary-500 ring-offset-1 dark:ring-offset-surface-900' : ''
                } ${cfg ? `${cfg.bg} ${cfg.color.split(' ')[0]}` : isWeekend && day.inMonth ? 'text-surface-400' : day.inMonth ? 'text-surface-700 dark:text-surface-300 hover:bg-surface-50 dark:hover:bg-surface-800' : ''}`}>
                  {day.date.getDate()}
                  {day.types.length > 0 && (
                    <div className="flex gap-0.5 mt-0.5">
                      {day.types.slice(0, 3).map((at, j) => <div key={j} className={`w-1.5 h-1.5 rounded-full ${ABSENCE_CONFIG[at].dot}`} />)}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
          {/* Legend */}
          <div className="flex flex-wrap gap-3 mt-4 pt-4 border-t border-surface-100 dark:border-surface-800 text-xs text-surface-500">
            {(Object.entries(ABSENCE_CONFIG) as [AbsenceType, typeof ABSENCE_CONFIG.homeoffice][]).map(([type, cfg]) => (
              <div key={type} className="flex items-center gap-1.5">
                <div className={`w-3 h-3 rounded-sm ${cfg.bg}`} />
                <span>{t(`absences.types.${type}`, type)}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Sidebar */}
        <div className="lg:col-span-2 space-y-6">
          {/* Homeoffice pattern */}
          <div className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-6">
            <button onClick={() => setShowPattern(!showPattern)} aria-expanded={showPattern} className="flex items-center justify-between w-full text-left">
              <h3 className="text-base font-semibold text-surface-900 dark:text-white flex items-center gap-2">
                <House weight="fill" className="w-5 h-5 text-primary-600" />
                {t('absences.weeklyPattern', 'Homeoffice-Muster')}
              </h3>
              <CaretRight weight="bold" className={`w-4 h-4 text-surface-400 transition-transform ${showPattern ? 'rotate-90' : ''}`} />
            </button>
            <AnimatePresence>
              {showPattern && (
                <motion.div initial={{ height: 0, opacity: 0 }} animate={{ height: 'auto', opacity: 1 }} exit={{ height: 0, opacity: 0 }} className="overflow-hidden">
                  <p className="text-sm text-surface-500 dark:text-surface-400 mt-3 mb-3">{t('absences.patternDesc', 'W\u00e4hle deine festen Homeoffice-Tage')}</p>
                  <div className="grid grid-cols-5 gap-2">
                    {[t('homeoffice.weekdaysShort.mon'), t('homeoffice.weekdaysShort.tue'), t('homeoffice.weekdaysShort.wed'), t('homeoffice.weekdaysShort.thu'), t('homeoffice.weekdaysShort.fri')].map((name, i) => {
                      const active = hoPattern?.weekdays.includes(i);
                      return (
                        <button key={i} onClick={() => {
                          const current = hoPattern?.weekdays || [];
                          const next = current.includes(i) ? current.filter(d => d !== i) : [...current, i].sort();
                          handlePatternSave(next);
                        }}
                          aria-pressed={!!active}
                          aria-label={`${name} ${active ? t('absences.patternActive', 'active') : t('absences.patternInactive', 'inactive')}`}
                          className={`flex flex-col items-center gap-1 py-3 rounded-xl border-2 transition-all font-medium ${
                            active ? 'bg-primary-100 dark:bg-primary-900/40 border-primary-400 dark:border-primary-600 text-primary-700 dark:text-primary-300' :
                            'bg-surface-50 dark:bg-surface-800 border-surface-200 dark:border-surface-700 text-surface-500'
                          }`}
                        >
                          <span className="text-sm">{name}</span>
                          {active && <House weight="fill" className="w-4 h-4" />}
                        </button>
                      );
                    })}
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </div>

          {/* Upcoming entries */}
          <div className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-6">
            <h3 className="text-base font-semibold text-surface-900 dark:text-white mb-3 flex items-center gap-2">
              <CalendarCheck weight="fill" className="w-5 h-5 text-emerald-600" />
              {t('absences.upcoming', 'Anstehend')}
            </h3>
            <div className="space-y-2 max-h-72 overflow-y-auto">
              {entries.filter(e => e.end_date >= todayStr).sort((a, b) => a.start_date.localeCompare(b.start_date)).map(entry => {
                const cfg = ABSENCE_CONFIG[entry.absence_type as AbsenceType] || ABSENCE_CONFIG.other;
                const Icon = cfg.icon;
                return (
                  <div key={entry.id} className="flex items-center justify-between p-3 bg-surface-50 dark:bg-surface-800/50 rounded-xl">
                    <div className="flex items-center gap-3 min-w-0">
                      <Icon weight="fill" className={`w-5 h-5 flex-shrink-0 ${cfg.color}`} />
                      <div className="min-w-0">
                        <span className="text-sm font-medium text-surface-900 dark:text-white block truncate">
                          {new Date(entry.start_date + 'T00:00:00').toLocaleDateString(undefined, { day: 'numeric', month: 'short' })}
                          {entry.start_date !== entry.end_date && <> &ndash; {new Date(entry.end_date + 'T00:00:00').toLocaleDateString(undefined, { day: 'numeric', month: 'short' })}</>}
                        </span>
                        <span className={`text-xs ${cfg.color}`}>{t(`absences.types.${entry.absence_type}`, entry.absence_type)}</span>
                      </div>
                    </div>
                    <button onClick={() => deleteEntry(entry.id)} aria-label={t('absences.deleteEntry', 'Delete absence entry')} className="p-2 rounded-lg text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20">
                      <Trash weight="bold" className="w-4 h-4" aria-hidden="true" />
                    </button>
                  </div>
                );
              })}
              {entries.filter(e => e.end_date >= todayStr).length === 0 && (
                <div className="text-center py-6">
                  <motion.div animate={{ y: [0, -4, 0] }} transition={{ duration: 2, repeat: Infinity, ease: 'easeInOut' }}>
                    <CalendarCheck weight="light" className="w-12 h-12 text-surface-200 dark:text-surface-700 mx-auto" />
                  </motion.div>
                  <p className="text-sm text-surface-500 dark:text-surface-400 mt-3">{t('absences.noEntries', 'Keine Eintr\u00e4ge')}</p>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Add absence modal */}
      <AnimatePresence>
        {showAdd && <AddAbsenceModal onClose={() => setShowAdd(false)} onAdd={handleAdd} t={t} />}
      </AnimatePresence>
    </motion.div>
  );
}

function AddAbsenceModal({ onClose, onAdd, t }: {
  onClose: () => void;
  onAdd: (type: AbsenceType, start: string, end: string, note: string) => void;
  t: (key: string, fallback?: string) => string;
}) {
  const [type, setType] = useState<AbsenceType>('homeoffice');
  const [startDate, setStartDate] = useState('');
  const [endDate, setEndDate] = useState('');
  const [note, setNote] = useState('');

  const todayStr = new Date().toISOString().slice(0, 10);

  return (
    <>
      <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="fixed inset-0 bg-black/40 z-50" onClick={onClose} aria-hidden="true" />
      <motion.div
        initial={{ opacity: 0, scale: 0.95, y: 20 }}
        animate={{ opacity: 1, scale: 1, y: 0 }}
        exit={{ opacity: 0, scale: 0.95 }}
        role="dialog"
        aria-modal="true"
        aria-label={t('absences.addAbsence', 'Abwesenheit eintragen')}
        className="fixed z-50 top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-full max-w-md glass-modal shadow-2xl p-6"
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-surface-900 dark:text-white">{t('absences.addAbsence', 'Abwesenheit eintragen')}</h2>
          <button onClick={onClose} aria-label={t('common.cancel', 'Close')} className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800"><X weight="bold" className="w-5 h-5 text-surface-500" aria-hidden="true" /></button>
        </div>

        {/* Type pills */}
        <div className="grid grid-cols-3 sm:grid-cols-5 gap-2 mb-4">
          {(Object.entries(ABSENCE_CONFIG) as [AbsenceType, typeof ABSENCE_CONFIG.homeoffice][]).map(([at, cfg]) => {
            const Icon = cfg.icon;
            const active = type === at;
            return (
              <button key={at} onClick={() => setType(at)}
                className={`flex flex-col items-center gap-1 py-3 px-2 rounded-xl border-2 transition-all ${
                  active ? `${cfg.bg} border-current ${cfg.color}` : 'bg-surface-50 dark:bg-surface-800 border-surface-200 dark:border-surface-700 text-surface-500'
                }`}
              >
                <Icon weight={active ? 'fill' : 'regular'} className="w-5 h-5" />
                <span className="text-xs font-medium truncate w-full text-center">{t(`absences.types.${at}`, at)}</span>
              </button>
            );
          })}
        </div>

        {/* Quick buttons */}
        <div className="flex gap-2 mb-4">
          <button onClick={() => { setStartDate(todayStr); setEndDate(todayStr); }} className="btn btn-secondary text-sm flex-1">{t('absences.quickToday', 'Heute')}</button>
        </div>

        {/* Date range */}
        <div className="grid grid-cols-2 gap-3 mb-4">
          <div>
            <label htmlFor="absence-start" className="text-xs text-surface-500 mb-1 block">{t('absences.startDate', 'Von')}</label>
            <input id="absence-start" type="date" value={startDate} onChange={e => { setStartDate(e.target.value); if (!endDate || e.target.value > endDate) setEndDate(e.target.value); }} className="input w-full" />
          </div>
          <div>
            <label htmlFor="absence-end" className="text-xs text-surface-500 mb-1 block">{t('absences.endDate', 'Bis')}</label>
            <input id="absence-end" type="date" value={endDate} onChange={e => setEndDate(e.target.value)} className="input w-full" min={startDate} />
          </div>
        </div>

        <label htmlFor="absence-note" className="sr-only">{t('absences.notePlaceholder', 'Notiz (optional)')}</label>
        <input id="absence-note" type="text" placeholder={t('absences.notePlaceholder', 'Notiz (optional)')} value={note} onChange={e => setNote(e.target.value)} className="input w-full mb-4" />

        <button onClick={() => {
          if (!startDate || !endDate || endDate < startDate) return;
          onAdd(type, startDate, endDate, note);
        }} disabled={!startDate || !endDate || endDate < startDate} className="btn btn-primary w-full">
          <Plus weight="bold" className="w-4 h-4" /> {t('absences.addBtn', 'Eintragen')}
        </button>
      </motion.div>
    </>
  );
}
