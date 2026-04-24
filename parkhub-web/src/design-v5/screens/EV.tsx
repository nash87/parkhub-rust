import { useMemo, useState } from 'react';
import NumberFlow from '@number-flow/react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge, Card, V5NamedIcon } from '../primitives';
import { useV5Toast } from '../Toast';
import { api, type EvCharger, type ChargerConnector, type ChargerStatus, type ChargingSession } from '../../api/client';
import type { ScreenId } from '../nav';

const CONNECTOR_LABEL: Record<ChargerConnector, string> = {
  type2: 'Type 2',
  ccs: 'CCS',
  chademo: 'CHAdeMO',
  tesla: 'Tesla',
};

const STATUS_LABEL: Record<ChargerStatus, string> = {
  available: 'Verfügbar',
  in_use: 'Belegt',
  offline: 'Offline',
  maintenance: 'Wartung',
};

function statusVariant(s: ChargerStatus) {
  switch (s) {
    case 'available': return 'success' as const;
    case 'in_use': return 'warning' as const;
    case 'offline': return 'gray' as const;
    case 'maintenance': return 'error' as const;
  }
}

function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' });
}

export function EVV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();
  const [selectedLot, setSelectedLot] = useState<string>('');

  const lotsQuery = useQuery({
    queryKey: ['ev-lots'],
    queryFn: async () => {
      const res = await api.getLots();
      if (!res.success) throw new Error(res.error?.message ?? 'Parkhäuser konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 60_000,
    refetchOnWindowFocus: true,
  });

  const lots = lotsQuery.data ?? [];
  const activeLot = selectedLot || lots[0]?.id || '';

  const chargersQuery = useQuery({
    queryKey: ['ev-chargers', activeLot],
    enabled: !!activeLot,
    queryFn: async () => {
      const res = await api.getLotChargers(activeLot);
      if (!res.success) throw new Error(res.error?.message ?? 'Ladestationen konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 15_000,
    refetchOnWindowFocus: true,
  });

  const sessionsQuery = useQuery({
    queryKey: ['ev-sessions'],
    queryFn: async () => {
      const res = await api.getChargerSessions();
      if (!res.success) throw new Error(res.error?.message ?? 'Sitzungen konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 15_000,
    refetchOnWindowFocus: true,
  });

  const startMutation = useMutation({
    mutationFn: async (chargerId: string) => {
      const res = await api.startCharging(chargerId);
      if (!res.success) throw new Error(res.error?.message ?? 'Laden starten fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['ev-chargers'] });
      qc.invalidateQueries({ queryKey: ['ev-sessions'] });
      toast('Laden gestartet', 'success');
    },
    onError: (err: Error) => toast(err.message, 'error'),
  });

  const stopMutation = useMutation({
    mutationFn: async (chargerId: string) => {
      const res = await api.stopCharging(chargerId);
      if (!res.success) throw new Error(res.error?.message ?? 'Laden stoppen fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['ev-chargers'] });
      qc.invalidateQueries({ queryKey: ['ev-sessions'] });
      toast('Laden beendet', 'success');
    },
    onError: (err: Error) => toast(err.message, 'error'),
  });

  const isLoading = lotsQuery.isLoading || chargersQuery.isLoading || sessionsQuery.isLoading;
  const isError = lotsQuery.isError || chargersQuery.isError || sessionsQuery.isError;

  const chargers: EvCharger[] = chargersQuery.data ?? [];
  const sessions: ChargingSession[] = sessionsQuery.data ?? [];

  const activeSession = useMemo(() => {
    const map = new Map<string, ChargingSession>();
    for (const s of sessions) {
      if (s.status === 'active') map.set(s.charger_id, s);
    }
    return map;
  }, [sessions]);

  const availableCount = useMemo(() => chargers.filter((c) => c.status === 'available').length, [chargers]);
  const liveCount = useMemo(() => sessions.filter((s) => s.status === 'active').length, [sessions]);

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto' }}>
        <div style={{ height: 32, width: 200, borderRadius: 8, background: 'var(--v5-sur2)', marginBottom: 14, animation: 'ph-v5-pulse 1.6s ease infinite' }} />
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))', gap: 10 }}>
          {[0, 1, 2, 3].map((i) => (
            <div key={i} style={{ height: 130, borderRadius: 14, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.08}s` }} />
          ))}
        </div>
      </div>
    );
  }

  if (isError) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 28, textAlign: 'center', maxWidth: 360 }}>
          <V5NamedIcon name="x" size={26} color="var(--v5-err)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Fehler beim Laden</div>
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>Ladestationen konnten nicht geladen werden.</div>
        </Card>
      </div>
    );
  }

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 10, flexWrap: 'wrap' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>EV-Laden</span>
          <Badge variant="gray"><NumberFlow value={chargers.length} /></Badge>
        </div>
        {lots.length > 1 && (
          <select
            aria-label="Parkhaus"
            value={activeLot}
            onChange={(e) => setSelectedLot(e.target.value)}
            style={{
              padding: '6px 10px',
              borderRadius: 9,
              background: 'var(--v5-sur2)',
              border: '1px solid var(--v5-bor)',
              color: 'var(--v5-txt)',
              fontSize: 12,
              fontFamily: 'inherit',
              outline: 'none',
            }}
          >
            {lots.map((l) => (
              <option key={l.id} value={l.id}>{l.name}</option>
            ))}
          </select>
        )}
      </div>

      <div className="v5-ani" style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 10, animationDelay: '0.06s' }}>
        <StatTile label="Ladepunkte" value={chargers.length} />
        <StatTile label="Verfügbar" value={availableCount} accent />
        <StatTile label="Aktive Sitzungen" value={liveCount} />
      </div>

      {chargers.length === 0 ? (
        <Card className="v5-ani" style={{ padding: 36, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10, animationDelay: '0.12s' }}>
          <div style={{ width: 48, height: 48, borderRadius: 14, background: 'var(--v5-acc-muted)', border: '1.5px dashed color-mix(in oklch, var(--v5-acc) 50%, transparent)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <V5NamedIcon name="bolt" size={20} color="var(--v5-acc)" />
          </div>
          <div style={{ textAlign: 'center' }}>
            <div style={{ fontWeight: 600, color: 'var(--v5-txt)', fontSize: 13 }}>Keine Ladestationen</div>
            <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 3 }}>In diesem Parkhaus sind noch keine Ladesäulen verfügbar.</div>
          </div>
        </Card>
      ) : (
        <div className="v5-ani" style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))', gap: 10, animationDelay: '0.12s' }}>
          {chargers.map((c, i) => {
            const active = activeSession.get(c.id);
            const isStarting = startMutation.isPending && startMutation.variables === c.id;
            const isStopping = stopMutation.isPending && stopMutation.variables === c.id;
            return (
              <Card
                key={c.id}
                data-testid="charger-card"
                className="v5-ani"
                style={{ padding: '14px 16px', display: 'flex', flexDirection: 'column', gap: 10, animationDelay: `${i * 0.05}s` }}
              >
                <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between', gap: 8 }}>
                  <div>
                    <div style={{ fontSize: 14, fontWeight: 700, color: 'var(--v5-txt)' }}>{c.label}</div>
                    <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 2 }}>
                      {CONNECTOR_LABEL[c.connector_type]} · {c.power_kw} kW
                    </div>
                  </div>
                  <Badge variant={statusVariant(c.status)} dot>{STATUS_LABEL[c.status]}</Badge>
                </div>
                {c.location_hint && (
                  <div style={{ fontSize: 11, color: 'var(--v5-mut)' }}>{c.location_hint}</div>
                )}
                {active && (
                  <div style={{ fontSize: 11, color: 'var(--v5-acc)', display: 'flex', alignItems: 'center', gap: 6 }}>
                    <V5NamedIcon name="bolt" size={11} color="var(--v5-acc)" />
                    Laden seit {formatTime(active.start_time)}
                  </div>
                )}
                {c.status === 'available' && !active && (
                  <button
                    type="button"
                    disabled={isStarting}
                    onClick={() => startMutation.mutate(c.id)}
                    className="v5-btn"
                    style={{
                      padding: '8px 14px',
                      borderRadius: 9,
                      background: 'var(--v5-acc)',
                      color: 'var(--v5-accent-fg)',
                      border: 'none',
                      fontSize: 11,
                      fontWeight: 600,
                      cursor: isStarting ? 'default' : 'pointer',
                      opacity: isStarting ? 0.6 : 1,
                    }}
                  >
                    {isStarting ? 'Starten…' : 'Laden starten'}
                  </button>
                )}
                {active && (
                  <button
                    type="button"
                    disabled={isStopping}
                    onClick={() => stopMutation.mutate(c.id)}
                    style={{
                      padding: '8px 14px',
                      borderRadius: 9,
                      background: 'color-mix(in oklch, var(--v5-err) 10%, transparent)',
                      color: 'var(--v5-err)',
                      border: '1px solid color-mix(in oklch, var(--v5-err) 30%, transparent)',
                      fontSize: 11,
                      fontWeight: 600,
                      cursor: isStopping ? 'default' : 'pointer',
                      opacity: isStopping ? 0.6 : 1,
                    }}
                  >
                    {isStopping ? 'Stoppen…' : 'Laden beenden'}
                  </button>
                )}
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}

function StatTile({ label, value, accent = false }: { label: string; value: number; accent?: boolean }) {
  return (
    <Card style={{ padding: 14, background: accent ? 'var(--v5-acc-muted)' : 'var(--v5-sur)' }}>
      <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.4, color: accent ? 'var(--v5-acc)' : 'var(--v5-mut)', textTransform: 'uppercase' }}>
        {label}
      </div>
      <div style={{ marginTop: 6, fontSize: 28, fontWeight: 800, color: accent ? 'var(--v5-acc)' : 'var(--v5-txt)', letterSpacing: -1 }}>
        <NumberFlow value={value} />
      </div>
    </Card>
  );
}
