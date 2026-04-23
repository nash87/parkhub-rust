import NumberFlow from '@number-flow/react';
import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge, Card, V5NamedIcon } from '../primitives';
import { useV5Toast } from '../Toast';
import { api, type Booking } from '../../api/client';
import type { ScreenId } from '../nav';

type FilterKey = 'alle' | 'active' | 'confirmed' | 'completed' | 'cancelled';

const FILTERS: { key: FilterKey; label: string }[] = [
  { key: 'alle', label: 'Alle' },
  { key: 'active', label: 'Aktiv' },
  { key: 'confirmed', label: 'Bestätigt' },
  { key: 'completed', label: 'Abgeschlossen' },
  { key: 'cancelled', label: 'Storniert' },
];

type BadgeVariant = 'primary' | 'success' | 'warning' | 'error' | 'info' | 'gray' | 'ev' | 'purple';

function statusVariant(s: Booking['status']): BadgeVariant {
  switch (s) {
    case 'active': return 'success';
    case 'confirmed': return 'primary';
    case 'completed': return 'gray';
    case 'cancelled': return 'error';
    default: return 'gray';
  }
}

const STATUS_LABEL: Record<Booking['status'], string> = {
  active: 'Aktiv',
  confirmed: 'Bestätigt',
  completed: 'Abgeschlossen',
  cancelled: 'Storniert',
};

function formatDateTime(iso: string): string {
  return new Date(iso).toLocaleString('de-DE', {
    day: '2-digit', month: '2-digit', year: 'numeric',
    hour: '2-digit', minute: '2-digit',
  });
}

function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' });
}

