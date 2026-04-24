import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Badge, Card, V5NamedIcon } from '../primitives';
import { api } from '../../api/client';
import type { ScreenId } from '../nav';

const DAYS_FULL = ['Montag', 'Dienstag', 'Mittwoch', 'Donnerstag', 'Freitag', 'Samstag', 'Sonntag'];
const DAYS_SHORT = ['Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa', 'So'];
const DAY_KEY_EN = ['monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday', 'sunday'];

type Level = 'low' | 'medium' | 'high';

interface DayPrediction {
  dayIndex: number;
  dayName: string;
  dayShort: string;
  predicted: number;
  confidence: number;
  peakHour: number;
  offPeakHour: number;
  level: Level;
}

function getLevel(pct: number): Level {
  if (pct >= 70) return 'high';
  if (pct >= 40) return 'medium';
  return 'low';
}

function levelColor(level: Level): string {
  switch (level) {
    case 'low': return 'oklch(0.65 0.17 160)';
    case 'medium': return 'oklch(0.74 0.16 75)';
    case 'high': return 'oklch(0.58 0.22 25)';
  }
}

function levelLabel(level: Level): string {
  switch (level) {
    case 'low': return 'Ruhig';
    case 'medium': return 'Mittel';
    case 'high': return 'Voll';
  }
}

function levelVariant(level: Level) {
  switch (level) {
    case 'low': return 'success' as const;
    case 'medium': return 'warning' as const;
    case 'high': return 'error' as const;
  }
}

function formatHour(hour: number): string {
  return `${String(hour).padStart(2, '0')}:00`;
}

// Stable deterministic weekday/weekend fallback when historical data is absent.
function fallbackPrediction(idx: number): { predicted: number; peakHour: number; offPeakHour: number } {
  if (idx < 5) return { predicted: 60, peakHour: 9, offPeakHour: 14 };
  return { predicted: 20, peakHour: 10, offPeakHour: 7 };
}

