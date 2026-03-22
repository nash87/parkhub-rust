import { useEffect, useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Car, MagnifyingGlass, Flag, Lightning } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

interface FleetVehicle {
  id: string;
  user_id: string;
  username?: string;
  license_plate: string;
  make?: string;
  model?: string;
  color?: string;
  vehicle_type: string;
  is_default: boolean;
  created_at: string;
  bookings_count: number;
  last_used?: string;
  flagged: boolean;
  flag_reason?: string;
}

interface FleetStats {
  total_vehicles: number;
  types_distribution: Record<string, number>;
  electric_count: number;
  electric_ratio: number;
  flagged_count: number;
}

const typeColors: Record<string, string> = {
  car: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
  electric: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/30 dark:text-emerald-400',
  motorcycle: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  bicycle: 'bg-teal-100 text-teal-700 dark:bg-teal-900/30 dark:text-teal-400',
  van: 'bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400',
  truck: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  suv: 'bg-indigo-100 text-indigo-700 dark:bg-indigo-900/30 dark:text-indigo-400',
};

export function AdminFleetPage() {
  const { t } = useTranslation();
  const [vehicles, setVehicles] = useState<FleetVehicle[]>([]);
  const [stats, setStats] = useState<FleetStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [typeFilter, setTypeFilter] = useState('');

  const loadData = useCallback(async () => {
    setLoading(true);
    try {
      const params = new URLSearchParams();
      if (search) params.set('search', search);
      if (typeFilter) params.set('type', typeFilter);
      const qs = params.toString();

      const [fleetRes, statsRes] = await Promise.all([
        fetch(`/api/v1/admin/fleet${qs ? `?${qs}` : ''}`).then(r => r.json()),
        fetch('/api/v1/admin/fleet/stats').then(r => r.json()),
      ]);

      if (fleetRes.success) setVehicles(fleetRes.data || []);
      if (statsRes.success) setStats(statsRes.data);
    } catch {
      toast.error(t('common.error'));
    } finally {
      setLoading(false);
    }
  }, [search, typeFilter, t]);

  useEffect(() => {
    loadData();
  }, [loadData]);

  async function handleFlag(vehicleId: string, flagged: boolean, reason?: string) {
    try {
      const res = await fetch(`/api/v1/admin/fleet/${vehicleId}/flag`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ flagged, reason }),
      });
      const json = await res.json();
      if (json.success) {
        toast.success(flagged ? t('fleet.flagged', 'Vehicle flagged') : t('fleet.unflagged', 'Flag removed'));
        loadData();
      }
    } catch {
      toast.error(t('common.error'));
    }
  }

  const vehicleTypes = ['car', 'electric', 'motorcycle', 'bicycle', 'van', 'truck', 'suv'];

  if (loading && vehicles.length === 0) return (
    <div className="space-y-4">
      <div className="h-8 w-48 skeleton rounded-lg" />
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-4">
        {[1, 2, 3, 4].map(i => <div key={i} className="h-24 skeleton rounded-2xl" />)}
      </div>
      <div className="h-64 skeleton rounded-2xl" />
    </div>
  );

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      {/* Header */}
      <div className="flex items-center gap-3">
        <Car weight="duotone" className="w-6 h-6 text-primary-500" />
        <div>
          <h2 className="text-xl font-bold text-surface-900 dark:text-white">{t('fleet.title', 'Fleet Management')}</h2>
          <p className="text-sm text-surface-500">{t('fleet.subtitle', 'All vehicles across all users')}</p>
        </div>
      </div>

      {/* Stats cards */}
      {stats && (
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-4" data-testid="fleet-stats">
          <div className="glass-card rounded-2xl p-4">
            <p className="text-xs text-surface-500">{t('fleet.totalVehicles', 'Total Vehicles')}</p>
            <p className="text-2xl font-bold text-surface-900 dark:text-white">{stats.total_vehicles}</p>
          </div>
          <div className="glass-card rounded-2xl p-4">
            <p className="text-xs text-surface-500">{t('fleet.electricCount', 'Electric')}</p>
            <p className="text-2xl font-bold text-emerald-600 flex items-center gap-1">
              <Lightning weight="fill" className="w-5 h-5" />
              {stats.electric_count}
            </p>
          </div>
          <div className="glass-card rounded-2xl p-4">
            <p className="text-xs text-surface-500">{t('fleet.electricRatio', 'Electric Ratio')}</p>
            <p className="text-2xl font-bold text-surface-900 dark:text-white">{(stats.electric_ratio * 100).toFixed(0)}%</p>
          </div>
          <div className="glass-card rounded-2xl p-4">
            <p className="text-xs text-surface-500">{t('fleet.flaggedCount', 'Flagged')}</p>
            <p className="text-2xl font-bold text-red-600">{stats.flagged_count}</p>
          </div>
        </div>
      )}

      {/* Type distribution */}
      {stats && Object.keys(stats.types_distribution).length > 0 && (
        <div className="glass-card rounded-2xl p-4" data-testid="type-distribution">
          <h3 className="text-sm font-medium text-surface-600 dark:text-surface-300 mb-3">{t('fleet.byType', 'By Type')}</h3>
          <div className="flex flex-wrap gap-2">
            {Object.entries(stats.types_distribution).map(([type, count]) => (
              <span key={type} className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-sm font-medium ${typeColors[type] || 'bg-surface-100 text-surface-600'}`}>
                {type} <span className="opacity-75">({count})</span>
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Filters */}
      <div className="flex gap-3 flex-wrap">
        <div className="relative flex-1 min-w-[200px]">
          <MagnifyingGlass className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
          <input
            type="text"
            value={search}
            onChange={e => setSearch(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && loadData()}
            placeholder={t('fleet.search', 'Search plate, make, model...')}
            className="input-field pl-9 text-sm w-full"
            data-testid="fleet-search"
          />
        </div>
        <select
          value={typeFilter}
          onChange={e => setTypeFilter(e.target.value)}
          className="input-field text-sm"
          data-testid="fleet-type-filter"
        >
          <option value="">{t('fleet.allTypes', 'All Types')}</option>
          {vehicleTypes.map(vt => (
            <option key={vt} value={vt}>{vt.charAt(0).toUpperCase() + vt.slice(1)}</option>
          ))}
        </select>
      </div>

      {/* Table */}
      <div className="glass-card rounded-2xl overflow-hidden" data-testid="fleet-table">
        <div className="overflow-x-auto">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-surface-200 dark:border-surface-700 text-left">
                <th className="px-4 py-3 font-medium text-surface-500">{t('fleet.colPlate', 'Plate')}</th>
                <th className="px-4 py-3 font-medium text-surface-500">{t('fleet.colType', 'Type')}</th>
                <th className="px-4 py-3 font-medium text-surface-500">{t('fleet.colOwner', 'Owner')}</th>
                <th className="px-4 py-3 font-medium text-surface-500">{t('fleet.colMakeModel', 'Make/Model')}</th>
                <th className="px-4 py-3 font-medium text-surface-500">{t('fleet.colBookings', 'Bookings')}</th>
                <th className="px-4 py-3 font-medium text-surface-500">{t('fleet.colLastUsed', 'Last Used')}</th>
                <th className="px-4 py-3 font-medium text-surface-500">{t('fleet.colActions', 'Actions')}</th>
              </tr>
            </thead>
            <tbody>
              {vehicles.length === 0 ? (
                <tr>
                  <td colSpan={7} className="px-4 py-8 text-center text-surface-400">
                    {t('fleet.empty', 'No vehicles found')}
                  </td>
                </tr>
              ) : vehicles.map(v => (
                <tr key={v.id} className={`border-b border-surface-100 dark:border-surface-800 hover:bg-surface-50 dark:hover:bg-surface-800/50 transition-colors ${v.flagged ? 'bg-red-50/50 dark:bg-red-900/10' : ''}`} data-testid="fleet-row">
                  <td className="px-4 py-3 font-mono font-medium text-surface-900 dark:text-white">
                    {v.license_plate}
                    {v.flagged && <Flag weight="fill" className="inline w-4 h-4 text-red-500 ml-1" />}
                  </td>
                  <td className="px-4 py-3">
                    <span className={`inline-flex px-2 py-0.5 rounded-full text-xs font-medium ${typeColors[v.vehicle_type] || 'bg-surface-100 text-surface-600'}`}>
                      {v.vehicle_type}
                    </span>
                  </td>
                  <td className="px-4 py-3 text-surface-600 dark:text-surface-300">{v.username || '-'}</td>
                  <td className="px-4 py-3 text-surface-500">
                    {[v.make, v.model].filter(Boolean).join(' ') || '-'}
                  </td>
                  <td className="px-4 py-3 text-surface-600">{v.bookings_count}</td>
                  <td className="px-4 py-3 text-surface-500 text-xs">
                    {v.last_used ? new Date(v.last_used).toLocaleDateString() : '-'}
                  </td>
                  <td className="px-4 py-3">
                    <button
                      onClick={() => handleFlag(v.id, !v.flagged, v.flagged ? undefined : 'admin flag')}
                      className={`text-xs px-2 py-1 rounded-lg font-medium transition-colors ${
                        v.flagged
                          ? 'bg-red-100 text-red-700 hover:bg-red-200'
                          : 'bg-surface-100 text-surface-600 hover:bg-surface-200'
                      }`}
                      data-testid="flag-btn"
                    >
                      {v.flagged ? t('fleet.unflag', 'Unflag') : t('fleet.flag', 'Flag')}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </motion.div>
  );
}
