import { useEffect, useState, useMemo } from 'react';
import { motion } from 'framer-motion';
import { Trophy, Medal, Lightning, Star } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import { getInMemoryToken } from '../api/client';
import { staggerSlow, fadeUp } from '../constants/animations';
import { useTheme } from '../context/ThemeContext';

interface TeamMember {
  id: string;
  username: string;
  name: string;
  role: string;
}

interface AdminStats {
  total_bookings: number;
  active_bookings: number;
  total_users: number;
  ev_bookings?: number;
  morning_bookings?: number;
  swap_requests_accepted?: number;
  no_shows?: number;
  bookings_by_user?: Record<string, UserBookingStats>;
}

interface UserBookingStats {
  total: number;
  this_month: number;
  ev_count: number;
  morning_count: number;
  swaps_accepted: number;
  no_shows: number;
  avg_duration_hours: number;
}

interface LeaderboardEntry {
  id: string;
  name: string;
  username: string;
  ecoScore: number;
  bookingsThisMonth: number;
  evPercentage: number;
  badges: Badge[];
  noShows: number;
}

interface Badge {
  key: string;
  label: string;
  color: string;
}

function authHeaders(): Record<string, string> {
  const token = getInMemoryToken();
  return {
    Accept: 'application/json',
    'X-Requested-With': 'XMLHttpRequest',
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
  };
}

function computeEcoScore(stats: UserBookingStats): number {
  const bookingScore = Math.min(stats.this_month * 5, 40);
  const evScore = stats.total > 0 ? (stats.ev_count / stats.total) * 30 : 0;
  const durationScore = Math.min(stats.avg_duration_hours * 3, 20);
  const reliabilityScore = stats.no_shows === 0 ? 10 : Math.max(0, 10 - stats.no_shows * 3);
  return Math.round(bookingScore + evScore + durationScore + reliabilityScore);
}