export function VorhersagenV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const statsQuery = useQuery({
    queryKey: ['admin-stats-forecast'],
    queryFn: async () => {
      const res = await api.getAdminStatsExtended();
      if (!res.success) throw new Error(res.error?.message ?? 'Vorhersagen konnten nicht geladen werden');
      return res.data;
    },
    staleTime: 60_000,
    refetchOnWindowFocus: true,
  });

  const predictions = useMemo<DayPrediction[]>(() => {
    const byDay = statsQuery.data?.occupancy_by_day ?? {};
    const byHour = statsQuery.data?.occupancy_by_hour ?? {};
    const hasHourly = Object.keys(byHour).length > 0;
    return DAYS_FULL.map((name, idx) => {
      const dayData = byDay[String(idx)] ?? byDay[DAY_KEY_EN[idx]] ?? byDay[name.toLowerCase()];
      let predicted: number;
      let peakHour: number;
      let offPeakHour: number;
      let confidence: number;
      if (dayData) {
        predicted = Math.round(dayData.avg_percentage);
        peakHour = dayData.peak_hour;
        offPeakHour = peakHour >= 12 ? 7 : 14;
        confidence = hasHourly ? 85 : 65;
      } else {
        const f = fallbackPrediction(idx);
        predicted = f.predicted;
        peakHour = f.peakHour;
        offPeakHour = f.offPeakHour;
        confidence = 40;
      }
      return {
        dayIndex: idx,
        dayName: name,
        dayShort: DAYS_SHORT[idx],
        predicted,
        confidence,
        peakHour,
        offPeakHour,
        level: getLevel(predicted),
      };
    });
  }, [statsQuery.data]);

  const recommendation = useMemo(() => {
    if (!predictions.length) return null;
    const best = [...predictions].sort((a, b) => a.predicted - b.predicted)[0];
    return {
      day: best.dayName,
      timeSlot: `${formatHour(best.offPeakHour)}–${formatHour(best.offPeakHour + 2)}`,
    };
  }, [predictions]);

  if (statsQuery.isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto' }}>
        <div style={{ height: 32, width: 200, borderRadius: 8, background: 'var(--v5-sur2)', marginBottom: 14, animation: 'ph-v5-pulse 1.6s ease infinite' }} />
        <div style={{ height: 110, borderRadius: 14, background: 'var(--v5-sur2)', marginBottom: 12, animation: 'ph-v5-pulse 1.6s ease infinite' }} />
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: 8 }}>
          {[0, 1, 2, 3, 4, 5, 6].map((i) => (
            <div key={i} style={{ height: 160, borderRadius: 12, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.06}s` }} />
          ))}
        </div>
      </div>
    );
  }

  if (statsQuery.isError) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 28, textAlign: 'center', maxWidth: 360 }}>
          <V5NamedIcon name="x" size={26} color="var(--v5-err)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Fehler beim Laden</div>
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>Vorhersagen konnten nicht geladen werden.</div>
        </Card>
      </div>
    );
  }

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Vorhersagen</span>
        <Badge variant="purple">KI</Badge>
      </div>

      {recommendation && (
        <Card className="v5-ani" style={{ padding: 18, animationDelay: '0.06s', background: 'linear-gradient(145deg, var(--v5-acc-muted), transparent)', border: '1px solid color-mix(in oklch, var(--v5-acc) 30%, transparent)' }} data-testid="recommendation">
          <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.4, color: 'var(--v5-acc)', textTransform: 'uppercase' }}>
            Beste Buchungszeit
          </div>
          <div style={{ marginTop: 6, fontSize: 20, fontWeight: 700, color: 'var(--v5-txt)' }}>
            {recommendation.day}, {recommendation.timeSlot}
          </div>
          <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 4 }}>
            Basierend auf historischen Buchungsmustern und aktueller Auslastung.
          </div>
        </Card>
      )}

      <div className="v5-ani" style={{ animationDelay: '0.12s' }}>
        <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.4, color: 'var(--v5-mut)', textTransform: 'uppercase', marginBottom: 10 }}>
          7-Tage-Prognose
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(120px, 1fr))', gap: 8 }} data-testid="forecast-grid">
          {predictions.map((day) => (
            <Card
              key={day.dayIndex}
              data-testid="day-card"
              style={{ padding: 12, display: 'flex', flexDirection: 'column', gap: 8 }}
            >
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
                <span style={{ fontSize: 12, fontWeight: 700, color: 'var(--v5-txt)' }}>{day.dayShort}</span>
                <Badge variant={levelVariant(day.level)} dot>{levelLabel(day.level)}</Badge>
              </div>
              <div
                className="v5-mono"
                style={{ fontSize: 22, fontWeight: 800, color: levelColor(day.level), fontVariantNumeric: 'tabular-nums', letterSpacing: -1 }}
              >
                {day.predicted}%
              </div>
              <div style={{ height: 6, background: 'var(--v5-sur2)', borderRadius: 999, overflow: 'hidden' }}>
                <div style={{ height: '100%', width: `${day.predicted}%`, background: levelColor(day.level), borderRadius: 999 }} />
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 10 }}>
                <span style={{ color: 'var(--v5-mut)' }}>Spitze</span>
                <span className="v5-mono" style={{ color: 'var(--v5-err)', fontWeight: 500 }}>{formatHour(day.peakHour)}</span>
              </div>
              <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 10 }}>
                <span style={{ color: 'var(--v5-mut)' }}>Ruhig</span>
                <span className="v5-mono" style={{ color: 'var(--v5-ok)', fontWeight: 500 }}>{formatHour(day.offPeakHour)}</span>
              </div>
              <div style={{ fontSize: 10, color: 'var(--v5-mut)', textAlign: 'center', marginTop: 2 }}>
                {day.confidence}% Konfidenz
              </div>
            </Card>
          ))}
        </div>
      </div>

      <div style={{ fontSize: 10, color: 'var(--v5-mut)', textAlign: 'center', marginTop: 6 }}>
        Prognosen basieren auf historischen Mustern. Genauigkeit verbessert sich mit der Zeit.
      </div>
    </div>
  );
}
