import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import {
  ChartBar, SpinnerGap, Users, Buildings, CalendarCheck, Lightning,
} from '@phosphor-icons/react';
import { api, type AdminStats } from '../api/client';

function StatCard({ icon: Icon, label, value, color }: {
  icon: any;
  label: string;
  value: number;
  color: string;
}) {
  const colors: Record<string, string> = {
    primary: 'bg-primary-100 dark:bg-primary-900/30 text-primary-600 dark:text-primary-400',
    accent: 'bg-accent-100 dark:bg-accent-900/30 text-accent-600 dark:text-accent-400',
    info: 'bg-blue-100 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400',
    success: 'bg-emerald-100 dark:bg-emerald-900/30 text-emerald-600 dark:text-emerald-400',
  };

  const valueColors: Record<string, string> = {
    primary: 'text-primary-600 dark:text-primary-400',
    accent: 'text-accent-600 dark:text-accent-400',
    info: 'text-blue-600 dark:text-blue-400',
    success: 'text-emerald-600 dark:text-emerald-400',
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      className="stat-card"
    >
      <div className="flex items-start justify-between">
        <div>
          <p className="text-sm font-medium text-surface-500 dark:text-surface-400">{label}</p>
          <p className={`mt-2 stat-value ${valueColors[color]}`}>{value}</p>
        </div>
        <div className={`w-10 h-10 rounded-xl flex items-center justify-center ${colors[color]}`}>
          <Icon weight="fill" className="w-5 h-5" />
        </div>
      </div>
    </motion.div>
  );
}

export function AdminReportsPage() {
  const [stats, setStats] = useState<AdminStats | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    api.adminStats().then(res => {
      if (res.success && res.data) setStats(res.data);
    }).finally(() => setLoading(false));
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" />
      </div>
    );
  }

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-8">
      {/* Header */}
      <div className="flex items-center gap-3">
        <ChartBar weight="fill" className="w-6 h-6 text-primary-600" />
        <h2 className="text-xl font-semibold text-surface-900 dark:text-white">Reports</h2>
      </div>

      {/* Stats Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          icon={Users}
          label="Total Users"
          value={stats?.total_users ?? 0}
          color="primary"
        />
        <StatCard
          icon={Buildings}
          label="Total Lots"
          value={stats?.total_lots ?? 0}
          color="accent"
        />
        <StatCard
          icon={CalendarCheck}
          label="Total Bookings"
          value={stats?.total_bookings ?? 0}
          color="info"
        />
        <StatCard
          icon={Lightning}
          label="Active Bookings"
          value={stats?.active_bookings ?? 0}
          color="success"
        />
      </div>

      {/* Summary Card */}
      <div className="card p-6">
        <h3 className="text-lg font-semibold text-surface-900 dark:text-white mb-4">Overview</h3>
        <div className="space-y-4">
          <div className="flex items-center justify-between py-3 border-b border-surface-100 dark:border-surface-800">
            <span className="text-sm text-surface-600 dark:text-surface-400">Utilization Rate</span>
            <span className="text-sm font-semibold text-surface-900 dark:text-white">
              {stats && stats.total_lots > 0
                ? `${Math.round((stats.active_bookings / Math.max(stats.total_lots, 1)) * 100)}%`
                : '0%'}
            </span>
          </div>
          <div className="flex items-center justify-between py-3 border-b border-surface-100 dark:border-surface-800">
            <span className="text-sm text-surface-600 dark:text-surface-400">Avg. Bookings per User</span>
            <span className="text-sm font-semibold text-surface-900 dark:text-white">
              {stats && stats.total_users > 0
                ? (stats.total_bookings / stats.total_users).toFixed(1)
                : '0'}
            </span>
          </div>
          <div className="flex items-center justify-between py-3">
            <span className="text-sm text-surface-600 dark:text-surface-400">Active Booking Rate</span>
            <span className="text-sm font-semibold text-surface-900 dark:text-white">
              {stats && stats.total_bookings > 0
                ? `${Math.round((stats.active_bookings / stats.total_bookings) * 100)}%`
                : '0%'}
            </span>
          </div>
        </div>
      </div>
    </motion.div>
  );
}
