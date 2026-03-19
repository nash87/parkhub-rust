import { useState, useEffect } from 'react';
import { motion } from 'framer-motion';
import { SpinnerGap, Users, Buildings, CalendarCheck, Lightning } from '@phosphor-icons/react';
import { api, type AdminStats } from '../api/client';

function StatCard({ icon: Icon, label, value }: {
  icon: any;
  label: string;
  value: number;
  color?: string;
}) {
  return (
    <div className="stat-card">
      <div className="flex items-center gap-2 mb-2">
        <Icon weight="bold" className="w-4 h-4 text-surface-400" />
        <p className="text-sm font-medium text-surface-500 dark:text-surface-400">{label}</p>
      </div>
      <p className="stat-value text-surface-900 dark:text-white">{value}</p>
    </div>
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
      <h2 className="text-xl font-semibold text-surface-900 dark:text-white">Reports</h2>

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
        <h3 className="text-sm font-semibold text-surface-900 dark:text-white uppercase tracking-wide mb-4">Overview</h3>
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