export function BuchungenV5({ navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();
  const [filter, setFilter] = useState<FilterKey>('alle');

  const { data: bookings = [], isLoading, isError } = useQuery({
    queryKey: ['buchungen'],
    queryFn: async () => {
      const res = await api.getBookings();
      return res.data ?? [];
    },
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  });

  const cancelMutation = useMutation({
    mutationFn: (id: string) => api.cancelBooking(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['buchungen'] });
      toast('Buchung storniert', 'success');
    },
    onError: () => toast('Stornierung fehlgeschlagen', 'error'),
  });

  const filtered = filter === 'alle' ? bookings : bookings.filter((b) => b.status === filter);

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto' }}>
        <div style={{ height: 32, width: 220, borderRadius: 8, background: 'var(--v5-sur2)', marginBottom: 14, animation: 'ph-v5-pulse 1.6s ease infinite' }} />
        {[0, 1, 2].map((i) => (
          <div key={i} style={{ height: 60, borderRadius: 12, background: 'var(--v5-sur2)', marginBottom: 8, animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.1}s` }} />
        ))}
      </div>
    );
  }

  if (isError) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 28, textAlign: 'center', maxWidth: 360 }}>
          <V5NamedIcon name="x" size={26} color="var(--v5-err)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Fehler beim Laden</div>
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>Buchungen konnten nicht geladen werden.</div>
        </Card>
      </div>
    );
  }

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Buchungen</span>
          <Badge variant="gray"><NumberFlow value={filtered.length} /></Badge>
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

      <div
        className="v5-ani"
        role="group"
        aria-label="Status-Filter"
        style={{ display: 'flex', gap: 6, flexWrap: 'wrap', animationDelay: '0.06s' }}
      >
        {FILTERS.map((f) => {
          const active = filter === f.key;
          return (
            <button
              key={f.key}
              type="button"
              aria-pressed={active}
              onClick={() => setFilter(f.key)}
              style={{
                padding: '5px 12px',
                borderRadius: 999,
                fontSize: 11,
                fontWeight: 500,
                cursor: 'pointer',
                border: `1.5px solid ${active ? 'var(--v5-acc)' : 'var(--v5-bor)'}`,
                background: active ? 'var(--v5-acc-muted)' : 'transparent',
                color: active ? 'var(--v5-acc)' : 'var(--v5-mut)',
                transition: 'all 0.15s',
              }}
            >
              {f.label}
            </button>
          );
        })}
      </div>

      {filtered.length === 0 ? (
        <Card
          className="v5-ani"
          style={{ padding: 36, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10, animationDelay: '0.12s' }}
        >
          <div style={{ width: 48, height: 48, borderRadius: 14, background: 'var(--v5-acc-muted)', border: '1.5px dashed color-mix(in oklch, var(--v5-acc) 50%, transparent)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <V5NamedIcon name="cal" size={20} color="var(--v5-acc)" />
          </div>
          <div style={{ textAlign: 'center' }}>
            <div style={{ fontWeight: 600, color: 'var(--v5-txt)', fontSize: 13 }}>Keine Buchungen gefunden</div>
            <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 3 }}>
              {filter === 'alle' ? 'Reservieren Sie jetzt einen Parkplatz.' : `Keine Buchungen mit Status „${FILTERS.find((f) => f.key === filter)?.label}".`}
            </div>
          </div>
          {filter === 'alle' && (
            <button type="button" onClick={() => navigate('buchen')} className="v5-btn" style={{ padding: '8px 16px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)', border: 'none', fontSize: 11, fontWeight: 600, cursor: 'pointer' }}>
              + Platz buchen
            </button>
          )}
        </Card>
      ) : (
        <Card className="v5-ani" style={{ overflow: 'hidden', animationDelay: '0.12s' }}>
          <div
            className="v5-mono"
            style={{
              display: 'grid',
              gridTemplateColumns: '90px 1fr 100px 150px 90px 110px 100px',
              padding: '8px 16px',
              fontSize: 9,
              letterSpacing: 1.2,
              textTransform: 'uppercase',
              color: 'var(--v5-mut)',
              borderBottom: '1px solid var(--v5-bor)',
            }}
          >
            <span>ID</span>
            <span>Stellplatz</span>
            <span>Kennz.</span>
            <span>Datum / Zeit</span>
            <span>Typ</span>
            <span>Status</span>
            <span>Aktionen</span>
          </div>
          {filtered.map((b, i) => {
            const isActive = b.status === 'active';
            const isCancellable = isActive || b.status === 'confirmed';
            const isCancellingThis = cancelMutation.isPending && cancelMutation.variables === b.id;
            return (
              <div
                key={b.id}
                data-testid="buchungen-row"
                className="v5-row"
                style={{
                  display: 'grid',
                  gridTemplateColumns: '90px 1fr 100px 150px 90px 110px 100px',
                  padding: '10px 16px',
                  borderBottom: i < filtered.length - 1 ? '1px solid var(--v5-bor)' : 'none',
                  alignItems: 'center',
                  gap: 4,
                }}
              >
                <span className="v5-mono" style={{ fontSize: 10, color: 'var(--v5-mut)' }} title={b.id}>
                  {b.id.slice(0, 8)}…
                </span>
                <div>
                  <div style={{ fontSize: 12, fontWeight: 500, color: 'var(--v5-txt)' }}>{b.lot_name ?? '—'}</div>
                  <div style={{ fontSize: 10, color: 'var(--v5-mut)' }}>Platz {b.slot_number ?? '?'}</div>
                </div>
                <span className="v5-mono" style={{ fontSize: 11, color: 'var(--v5-txt)', fontWeight: 500 }}>
                  {b.vehicle_plate ?? '—'}
                </span>
                <div>
                  <div style={{ fontSize: 11, color: 'var(--v5-txt)' }}>{formatDateTime(b.start_time)}</div>
                  <div style={{ fontSize: 10, color: 'var(--v5-mut)' }}>– {formatTime(b.end_time)}</div>
                </div>
                <span style={{ fontSize: 11, color: 'var(--v5-mut)' }}>{b.booking_type ?? 'Standard'}</span>
                <Badge variant={statusVariant(b.status)} dot>{STATUS_LABEL[b.status] ?? b.status}</Badge>
                <div style={{ display: 'flex', gap: 5, alignItems: 'center' }}>
                  {isActive && (
                    <button
                      type="button"
                      onClick={() => {
                        navigate('einchecken');
                        toast('Einchecken geöffnet', 'info');
                      }}
                      style={{ padding: '3px 8px', borderRadius: 6, background: 'var(--v5-acc-muted)', border: 'none', fontSize: 10, color: 'var(--v5-acc)', cursor: 'pointer', fontWeight: 500 }}
                    >
                      Check-in
                    </button>
                  )}
                  {isCancellable && (
                    <button
                      type="button"
                      disabled={isCancellingThis}
                      aria-label={`Buchung ${b.id} stornieren`}
                      onClick={() => cancelMutation.mutate(b.id)}
                      style={{ padding: '3px 8px', borderRadius: 6, background: 'color-mix(in oklch, var(--v5-err) 8%, transparent)', border: 'none', fontSize: 10, color: 'var(--v5-err)', cursor: isCancellingThis ? 'default' : 'pointer', opacity: isCancellingThis ? 0.5 : 1, fontWeight: 500 }}
                    >
                      {isCancellingThis ? '…' : 'Storno'}
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </Card>
      )}
    </div>
  );
}
