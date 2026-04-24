import { useMemo, useState } from 'react';
import NumberFlow from '@number-flow/react';
import { useQuery } from '@tanstack/react-query';
import { Badge, Card, SectionLabel, V5NamedIcon } from '../primitives';
import { api, type LotMarker, type MarkerColor } from '../../api/client';
import type { ScreenId } from '../nav';

const MARKER_COLOR: Record<MarkerColor, string> = {
  green: 'var(--v5-ok)',
  yellow: 'var(--v5-warn)',
  red: 'var(--v5-err)',
  gray: 'var(--v5-mut)',
};

const COLOR_LABEL: Record<MarkerColor, string> = {
  green: 'Viel frei',
  yellow: 'Knapp',
  red: 'Voll',
  gray: 'Geschlossen',
};

const COLOR_VARIANT: Record<MarkerColor, 'success' | 'warning' | 'error' | 'gray'> = {
  green: 'success',
  yellow: 'warning',
  red: 'error',
  gray: 'gray',
};

export function KarteV5({ navigate }: { navigate: (id: ScreenId) => void }) {
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const { data: markers = [], isLoading, isError } = useQuery({
    queryKey: ['karte'],
    queryFn: async () => {
      const res = await api.getMapMarkers();
      if (!res.success) throw new Error(res.error?.message ?? 'Karte konnte nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  });

  const totals = useMemo(() => {
    const free = markers.reduce((sum, m) => sum + m.available_slots, 0);
    const total = markers.reduce((sum, m) => sum + m.total_slots, 0);
    const occupancy = total > 0 ? Math.round(((total - free) / total) * 100) : 0;
    return { free, total, occupancy };
  }, [markers]);

  const selected = selectedId ? markers.find((m) => m.id === selectedId) ?? null : markers[0] ?? null;

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
        <div style={{ height: 32, width: 220, borderRadius: 8, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite' }} />
        <div style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) 280px', gap: 12 }}>
          <div style={{ height: 380, borderRadius: 14, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite' }} />
          <div style={{ height: 380, borderRadius: 14, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite' }} />
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
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>Karte konnte nicht geladen werden.</div>
        </Card>
      </div>
    );
  }

  if (markers.length === 0) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 36, textAlign: 'center', maxWidth: 380, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10 }}>
          <div style={{ width: 48, height: 48, borderRadius: 14, background: 'var(--v5-acc-muted)', border: '1.5px dashed color-mix(in oklch, var(--v5-acc) 50%, transparent)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <V5NamedIcon name="map" size={20} color="var(--v5-acc)" />
          </div>
          <div>
            <div style={{ fontWeight: 600, color: 'var(--v5-txt)', fontSize: 13 }}>Keine Standorte</div>
            <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 3 }}>Es sind noch keine Parkplätze kartiert.</div>
          </div>
        </Card>
      </div>
    );
  }

  // Compute lat/lng bounds for the simple visual
  const lats = markers.map((m) => m.latitude);
  const lngs = markers.map((m) => m.longitude);
  const minLat = Math.min(...lats);
  const maxLat = Math.max(...lats);
  const minLng = Math.min(...lngs);
  const maxLng = Math.max(...lngs);
  const latSpan = Math.max(0.001, maxLat - minLat);
  const lngSpan = Math.max(0.001, maxLng - minLng);

  function posFor(m: LotMarker): { left: string; top: string } {
    // Normalise to a 0-100 box with 8% inset padding
    const leftPct = 8 + ((m.longitude - minLng) / lngSpan) * 84;
    const topPct = 8 + ((maxLat - m.latitude) / latSpan) * 84;
    return { left: `${leftPct}%`, top: `${topPct}%` };
  }

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Karte</span>
          <Badge variant="gray">
            <NumberFlow value={markers.length} /> Standorte
          </Badge>
        </div>
        <button
          type="button"
          onClick={() => navigate('buchen')}
          className="v5-btn"
          style={{ padding: '7px 14px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)', border: 'none', fontSize: 11, fontWeight: 600, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 5 }}
        >
          <V5NamedIcon name="plus" size={12} />
          Platz buchen
        </button>
      </div>

      <div className="v5-ani" style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 10, animationDelay: '0.06s' }}>
        <SummaryStat label="Frei" value={totals.free} icon="check" />
        <SummaryStat label="Gesamt" value={totals.total} icon="list" />
        <SummaryStat label="Belegung" value={totals.occupancy} suffix="%" icon="trend" />
      </div>

      <div className="v5-ani" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) 300px', gap: 12, animationDelay: '0.12s' }}>
        <Card style={{ padding: 0, overflow: 'hidden' }}>
          <div
            role="img"
            aria-label="Standortübersicht"
            style={{
              position: 'relative',
              width: '100%',
              height: 380,
              background:
                'repeating-linear-gradient(0deg, var(--v5-sur2) 0 1px, transparent 1px 40px), repeating-linear-gradient(90deg, var(--v5-sur2) 0 1px, transparent 1px 40px), var(--v5-sur)',
            }}
          >
            {markers.map((m) => {
              const { left, top } = posFor(m);
              const active = selected?.id === m.id;
              return (
                <button
                  key={m.id}
                  type="button"
                  data-testid="karte-marker"
                  aria-label={`${m.name} – ${m.available_slots} von ${m.total_slots} frei`}
                  aria-pressed={active}
                  onClick={() => setSelectedId(m.id)}
                  style={{
                    position: 'absolute',
                    left,
                    top,
                    transform: 'translate(-50%, -100%)',
                    width: active ? 30 : 22,
                    height: active ? 30 : 22,
                    borderRadius: '50% 50% 50% 0',
                    rotate: '-45deg',
                    background: MARKER_COLOR[m.color],
                    border: '2px solid var(--v5-sur)',
                    boxShadow: active ? '0 4px 14px oklch(0 0 0 / 0.25)' : '0 1px 4px oklch(0 0 0 / 0.15)',
                    cursor: 'pointer',
                    padding: 0,
                    transition: 'all 0.15s',
                  }}
                />
              );
            })}
          </div>
        </Card>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          {selected && (
            <Card style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 10 }}>
              <SectionLabel>Ausgewählter Standort</SectionLabel>
              <div>
                <div style={{ fontSize: 14, fontWeight: 700, color: 'var(--v5-txt)' }}>{selected.name}</div>
                {selected.address && (
                  <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 2 }}>{selected.address}</div>
                )}
              </div>
              <div style={{ display: 'flex', gap: 6, alignItems: 'center', flexWrap: 'wrap' }}>
                <Badge variant={COLOR_VARIANT[selected.color]} dot>
                  {COLOR_LABEL[selected.color]}
                </Badge>
                <span className="v5-mono" style={{ fontSize: 11, color: 'var(--v5-mut)' }}>
                  {selected.available_slots} / {selected.total_slots} frei
                </span>
              </div>
              <OccupancyBar free={selected.available_slots} total={selected.total_slots} />
            </Card>
          )}

          <Card style={{ padding: 0, overflow: 'hidden' }}>
            <div style={{ padding: '12px 16px 8px' }}>
              <SectionLabel>Alle Standorte</SectionLabel>
            </div>
            <div style={{ maxHeight: 260, overflow: 'auto' }}>
              {markers.map((m, i) => {
                const active = selected?.id === m.id;
                return (
                  <button
                    key={m.id}
                    type="button"
                    data-testid="karte-list-row"
                    onClick={() => setSelectedId(m.id)}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 10,
                      width: '100%',
                      padding: '10px 16px',
                      borderTop: i === 0 ? 'none' : '1px solid var(--v5-bor)',
                      background: active ? 'var(--v5-acc-muted)' : 'transparent',
                      border: 'none',
                      borderRadius: 0,
                      textAlign: 'left',
                      cursor: 'pointer',
                      fontFamily: 'inherit',
                      color: 'inherit',
                    }}
                  >
                    <span style={{ width: 10, height: 10, borderRadius: '50%', background: MARKER_COLOR[m.color], flexShrink: 0 }} />
                    <div style={{ minWidth: 0, flex: 1 }}>
                      <div style={{ fontSize: 12, fontWeight: 500, color: 'var(--v5-txt)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                        {m.name}
                      </div>
                      <div style={{ fontSize: 10, color: 'var(--v5-mut)', marginTop: 1 }}>
                        {m.available_slots} / {m.total_slots} frei
                      </div>
                    </div>
                  </button>
                );
              })}
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}

