import { useEffect, useMemo, useState, type ComponentType } from 'react';
import { Link } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  ArrowRight,
  Buildings,
  CalendarCheck,
  ChartLine,
  GearSix,
  Lightning,
  Megaphone,
  SpinnerGap,
  Users,
  WarningCircle,
} from '@phosphor-icons/react';
import { api, type AdminStats, type Booking, type ParkingLot } from '../api/client';

type OverviewIcon = ComponentType<{ className?: string; weight?: 'regular' | 'fill' | 'bold' | 'duotone' }>;

function percent(numerator: number, denominator: number): number {
  if (denominator <= 0) return 0;
  return Math.min(100, Math.round((numerator / denominator) * 100));
}

function AdminMetricCard({
  icon: Icon,
  label,
  value,
  helper,
}: {
  icon: OverviewIcon;
  label: string;
  value: string | number;
  helper: string;
}) {
  return (
    <div className="rounded-lg border border-surface-200 bg-white p-4 shadow-sm dark:border-surface-800 dark:bg-surface-900">
      <div className="flex items-center gap-2 text-surface-500 dark:text-surface-400">
        <Icon weight="bold" className="h-4 w-4" />
        <p className="text-xs font-semibold uppercase tracking-normal">{label}</p>
      </div>
      <p className="mt-3 text-3xl font-semibold tracking-normal text-surface-950 dark:text-white">
        {value}
      </p>
      <p className="mt-1 text-sm text-surface-500 dark:text-surface-400">{helper}</p>
    </div>
  );
}

function SnapshotRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-surface-100 py-3 last:border-b-0 dark:border-surface-800">
      <span className="text-sm text-surface-600 dark:text-surface-400">{label}</span>
      <span className="font-mono text-sm font-semibold text-surface-950 dark:text-white">{value}</span>
    </div>
  );
}

