import { useEffect, useState, useCallback } from 'react';
import { useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';

interface FloorDisplay {
  floor_name: string;
  floor_number: number;
  total_slots: number;
  available_slots: number;
  occupancy_percent: number;
}

interface LotDisplayData {
  lot_id: string;
  lot_name: string;
  total_slots: number;
  available_slots: number;
  occupancy_percent: number;
  color_status: 'green' | 'yellow' | 'red';
  floors: FloorDisplay[];
  timestamp: string;
}

const COLOR_MAP = {
  green: { bar: 'bg-emerald-500', text: 'text-emerald-400', glow: 'shadow-emerald-500/30' },
  yellow: { bar: 'bg-amber-400', text: 'text-amber-400', glow: 'shadow-amber-400/30' },
  red: { bar: 'bg-red-500', text: 'text-red-400', glow: 'shadow-red-500/30' },
};

/** Full-screen lobby display designed for parking garage monitors. */
export function LobbyDisplayPage() {
  const { lotId } = useParams<{ lotId: string }>();
  const { t } = useTranslation();
  const [data, setData] = useState<LotDisplayData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);
  const [currentTime, setCurrentTime] = useState(new Date());

  const fetchDisplay = useCallback(async () => {
    if (!lotId) return;
    try {
      const res = await fetch(`/api/v1/lots/${lotId}/display`);
      const json = await res.json();
      if (json.success && json.data) {
        setData(json.data);
        setLastUpdated(new Date());
        setError(null);
      } else {
        setError(json.error?.message || t('lobby.error', 'Lot not found'));
      }
    } catch {
      setError(t('lobby.networkError', 'Network error'));
    }
  }, [lotId, t]);

  // Poll every 10 seconds
  useEffect(() => {
    fetchDisplay();
    const interval = setInterval(fetchDisplay, 10_000);
    return () => clearInterval(interval);
  }, [fetchDisplay]);

  // Update clock every second
  useEffect(() => {
    const timer = setInterval(() => setCurrentTime(new Date()), 1000);
    return () => clearInterval(timer);
  }, []);

  if (error) {
    return (
      <div className="min-h-dvh bg-gray-950 flex items-center justify-center" data-testid="lobby-error">
        <p className="text-red-400 text-4xl font-bold">{error}</p>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="min-h-dvh bg-gray-950 flex items-center justify-center" data-testid="lobby-loading">
        <div className="w-16 h-16 border-4 border-white/20 border-t-white rounded-full animate-spin" />
      </div>
    );
  }

  const colors = COLOR_MAP[data.color_status] || COLOR_MAP.green;
  const occupancyWidth = Math.min(data.occupancy_percent, 100);

  return (
    <div
      className="min-h-dvh bg-gray-950 text-white flex flex-col p-8 select-none overflow-hidden"
      data-testid="lobby-display"
    >
      {/* Header: lot name + clock */}
      <div className="flex items-center justify-between mb-8">
        <h1 className="text-[4rem] leading-tight font-black tracking-tight truncate" data-testid="lobby-lot-name">
          {data.lot_name}
        </h1>
        <time className="text-3xl font-mono text-gray-400 tabular-nums shrink-0 ml-8" data-testid="lobby-clock">
          {currentTime.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}
        </time>
      </div>

      {/* Main number display */}
      <div className="flex-1 flex flex-col items-center justify-center gap-6">
        <div className="text-center">
          <span
            className={`text-[8rem] leading-none font-black tabular-nums ${colors.text}`}
            data-testid="lobby-available"
          >
            {data.available_slots}
          </span>
          <span className="text-[4rem] text-gray-500 font-light mx-4">/</span>
          <span className="text-[4rem] text-gray-400 font-semibold tabular-nums" data-testid="lobby-total">
            {data.total_slots}
          </span>
        </div>
        <p className="text-2xl text-gray-400 uppercase tracking-widest">
          {t('lobby.available', 'Available')}
        </p>

        {/* Occupancy bar */}
        <div className="w-full max-w-4xl mt-4" data-testid="lobby-bar">
          <div className="w-full h-8 bg-gray-800 rounded-full overflow-hidden">
            <div
              className={`h-full ${colors.bar} rounded-full transition-all duration-700 ease-out shadow-lg ${colors.glow}`}
              style={{ width: `${occupancyWidth}%` }}
              role="progressbar"
              aria-valuenow={Math.round(data.occupancy_percent)}
              aria-valuemin={0}
              aria-valuemax={100}
              aria-label={t('lobby.occupancy', 'Occupancy')}
            />
          </div>
          <p className="text-center text-xl text-gray-500 mt-2">
            {t('lobby.occupancy', 'Occupancy')}: {Math.round(data.occupancy_percent)}%
          </p>
        </div>
      </div>

      {/* Floor breakdown */}
      {data.floors.length > 0 && (
        <div className="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-4 mt-8" data-testid="lobby-floors">
          {data.floors.map((floor) => {
            const floorOcc = Math.min(floor.occupancy_percent, 100);
            const floorColor = floor.occupancy_percent > 80 ? 'red' : floor.occupancy_percent >= 50 ? 'yellow' : 'green';
            const fc = COLOR_MAP[floorColor];
            return (
              <div
                key={floor.floor_number}
                className="bg-gray-900 rounded-2xl p-5 flex flex-col items-center"
                data-testid="lobby-floor-card"
              >
                <p className="text-lg text-gray-400 mb-1">
                  {t('lobby.floor', 'Floor')} {floor.floor_name || floor.floor_number}
                </p>
                <p className={`text-4xl font-black tabular-nums ${fc.text}`}>
                  {floor.available_slots}
                </p>
                <p className="text-sm text-gray-500">
                  / {floor.total_slots}
                </p>
                <div className="w-full h-2 bg-gray-800 rounded-full mt-3 overflow-hidden">
                  <div
                    className={`h-full ${fc.bar} rounded-full transition-all duration-700`}
                    style={{ width: `${floorOcc}%` }}
                  />
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Footer: last updated */}
      <div className="flex justify-center mt-8 text-gray-600 text-lg">
        <p data-testid="lobby-last-updated">
          {t('lobby.lastUpdated', 'Last updated')}: {lastUpdated?.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }) || '—'}
        </p>
      </div>
    </div>
  );
}