function OccupancyBar({ free, total }: { free: number; total: number }) {
  const pct = total > 0 ? Math.round((free / total) * 100) : 0;
  return (
    <div>
      <div style={{ height: 4, background: 'var(--v5-sur2)', borderRadius: 4, overflow: 'hidden' }}>
        <div
          style={{
            height: '100%',
            width: `${Math.max(4, pct)}%`,
            background: 'var(--v5-acc)',
            borderRadius: 4,
            transition: 'width 0.6s ease',
          }}
        />
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 4, fontSize: 10, color: 'var(--v5-mut)' }}>
        <span>{pct}% frei</span>
        <span>{total - free} belegt</span>
      </div>
    </div>
  );
}

function SummaryStat({ label, value, suffix, icon }: { label: string; value: number; suffix?: string; icon: 'check' | 'list' | 'trend' }) {
  return (
    <Card style={{ padding: '12px 14px' }}>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div>
          <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.3, color: 'var(--v5-mut)', textTransform: 'uppercase' }}>
            {label}
          </div>
          <div className="v5-mono" style={{ fontSize: 22, fontWeight: 700, color: 'var(--v5-txt)', marginTop: 4 }}>
            <NumberFlow value={value} />
            {suffix ?? ''}
          </div>
        </div>
        <div style={{ width: 26, height: 26, borderRadius: 8, background: 'var(--v5-sur2)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <V5NamedIcon name={icon} size={12} color="var(--v5-mut)" />
        </div>
      </div>
    </Card>
  );
}
