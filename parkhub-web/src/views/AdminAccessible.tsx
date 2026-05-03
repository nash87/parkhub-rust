import { useEffect, useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Wheelchair, Question, ToggleLeft, ToggleRight, ChartBar, UsersIcon } from '@phosphor-icons/react';
import { V11Meter } from '../components/v11/V11Meter';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

interface AccessibleStats {
  total_accessible_slots: number;
  occupied_accessible_slots: number;
  utilization_percent: number;
  total_accessible_bookings: number;
  users_with_accessibility_needs: number;
  priority_booking_active: boolean;
  priority_minutes: number;
}

interface Lot {
  id: string;
  name: string;
}

interface Slot {
  id: string;
  lot_id: string;
  slot_number: number | string;
  status: string;
  slot_type?: string;
  is_accessible?: boolean;
}

export function AdminAccessiblePage() {
  const { t } = useTranslation();
  const [stats, setStats] = useState<AccessibleStats | null>(null);
  const [lots, setLots] = useState<Lot[]>([]);
  const [selectedLot, setSelectedLot] = useState<string>('');
  const [slots, setSlots] = useState<Slot[]>([]);
  const [loading, setLoading] = useState(true);
  const [slotsLoading, setSlotsLoading] = useState(false);
  const [showHelp, setShowHelp] = useState(false);

  const loadStats = useCallback(async () => {
    try {
      const [statsRes, lotsRes] = await Promise.all([
        fetch('/api/v1/bookings/accessible-stats').then(r => r.json()),
        fetch('/api/v1/lots').then(r => r.json()),
      ]);
      if (statsRes.success) setStats(statsRes.data);
      if (lotsRes.success) setLots(lotsRes.data || []);
    } catch { /* ignore */ }
    setLoading(false);
  }, []);

  useEffect(() => { loadStats(); }, [loadStats]);

  const loadSlots = useCallback(async (lotId: string) => {
    setSlotsLoading(true);
    try {
      const res = await fetch(`/api/v1/lots/${lotId}/slots`).then(r => r.json());
      if (res.success) setSlots(res.data || []);
    } catch { /* ignore */ }
    setSlotsLoading(false);
  }, []);

  useEffect(() => {
    if (selectedLot) loadSlots(selectedLot);
    else setSlots([]);
  }, [selectedLot, loadSlots]);

  async function toggleAccessible(lotId: string, slotId: string, current: boolean) {
    try {
      const res = await fetch(`/api/v1/admin/lots/${lotId}/slots/${slotId}/accessible`, {
        method: 'PUT',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ is_accessible: !current }),
      }).then(r => r.json());
      if (res.success) {
        setSlots(prev => prev.map(s => s.id === slotId ? { ...s, is_accessible: !current } : s));
        toast.success(t('accessible.toggleSuccess', 'Slot accessibility updated'));
        loadStats();
      } else {
        toast.error(res.error?.message || t('common.error'));
      }
    } catch { toast.error(t('common.error')); }
  }

  if (loading) {
    return (
      <div className="space-y-4">
        {Array.from({ length: 3 }, (_, i) => <div key={i} className="h-24 skeleton rounded-xl" />)}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* v11 SOTA hero — info tone (accessibility + neutral blue). Pairs
          with PRs #489/#490/#491/#493 chrome. */}
      <section className="admin-hero admin-hero--info">
        <div className="admin-hero-left">
          <div className="admin-hero-eyebrow">
            <span className="admin-hero-dot" aria-hidden="true"></span>
            <Wheelchair weight="bold" className="w-3.5 h-3.5" />
            {t('accessible.eyebrow', 'ACCESSIBLE PARKING')}
          </div>
          <h1 className="admin-hero-headline">{t('accessible.title', 'Accessible Parking')}</h1>
          <p className="admin-hero-sub">{t('accessible.subtitle', 'Manage accessible slots and view utilization')}</p>
        </div>
        <div className="admin-hero-actions">
          <button
            onClick={() => setShowHelp(!showHelp)}
            className="admin-hero-iconbtn"
            aria-label={t('common.help', 'Help')}
          >
            <Question weight="bold" className="w-4 h-4" />
          </button>
        </div>
      </section>

      {/* Help / About this module */}
      {showHelp && (
        <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-xl p-4">
          <p className="text-sm text-blue-800 dark:text-blue-300">
            {t('accessible.help', 'This module manages accessible parking slots for users with disabilities. Accessible users (wheelchair, reduced mobility, visual, hearing) get a 30-minute priority booking window on accessible slots. Admins can mark any slot as accessible, and users can set their accessibility needs in their profile.')}
          </p>
        </motion.div>
      )}

      {/* v11 SOTA stat meters — info / success / accent / warn tones */}
      {stats && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4" data-testid="accessible-stats">
          <StatCard tone="info" label={t('accessible.totalSlots', 'Accessible Slots')} value={stats.total_accessible_slots} icon={<Wheelchair weight="bold" className="w-3.5 h-3.5" />} />
          <StatCard tone="success" label={t('accessible.utilization', 'Utilization')} value={`${stats.utilization_percent.toFixed(0)}%`} icon={<ChartBar weight="bold" className="w-3.5 h-3.5" />} />
          <StatCard tone="accent" label={t('accessible.totalBookings', 'Active Bookings')} value={stats.total_accessible_bookings} icon={<Wheelchair weight="bold" className="w-3.5 h-3.5" />} />
          <StatCard tone="warn" label={t('accessible.usersWithNeeds', 'UsersIcon with Needs')} value={stats.users_with_accessibility_needs} icon={<UsersIcon weight="bold" className="w-3.5 h-3.5" />} />
        </div>
      )}

      {/* Priority info */}
      {stats?.priority_booking_active && (
        <div className="bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-xl px-4 py-3">
          <p className="text-sm text-amber-800 dark:text-amber-300">
            {t('accessible.priority', 'Priority booking active: accessible users get a {{minutes}}-minute head start on accessible slots.', { minutes: stats.priority_minutes })}
          </p>
        </div>
      )}

      {/* Lot selector + slot toggle */}
      <div className="bg-white dark:bg-surface-900 rounded-xl border border-surface-200 dark:border-surface-800 p-5">
        <h3 className="text-base font-semibold text-surface-900 dark:text-white mb-3">
          {t('accessible.manageSlots', 'Manage Accessible Slots')}
        </h3>
        <select
          data-testid="lot-selector"
          className="input mb-4"
          value={selectedLot}
          onChange={e => setSelectedLot(e.target.value)}
        >
          <option value="">{t('accessible.selectLot', 'Select a parking lot...')}</option>
          {lots.map(lot => (
            <option key={lot.id} value={lot.id}>{lot.name}</option>
          ))}
        </select>

        {slotsLoading && (
          <div className="grid grid-cols-4 sm:grid-cols-6 md:grid-cols-8 gap-2">
            {Array.from({ length: 12 }, (_, i) => <div key={i} className="h-12 skeleton rounded-lg" />)}
          </div>
        )}

        {!slotsLoading && selectedLot && slots.length > 0 && (
          <div className="space-y-2" data-testid="slot-list">
            <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-2">
              {slots.map(slot => (
                <button
                  key={slot.id}
                  data-testid="slot-toggle"
                  onClick={() => toggleAccessible(selectedLot, slot.id, !!slot.is_accessible)}
                  className={`flex items-center justify-between px-3 py-2.5 rounded-lg border transition-colors ${
                    slot.is_accessible
                      ? 'border-blue-300 dark:border-blue-700 bg-blue-50 dark:bg-blue-900/20'
                      : 'border-surface-200 dark:border-surface-700 bg-white dark:bg-surface-800'
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-surface-900 dark:text-white">
                      {t('accessible.slotLabel', 'Slot')} {slot.slot_number}
                    </span>
                    {slot.slot_type && slot.slot_type !== 'standard' && (
                      <span className="text-xs text-surface-500 dark:text-surface-400">({slot.slot_type})</span>
                    )}
                  </div>
                  {slot.is_accessible
                    ? <ToggleRight weight="fill" className="w-6 h-6 text-blue-500" />
                    : <ToggleLeft weight="regular" className="w-6 h-6 text-surface-400" />
                  }
                </button>
              ))}
            </div>
          </div>
        )}

        {!slotsLoading && selectedLot && slots.length === 0 && (
          <p className="text-sm text-surface-500 dark:text-surface-400">{t('accessible.noSlots', 'No slots found for this lot.')}</p>
        )}
      </div>
    </div>
  );
}

function StatCard({ label, value, icon, tone = 'primary' }: {
  label: string;
  value: string | number;
  icon: React.ReactNode;
  tone?: 'primary' | 'accent' | 'info' | 'success' | 'warn' | 'danger';
}) {
  // Delegates to the shared V11Meter primitive.
  return <V11Meter icon={icon} label={label} value={value} tone={tone} />;
}
