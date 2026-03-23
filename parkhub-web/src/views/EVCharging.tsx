import { useEffect, useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Lightning, Play, Stop, Question, Clock, BatteryCharging } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

interface EvCharger {
  id: string;
  lot_id: string;
  label: string;
  connector_type: 'type2' | 'ccs' | 'chademo' | 'tesla';
  power_kw: number;
  status: 'available' | 'in_use' | 'offline' | 'maintenance';
  location_hint: string | null;
}

interface ChargingSession {
  id: string;
  charger_id: string;
  user_id: string;
  start_time: string;
  end_time: string | null;
  kwh_consumed: number;
  status: 'active' | 'completed' | 'cancelled';
}

interface Lot {
  id: string;
  name: string;
}

interface ChargerStats {
  total_chargers: number;
  available: number;
  in_use: number;
  offline: number;
  total_sessions: number;
  total_kwh: number;
}

const statusColors: Record<string, string> = {
  available: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
  in_use: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400',
  offline: 'bg-surface-100 text-surface-500 dark:bg-surface-800 dark:text-surface-400',
  maintenance: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
};

const connectorLabels: Record<string, string> = {
  type2: 'Type 2',
  ccs: 'CCS',
  chademo: 'CHAdeMO',
  tesla: 'Tesla',
};