export function AdminOverviewPage() {
  const { t } = useTranslation();
  const [stats, setStats] = useState<AdminStats | null>(null);
  const [bookings, setBookings] = useState<Booking[]>([]);
  const [lots, setLots] = useState<ParkingLot[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let mounted = true;
    Promise.all([api.adminStats(), api.getBookings(), api.getLots()])
      .then(([statsRes, bookingsRes, lotsRes]) => {
        if (!mounted) return;
        if (statsRes.success && statsRes.data) setStats(statsRes.data);
        if (bookingsRes.success && bookingsRes.data) setBookings(bookingsRes.data);
        if (lotsRes.success && lotsRes.data) setLots(lotsRes.data);
      })
      .finally(() => {
        if (mounted) setLoading(false);
      });
    return () => {
      mounted = false;
    };
  }, []);

  const snapshot = useMemo(() => {
    const totalSlots = lots.reduce((sum, lot) => sum + lot.total_slots, 0);
    const availableSlots = lots.reduce((sum, lot) => sum + lot.available_slots, 0);
    const occupiedSlots = Math.max(totalSlots - availableSlots, 0);
    const activeBookings = stats?.active_bookings ?? 0;
    const totalBookings = stats?.total_bookings ?? bookings.length;
    const totalUsers = stats?.total_users ?? 0;

    return {
      totalSlots,
      occupiedSlots,
      utilization: percent(occupiedSlots, totalSlots),
      activeBookingRate: percent(activeBookings, totalBookings),
      bookingsPerUser: totalUsers > 0 ? (totalBookings / totalUsers).toFixed(1) : '0.0',
    };
  }, [bookings.length, lots, stats]);

  const quickActions = [
    { label: t('admin.settings', 'Settings'), to: '/admin/settings', icon: GearSix },
    { label: t('admin.users', 'Users'), to: '/admin/users', icon: Users },
    { label: t('admin.announcements', 'Announcements'), to: '/admin/announcements', icon: Megaphone },
    { label: t('admin.reports', 'Reports'), to: '/admin/reports', icon: ChartLine },
  ] as const;

  if (loading) {
    return (
      <div className="flex h-64 items-center justify-center" role="status" aria-label={t('common.loading', 'Loading')}>
        <SpinnerGap weight="bold" className="h-8 w-8 animate-spin text-primary-600" aria-hidden="true" />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-3 border-b border-surface-200 pb-5 dark:border-surface-800 lg:flex-row lg:items-end lg:justify-between">
        <div>
          <h2 className="text-2xl font-semibold tracking-normal text-surface-950 dark:text-white">
            {t('admin.overview', 'Overview')}
          </h2>
          <p className="mt-1 max-w-2xl text-sm leading-6 text-surface-500 dark:text-surface-400">
            {t('admin.overviewSubtitle', 'Instance status, operating priorities, and direct administration paths.')}
          </p>
        </div>
        {lots.length === 0 ? (
          <div className="inline-flex items-center gap-2 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm font-medium text-amber-800 dark:border-amber-900/60 dark:bg-amber-950/30 dark:text-amber-200">
            <WarningCircle weight="bold" className="h-4 w-4" />
            {t('admin.noLotsConfigured', 'No parking lots configured')}
          </div>
        ) : null}
      </div>

      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <AdminMetricCard
          icon={Users}
          label={t('admin.totalUsers', 'Total users')}
          value={stats?.total_users ?? 0}
          helper={t('admin.overviewUsersHelper', 'Known user accounts')}
        />
        <AdminMetricCard
          icon={Buildings}
          label={t('admin.totalLots', 'Parking lots')}
          value={stats?.total_lots ?? lots.length}
          helper={`${snapshot.occupiedSlots}/${snapshot.totalSlots} ${t('admin.slotsOccupied', 'slots occupied')}`}
        />
        <AdminMetricCard
          icon={CalendarCheck}
          label={t('admin.totalBookings', 'Bookings')}
          value={stats?.total_bookings ?? bookings.length}
          helper={t('admin.overviewBookingsHelper', 'All recorded reservations')}
        />
        <AdminMetricCard
          icon={Lightning}
          label={t('admin.activeBookings', 'Active bookings')}
          value={stats?.active_bookings ?? 0}
          helper={t('admin.overviewActiveHelper', 'Currently in effect')}
        />
      </div>

      <div className="grid grid-cols-1 gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(320px,420px)]">
        <section className="rounded-lg border border-surface-200 bg-white p-5 shadow-sm dark:border-surface-800 dark:bg-surface-900">
          <h3 className="text-sm font-semibold uppercase tracking-normal text-surface-950 dark:text-white">
            {t('admin.operatingSnapshot', 'Operating snapshot')}
          </h3>
          <div className="mt-4">
            <SnapshotRow label={t('admin.utilizationRate', 'Utilization rate')} value={`${snapshot.utilization}%`} />
            <SnapshotRow label={t('admin.activeBookingRate', 'Active booking rate')} value={`${snapshot.activeBookingRate}%`} />
            <SnapshotRow label={t('admin.avgBookingsPerUser', 'Avg. bookings per user')} value={snapshot.bookingsPerUser} />
            <SnapshotRow label={t('admin.parkingCapacity', 'Parking capacity')} value={String(snapshot.totalSlots)} />
          </div>
        </section>

        <section className="rounded-lg border border-surface-200 bg-white p-5 shadow-sm dark:border-surface-800 dark:bg-surface-900">
          <h3 className="text-sm font-semibold uppercase tracking-normal text-surface-950 dark:text-white">
            {t('admin.quickActions', 'Quick actions')}
          </h3>
          <div className="mt-4 grid gap-2">
            {quickActions.map((action) => (
              <Link
                key={action.to}
                to={action.to}
                className="group flex items-center gap-3 rounded-lg border border-surface-100 px-3 py-3 text-sm font-medium text-surface-700 transition-colors hover:border-primary-200 hover:bg-primary-50 hover:text-primary-700 dark:border-surface-800 dark:text-surface-300 dark:hover:border-primary-900 dark:hover:bg-primary-950/30 dark:hover:text-primary-300"
              >
                <action.icon weight="bold" className="h-4 w-4 shrink-0" />
                <span className="min-w-0 flex-1">{action.label}</span>
                <ArrowRight weight="bold" className="h-4 w-4 shrink-0 text-surface-400 group-hover:text-primary-500" />
              </Link>
            ))}
          </div>
        </section>
      </div>
    </div>
  );
}