function computeBadges(stats: UserBookingStats, t: (k: string, f?: string) => string): Badge[] {
  const badges: Badge[] = [];
  if (stats.ev_count > 0) {
    badges.push({ key: 'ev', label: t('leaderboard.badgeEv', 'EV Driver'), color: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400' });
  }
  if (stats.morning_count > 0) {
    badges.push({ key: 'early', label: t('leaderboard.badgeEarly', 'Early Bird'), color: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400' });
  }
  if (stats.swaps_accepted > 0) {
    badges.push({ key: 'team', label: t('leaderboard.badgeTeam', 'Team Player'), color: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400' });
  }
  if (stats.this_month >= 10) {
    badges.push({ key: 'frequent', label: t('leaderboard.badgeFrequent', 'Frequent Parker'), color: 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400' });
  }
  return badges;
}

const MEDAL_COLORS = ['text-yellow-500', 'text-surface-400', 'text-amber-700'];

export function TeamLeaderboardPage() {
  const { t } = useTranslation();
  const { designTheme } = useTheme();
  const surfaceVariant = designTheme === 'void' ? 'void' : 'marble';
  const [members, setMembers] = useState<TeamMember[]>([]);
  const [adminStats, setAdminStats] = useState<AdminStats | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([
      fetch('/api/v1/team', { headers: authHeaders(), credentials: 'include' }).then(r => r.json()),
      fetch('/api/v1/admin/stats', { headers: authHeaders(), credentials: 'include' }).then(r => r.json()),
    ])
      .then(([teamRes, statsRes]) => {
        if (teamRes?.success || teamRes?.data) setMembers(teamRes.data || []);
        if (statsRes?.success || statsRes?.data) setAdminStats(statsRes.data || null);
      })
      .catch(() => { /* handled by empty state */ })
      .finally(() => setLoading(false));
  }, []);

  const leaderboard = useMemo<LeaderboardEntry[]>(() => {
    if (!members.length) return [];
    const byUser = adminStats?.bookings_by_user || {};

    return members
      .map(m => {
        const stats: UserBookingStats = byUser[m.id] || {
          total: 0, this_month: 0, ev_count: 0, morning_count: 0,
          swaps_accepted: 0, no_shows: 0, avg_duration_hours: 0,
        };
        return {
          id: m.id,
          name: m.name || m.username,
          username: m.username,
          ecoScore: computeEcoScore(stats),
          bookingsThisMonth: stats.this_month,
          evPercentage: stats.total > 0 ? Math.round((stats.ev_count / stats.total) * 100) : 0,
          badges: computeBadges(stats, t),
          noShows: stats.no_shows,
        };
      })
      .sort((a, b) => b.ecoScore - a.ecoScore);
  }, [members, adminStats, t]);

  const mostActive = useMemo(() =>
    leaderboard.length > 0 ? [...leaderboard].sort((a, b) => b.bookingsThisMonth - a.bookingsThisMonth)[0] : null,
  [leaderboard]);

  const greenest = useMemo(() =>
    leaderboard.length > 0 ? [...leaderboard].sort((a, b) => b.evPercentage - a.evPercentage)[0] : null,
  [leaderboard]);

  const mostReliable = useMemo(() =>
    leaderboard.length > 0 ? [...leaderboard].sort((a, b) => a.noShows - b.noShows || b.ecoScore - a.ecoScore)[0] : null,
  [leaderboard]);

  if (loading) {
    return (
      <div className="space-y-4" data-testid="loading">
        <div className="h-8 w-56 skeleton rounded-lg" />
        <div className="grid grid-cols-3 gap-4">
          {[1, 2, 3].map(i => <div key={i} className="h-24 skeleton rounded-2xl" />)}
        </div>
        {[1, 2, 3, 4].map(i => <div key={i} className="h-20 skeleton rounded-2xl" />)}
      </div>
    );
  }

  if (!leaderboard.length) {
    return (
      <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} className="space-y-6" data-testid="leaderboard-page" data-surface={surfaceVariant}>
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white flex items-center gap-3">
            <Trophy weight="fill" className="w-7 h-7 text-yellow-500" />
            {t('leaderboard.title', 'Team Leaderboard')}
          </h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">{t('leaderboard.subtitle', 'See how your team is doing')}</p>
        </div>
        <div className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-12 text-center" data-testid="empty-state">
          <Trophy weight="light" className="w-12 h-12 text-surface-200 dark:text-surface-700 mx-auto mb-3" />
          <p className="text-surface-500 dark:text-surface-400">{t('leaderboard.empty', 'No team data available')}</p>
        </div>
      </motion.div>
    );
  }

  return (
    <motion.div
      variants={staggerSlow}
      initial="hidden"
      animate="show"
      className="space-y-6"
      data-testid="leaderboard-page"
      data-surface={surfaceVariant}
    >
      {/* Header */}
      <motion.div variants={fadeUp}>
        <h1 className="text-2xl font-bold text-surface-900 dark:text-white flex items-center gap-3">
          <Trophy weight="fill" className="w-7 h-7 text-yellow-500" />
          {t('leaderboard.title', 'Team Leaderboard')}
        </h1>
        <p className="text-surface-500 dark:text-surface-400 mt-1">{t('leaderboard.subtitle', 'See how your team is doing')}</p>
      </motion.div>

      {/* Top 3 stat cards */}
      <motion.div variants={fadeUp} className="grid grid-cols-1 sm:grid-cols-3 gap-4" data-testid="highlight-cards">
        {mostActive && (
          <div className="bg-white dark:bg-surface-900 rounded-xl p-5 border border-surface-200 dark:border-surface-800 shadow-sm">
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 rounded-lg bg-primary-50 dark:bg-primary-950/30 flex items-center justify-center">
                <Star weight="fill" className="w-5 h-5 text-primary-600 dark:text-primary-400" />
              </div>
              <span className="text-sm text-surface-500 dark:text-surface-400">{t('leaderboard.mostActive', 'Most Active')}</span>
            </div>
            <div className="text-lg font-bold text-surface-900 dark:text-white" data-testid="most-active">{mostActive.name}</div>
            <div className="text-sm text-surface-500">{mostActive.bookingsThisMonth} {t('leaderboard.bookings', 'bookings')}</div>
          </div>
        )}
        {greenest && (
          <div className="bg-white dark:bg-surface-900 rounded-xl p-5 border border-surface-200 dark:border-surface-800 shadow-sm">
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 rounded-lg bg-green-50 dark:bg-green-950/30 flex items-center justify-center">
                <Lightning weight="fill" className="w-5 h-5 text-green-600 dark:text-green-400" />
              </div>
              <span className="text-sm text-surface-500 dark:text-surface-400">{t('leaderboard.greenest', 'Greenest (EV)')}</span>
            </div>
            <div className="text-lg font-bold text-surface-900 dark:text-white" data-testid="greenest">{greenest.name}</div>
            <div className="text-sm text-surface-500">{greenest.evPercentage}% EV</div>
          </div>
        )}
        {mostReliable && (
          <div className="bg-white dark:bg-surface-900 rounded-xl p-5 border border-surface-200 dark:border-surface-800 shadow-sm">
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 rounded-lg bg-amber-50 dark:bg-amber-950/30 flex items-center justify-center">
                <Medal weight="fill" className="w-5 h-5 text-amber-600 dark:text-amber-400" />
              </div>
              <span className="text-sm text-surface-500 dark:text-surface-400">{t('leaderboard.mostReliable', 'Most Reliable')}</span>
            </div>
            <div className="text-lg font-bold text-surface-900 dark:text-white" data-testid="most-reliable">{mostReliable.name}</div>
            <div className="text-sm text-surface-500">{mostReliable.noShows} {t('leaderboard.noShows', 'no-shows')}</div>
          </div>
        )}
      </motion.div>

      {/* Podium — top 3 */}
      {leaderboard.length >= 3 && (
        <motion.div variants={fadeUp} className="glass-card p-6" data-testid="podium">
          <div className="flex items-end justify-center gap-4 h-40">
            {/* 2nd place */}
            <PodiumSlot entry={leaderboard[1]} rank={2} height="h-24" t={t} />
            {/* 1st place */}
            <PodiumSlot entry={leaderboard[0]} rank={1} height="h-32" t={t} />
            {/* 3rd place */}
            <PodiumSlot entry={leaderboard[2]} rank={3} height="h-20" t={t} />
          </div>
        </motion.div>
      )}

      {/* Leaderboard rows */}
      <div className="space-y-2" data-testid="leaderboard-rows">
        {leaderboard.map((entry, idx) => (
          <motion.div
            key={entry.id}
            variants={fadeUp}
            className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-4 flex items-center gap-4"
            data-testid="leaderboard-row"
          >
            {/* Rank */}
            <div className="w-8 h-8 flex items-center justify-center flex-shrink-0">
              {idx < 3 ? (
                <Medal weight="fill" className={`w-6 h-6 ${MEDAL_COLORS[idx]}`} data-testid={`medal-${idx + 1}`} />
              ) : (
                <span className="text-sm font-bold text-surface-400">{idx + 1}</span>
              )}
            </div>

            {/* Avatar + name */}
            <div className="flex items-center gap-3 flex-1 min-w-0">
              <div className="w-9 h-9 bg-surface-200 dark:bg-surface-700 rounded-full flex items-center justify-center text-sm font-semibold text-surface-700 dark:text-surface-300 flex-shrink-0">
                {entry.name.charAt(0).toUpperCase()}
              </div>
              <div className="min-w-0">
                <p className="font-medium text-surface-900 dark:text-white truncate">{entry.name}</p>
                <div className="flex flex-wrap gap-1 mt-0.5">
                  {entry.badges.map(b => (
                    <span key={b.key} className={`px-1.5 py-0.5 rounded-full text-[10px] font-medium ${b.color}`} data-testid={`badge-${b.key}`}>
                      {b.label}
                    </span>
                  ))}
                </div>
              </div>
            </div>

            {/* Eco score */}
            <div className="text-right flex-shrink-0">
              <div className="text-lg font-bold text-surface-900 dark:text-white" style={{ fontVariantNumeric: 'tabular-nums' }}>
                {entry.ecoScore}
              </div>
              <div className="text-[10px] text-surface-400 uppercase tracking-wider">{t('leaderboard.ecoScore', 'Eco Score')}</div>
            </div>

            {/* Bookings bar */}
            <div className="hidden sm:flex items-center gap-2 w-32 flex-shrink-0">
              <div className="flex-1 h-2 bg-surface-100 dark:bg-surface-800 rounded-full overflow-hidden">
                <div
                  className="h-full bg-primary-500 rounded-full transition-all"
                  style={{ width: `${Math.min(100, (entry.bookingsThisMonth / Math.max(...leaderboard.map(e => e.bookingsThisMonth), 1)) * 100)}%` }}
                />
              </div>
              <span className="text-xs text-surface-500 tabular-nums w-6 text-right">{entry.bookingsThisMonth}</span>
            </div>
          </motion.div>
        ))}
      </div>
    </motion.div>
  );
}

function PodiumSlot({ entry, rank, height, t }: {
  entry: LeaderboardEntry;
  rank: number;
  height: string;
  t: (k: string, f?: string) => string;
}) {
  return (
    <div className="flex flex-col items-center w-24" data-testid={`podium-${rank}`}>
      <div className="w-10 h-10 bg-surface-200 dark:bg-surface-700 rounded-full flex items-center justify-center text-sm font-semibold text-surface-700 dark:text-surface-300 mb-1">
        {entry.name.charAt(0).toUpperCase()}
      </div>
      <span className="text-xs font-medium text-surface-900 dark:text-white text-center truncate w-full">{entry.name}</span>
      <span className="text-[10px] text-surface-400">{entry.ecoScore} pts</span>
      <div className={`${height} w-full mt-2 rounded-t-lg flex items-start justify-center pt-2 ${
        rank === 1 ? 'bg-yellow-100 dark:bg-yellow-900/30' :
        rank === 2 ? 'bg-surface-100 dark:bg-surface-800' :
        'bg-amber-100 dark:bg-amber-900/20'
      }`}>
        <span className="text-lg font-bold text-surface-600 dark:text-surface-300">{rank}</span>
      </div>
    </div>
  );
}