export function EVChargingPage() {
  const { t } = useTranslation();
  const [lots, setLots] = useState<Lot[]>([]);
  const [selectedLot, setSelectedLot] = useState('');
  const [chargers, setChargers] = useState<EvCharger[]>([]);
  const [sessions, setSessions] = useState<ChargingSession[]>([]);
  const [loading, setLoading] = useState(true);
  const [showHelp, setShowHelp] = useState(false);

  const loadLots = useCallback(async () => {
    try {
      const res = await fetch('/api/v1/lots').then(r => r.json());
      if (res.success) {
        setLots(res.data || []);
        if (res.data?.length > 0 && !selectedLot) setSelectedLot(res.data[0].id);
      }
    } catch { /* ignore */ }
  }, [selectedLot]);

  const loadChargers = useCallback(async () => {
    if (!selectedLot) return;
    setLoading(true);
    try {
      const [chRes, sesRes] = await Promise.all([
        fetch(`/api/v1/lots/${selectedLot}/chargers`).then(r => r.json()),
        fetch('/api/v1/chargers/sessions').then(r => r.json()),
      ]);
      if (chRes.success) setChargers(chRes.data || []);
      if (sesRes.success) setSessions(sesRes.data || []);
    } catch { /* ignore */ }
    setLoading(false);
  }, [selectedLot]);

  useEffect(() => { loadLots(); }, [loadLots]);
  useEffect(() => { loadChargers(); }, [loadChargers]);

  async function handleStart(chargerId: string) {
    try {
      const res = await fetch(`/api/v1/chargers/${chargerId}/start`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({}),
      }).then(r => r.json());
      if (res.success) { toast.success(t('evCharging.started')); loadChargers(); }
      else toast.error(res.error?.message || t('common.error'));
    } catch { toast.error(t('common.error')); }
  }

  async function handleStop(chargerId: string) {
    try {
      const res = await fetch(`/api/v1/chargers/${chargerId}/stop`, { method: 'POST' }).then(r => r.json());
      if (res.success) { toast.success(t('evCharging.stopped')); loadChargers(); }
      else toast.error(res.error?.message || t('common.error'));
    } catch { toast.error(t('common.error')); }
  }

  const activeSession = (chargerId: string) => sessions.find(s => s.charger_id === chargerId && s.status === 'active');

  return (
    <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white flex items-center gap-2">
            <Lightning weight="duotone" className="w-7 h-7 text-yellow-500" />
            {t('evCharging.title')}
            <button onClick={() => setShowHelp(!showHelp)} className="text-surface-400 hover:text-primary-500" aria-label="Help">
              <Question weight="fill" className="w-5 h-5" />
            </button>
          </h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">{t('evCharging.subtitle')}</p>
        </div>
        <select value={selectedLot} onChange={e => setSelectedLot(e.target.value)} className="rounded-xl bg-surface-50 dark:bg-surface-800 border border-surface-200 dark:border-surface-700 px-3 py-2 text-sm">
          {lots.map(lot => <option key={lot.id} value={lot.id}>{lot.name}</option>)}
        </select>
      </div>

      {showHelp && (
        <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} className="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-xl p-4 text-sm text-yellow-700 dark:text-yellow-300">
          <p className="font-semibold mb-1">{t('evCharging.aboutTitle')}</p>
          <p>{t('evCharging.help')}</p>
        </motion.div>
      )}

      {loading ? (
        <div className="flex justify-center py-12"><div className="w-8 h-8 border-2 border-primary-500 border-t-transparent rounded-full animate-spin" /></div>
      ) : chargers.length === 0 ? (
        <div className="text-center py-12 text-surface-400">
          <Lightning className="w-12 h-12 mx-auto mb-3 opacity-40" />
          <p>{t('evCharging.empty')}</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {chargers.map(ch => {
            const active = activeSession(ch.id);
            return (
              <motion.div key={ch.id} initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="bg-white dark:bg-surface-800 rounded-xl border border-surface-200 dark:border-surface-700 p-4">
                <div className="flex items-center justify-between mb-3">
                  <span className="font-semibold text-surface-900 dark:text-white">{ch.label}</span>
                  <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${statusColors[ch.status]}`}>
                    {t(`evCharging.status.${ch.status}`)}
                  </span>
                </div>
                <div className="space-y-1 text-sm text-surface-500 mb-3">
                  <div className="flex items-center gap-1"><BatteryCharging className="w-4 h-4" />{connectorLabels[ch.connector_type]} - {ch.power_kw} kW</div>
                  {ch.location_hint && <div>{ch.location_hint}</div>}
                </div>
                {ch.status === 'available' && (
                  <button onClick={() => handleStart(ch.id)} className="btn-primary w-full flex items-center justify-center gap-2 text-sm">
                    <Play weight="bold" className="w-4 h-4" />{t('evCharging.startCharging')}
                  </button>
                )}
                {active && (
                  <div>
                    <div className="flex items-center gap-1 text-sm text-amber-600 mb-2">
                      <Clock className="w-4 h-4" />{t('evCharging.chargingSince')} {new Date(active.start_time).toLocaleTimeString()}
                    </div>
                    <button onClick={() => handleStop(ch.id)} className="btn-secondary w-full flex items-center justify-center gap-2 text-sm border-red-300 text-red-600 hover:bg-red-50">
                      <Stop weight="bold" className="w-4 h-4" />{t('evCharging.stopCharging')}
                    </button>
                  </div>
                )}
              </motion.div>
            );
          })}
        </div>
      )}

      {/* Session history */}
      {sessions.length > 0 && (
        <div>
          <h2 className="text-lg font-semibold text-surface-900 dark:text-white mb-3">{t('evCharging.history')}</h2>
          <div className="space-y-2">
            {sessions.filter(s => s.status === 'completed').slice(0, 10).map(s => (
              <div key={s.id} className="bg-white dark:bg-surface-800 rounded-xl border border-surface-200 dark:border-surface-700 p-3 flex items-center justify-between text-sm">
                <div>
                  <span className="text-surface-900 dark:text-white font-medium">{new Date(s.start_time).toLocaleDateString()}</span>
                  <span className="text-surface-500 ml-2">{new Date(s.start_time).toLocaleTimeString()} - {s.end_time ? new Date(s.end_time).toLocaleTimeString() : '-'}</span>
                </div>
                <span className="font-semibold text-green-600">{s.kwh_consumed.toFixed(1)} kWh</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </motion.div>
  );
}

export function AdminChargersPage() {
  const { t } = useTranslation();
  const [stats, setStats] = useState<ChargerStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [showHelp, setShowHelp] = useState(false);

  useEffect(() => {
    setLoading(true);
    fetch('/api/v1/admin/chargers').then(r => r.json()).then(res => {
      if (res.success) setStats(res.data);
      setLoading(false);
    }).catch(() => setLoading(false));
  }, []);

  return (
    <motion.div initial={{ opacity: 0, y: 12 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-surface-900 dark:text-white flex items-center gap-2">
          <Lightning weight="duotone" className="w-7 h-7 text-yellow-500" />
          {t('evCharging.adminTitle')}
          <button onClick={() => setShowHelp(!showHelp)} className="text-surface-400 hover:text-primary-500" aria-label="Help">
            <Question weight="fill" className="w-5 h-5" />
          </button>
        </h1>
        <p className="text-surface-500 dark:text-surface-400 mt-1">{t('evCharging.adminSubtitle')}</p>
      </div>

      {showHelp && (
        <motion.div initial={{ opacity: 0, height: 0 }} animate={{ opacity: 1, height: 'auto' }} className="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-xl p-4 text-sm text-yellow-700 dark:text-yellow-300">
          <p className="font-semibold mb-1">{t('evCharging.aboutTitle')}</p>
          <p>{t('evCharging.help')}</p>
        </motion.div>
      )}

      {loading || !stats ? (
        <div className="flex justify-center py-12"><div className="w-8 h-8 border-2 border-primary-500 border-t-transparent rounded-full animate-spin" /></div>
      ) : (
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-4">
          {[
            { label: t('evCharging.totalChargers'), value: stats.total_chargers, color: 'text-surface-900 dark:text-white' },
            { label: t('evCharging.status.available'), value: stats.available, color: 'text-green-600' },
            { label: t('evCharging.status.in_use'), value: stats.in_use, color: 'text-amber-600' },
            { label: t('evCharging.status.offline'), value: stats.offline, color: 'text-surface-400' },
            { label: t('evCharging.totalSessions'), value: stats.total_sessions, color: 'text-primary-600' },
            { label: t('evCharging.totalKwh'), value: `${stats.total_kwh.toFixed(0)} kWh`, color: 'text-yellow-600' },
          ].map((s, i) => (
            <div key={i} className="bg-white dark:bg-surface-800 rounded-xl border border-surface-200 dark:border-surface-700 p-4 text-center">
              <div className={`text-2xl font-bold ${s.color}`}>{s.value}</div>
              <div className="text-sm text-surface-500 mt-1">{s.label}</div>
            </div>
          ))}
        </div>
      )}
    </motion.div>
  );
}
