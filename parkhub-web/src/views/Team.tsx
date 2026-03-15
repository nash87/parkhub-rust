import { useEffect, useState, useMemo } from 'react';
import { motion } from 'framer-motion';
import {
  House, Airplane, FirstAidKit, Briefcase, Users, NoteBlank,
} from '@phosphor-icons/react';
import { api, type TeamAbsenceEntry } from '../api/client';
import { useTranslation } from 'react-i18next';

type AbsenceType = 'homeoffice' | 'vacation' | 'sick' | 'business_trip' | 'other';

const ABSENCE_CONFIG: Record<AbsenceType, { icon: typeof House; color: string; bg: string; dot: string }> = {
  homeoffice: { icon: House, color: 'text-primary-600 dark:text-primary-400', bg: 'bg-primary-100 dark:bg-primary-900/30', dot: 'bg-primary-500' },
  vacation: { icon: Airplane, color: 'text-orange-600 dark:text-orange-400', bg: 'bg-orange-100 dark:bg-orange-900/30', dot: 'bg-orange-500' },
  sick: { icon: FirstAidKit, color: 'text-red-600 dark:text-red-400', bg: 'bg-red-100 dark:bg-red-900/30', dot: 'bg-red-500' },
  business_trip: { icon: Briefcase, color: 'text-purple-600 dark:text-purple-400', bg: 'bg-purple-100 dark:bg-purple-900/30', dot: 'bg-purple-500' },
  other: { icon: NoteBlank, color: 'text-surface-600 dark:text-surface-400', bg: 'bg-surface-100 dark:bg-surface-800/50', dot: 'bg-surface-500' },
};

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

  const todayByUser = groupByUser(todayEntries);
  const upcomingByUser = groupByUser(upcomingEntries);

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
          <div className="space-y-3">
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
          <div className="space-y-3">
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
    <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-4">
      <div className="flex items-center gap-3 mb-3">
        <div className="w-10 h-10 bg-primary-100 dark:bg-primary-900/30 rounded-full flex items-center justify-center text-sm font-bold text-primary-700 dark:text-primary-300">
          {name.charAt(0).toUpperCase()}
        </div>
        <span className="font-medium text-surface-900 dark:text-white">{name}</span>
      </div>
      <div className="space-y-2">
        {entries.map((entry, i) => {
          const cfg = ABSENCE_CONFIG[entry.absence_type as AbsenceType] || ABSENCE_CONFIG.other;
          return (
            <div key={i} className={`flex items-center gap-2 px-3 py-2 rounded-lg ${cfg.bg}`}>
              <div className={`w-2 h-2 rounded-full ${cfg.dot}`} />
              <span className={`text-sm font-medium ${cfg.color}`}>{t(`absences.types.${entry.absence_type}`, entry.absence_type)}</span>
              <span className="text-sm text-surface-600 dark:text-surface-400 ml-auto">
                {new Date(entry.start_date + 'T00:00:00').toLocaleDateString(undefined, { day: 'numeric', month: 'short' })}
                {entry.start_date !== entry.end_date && <> &ndash; {new Date(entry.end_date + 'T00:00:00').toLocaleDateString(undefined, { day: 'numeric', month: 'short' })}</>}
              </span>
            </div>
          );
        })}
      </div>
    </motion.div>
  );
}
