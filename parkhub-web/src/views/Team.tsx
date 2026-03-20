import { useEffect, useState, useMemo } from 'react';
import { motion } from 'framer-motion';
import { Users } from '@phosphor-icons/react';
import { api, type TeamAbsenceEntry } from '../api/client';
import { useTranslation } from 'react-i18next';
import { ABSENCE_CONFIG, type AbsenceType } from '../constants/absenceConfig';

export function TeamPage() {
  const { t } = useTranslation();
  const [entries, setEntries] = useState<TeamAbsenceEntry[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.teamAbsences().then(res => {
      if (res.success && res.data) setEntries(res.data);
    }).finally(() => setLoading(false));
  }, []);

  const todayStr = useMemo(() => new Date().toISOString().slice(0, 10), []);

  // Group: today and upcoming
  const todayEntries = useMemo(() =>
    entries.filter(e => e.start_date <= todayStr && e.end_date >= todayStr),
  [entries, todayStr]);

  const upcomingEntries = useMemo(() =>
    entries.filter(e => e.start_date > todayStr).sort((a, b) => a.start_date.localeCompare(b.start_date)),
  [entries, todayStr]);

  // Group by user
  function groupByUser(list: TeamAbsenceEntry[]) {
    const groups: Record<string, TeamAbsenceEntry[]> = {};
    for (const e of list) {
      if (!groups[e.user_name]) groups[e.user_name] = [];
      groups[e.user_name].push(e);
    }
    return groups;
  }

  const todayByUser = useMemo(() => groupByUser(todayEntries), [todayEntries]);
  const upcomingByUser = useMemo(() => groupByUser(upcomingEntries), [upcomingEntries]);

  if (loading) return (
    <div className="space-y-4">
      <div className="h-8 w-48 skeleton rounded-lg" />
      {[1, 2, 3].map(i => <div key={i} className="h-20 skeleton rounded-2xl" />)}
    </div>
  );

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold text-surface-900 dark:text-white flex items-center gap-3">
          <Users weight="fill" className="w-7 h-7 text-primary-600" />
          {t('team.title', 'Team')}
        </h1>
        <p className="text-surface-500 dark:text-surface-400 mt-1">{t('team.subtitle', 'Abwesenheiten im Team')}</p>
      </div>

      {/* Today */}
      <section>
        <h2 className="text-sm font-semibold uppercase tracking-wider text-surface-500 dark:text-surface-400 mb-3">
          {t('team.today', 'Heute abwesend')} ({todayEntries.length})
        </h2>
        {Object.keys(todayByUser).length === 0 ? (
          <div className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-8 text-center">
            <Users weight="light" className="w-12 h-12 text-surface-200 dark:text-surface-700 mx-auto mb-3" />
            <p className="text-surface-500 dark:text-surface-400">{t('team.noAbsencesToday', 'Heute keine Abwesenheiten')}</p>
          </div>
        ) : (
          <div>
            {Object.entries(todayByUser).map(([name, items]) => (
              <TeamMemberCard key={name} name={name} entries={items} t={t} />
            ))}
          </div>
        )}
      </section>

      {/* Upcoming */}
      <section>
        <h2 className="text-sm font-semibold uppercase tracking-wider text-surface-500 dark:text-surface-400 mb-3">
          {t('team.upcoming', 'Kommende Abwesenheiten')} ({upcomingEntries.length})
        </h2>
        {Object.keys(upcomingByUser).length === 0 ? (
          <div className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-8 text-center">
            <p className="text-surface-500 dark:text-surface-400">{t('team.noUpcoming', 'Keine anstehenden Abwesenheiten')}</p>
          </div>
        ) : (
          <div>
            {Object.entries(upcomingByUser).map(([name, items]) => (
              <TeamMemberCard key={name} name={name} entries={items} t={t} />
            ))}
          </div>
        )}
      </section>
    </motion.div>
  );
}

function TeamMemberCard({ name, entries, t }: { name: string; entries: TeamAbsenceEntry[]; t: (k: string, f?: string) => string }) {
  return (
    <div className="py-3 border-b border-surface-100 dark:border-surface-800 last:border-b-0">
      <div className="flex items-center gap-3 mb-2">
        <div className="w-8 h-8 bg-surface-200 dark:bg-surface-700 rounded-full flex items-center justify-center text-sm font-semibold text-surface-700 dark:text-surface-300">
          {name.charAt(0).toUpperCase()}
        </div>
        <span className="text-sm font-medium text-surface-900 dark:text-white">{name}</span>
      </div>
      <div className="ml-11 space-y-1">
        {entries.map((entry, i) => {
          const cfg = ABSENCE_CONFIG[entry.absence_type as AbsenceType] || ABSENCE_CONFIG.other;
          return (
            <div key={i} className="flex items-center gap-2">
              <div className={`w-1 h-1 rounded-full ${cfg.dot}`} />
              <span className={`text-sm ${cfg.color}`}>{t(`absences.types.${entry.absence_type}`, entry.absence_type)}</span>
              <span className="text-sm text-surface-400 ml-auto">
                {new Date(entry.start_date + 'T00:00:00').toLocaleDateString(undefined, { day: 'numeric', month: 'short' })}
                {entry.start_date !== entry.end_date && <> &ndash; {new Date(entry.end_date + 'T00:00:00').toLocaleDateString(undefined, { day: 'numeric', month: 'short' })}</>}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
