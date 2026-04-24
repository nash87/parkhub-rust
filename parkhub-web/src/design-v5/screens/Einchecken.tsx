import { useEffect, useMemo, useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge, Card, V5NamedIcon } from '../primitives';
import { useV5Toast } from '../Toast';
import { api, type Booking } from '../../api/client';
import type { ScreenId } from '../nav';

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString('de-DE', { day: '2-digit', month: '2-digit', year: 'numeric' });
}
function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' });
}

function padded(n: number): string {
  return String(n).padStart(2, '0');
}

function elapsedSince(iso: string): string {
  const diff = Math.max(0, Date.now() - new Date(iso).getTime());
  const h = Math.floor(diff / 3600000);
  const m = Math.floor((diff % 3600000) / 60000);
  const s = Math.floor((diff % 60000) / 1000);
  return `${padded(h)}:${padded(m)}:${padded(s)}`;
}

export function EincheckenV5({ navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();

  const bookingsQuery = useQuery({
    queryKey: ['einchecken-bookings'],
    queryFn: async () => {
      const res = await api.getBookings();
      if (!res.success) throw new Error(res.error?.message ?? 'Buchungen konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  });

  const bookings: Booking[] = bookingsQuery.data ?? [];
  const activeBooking = useMemo(() => {
    const now = Date.now();
    return bookings.find((b) =>
      (b.status === 'active' || b.status === 'confirmed') &&
      new Date(b.start_time).getTime() <= now &&
      new Date(b.end_time).getTime() > now,
    ) ?? null;
  }, [bookings]);

  const statusQuery = useQuery({
    queryKey: ['checkin-status', activeBooking?.id],
    enabled: !!activeBooking,
    queryFn: async () => {
      const res = await api.getCheckInStatus(activeBooking!.id);
      if (!res.success) {
        // Not all backends return a status row when the user has never checked in.
        return { checked_in: false, checked_in_at: null, checked_out_at: null };
      }
      return res.data ?? { checked_in: false, checked_in_at: null, checked_out_at: null };
    },
    staleTime: 15_000,
    refetchOnWindowFocus: true,
  });

  const checkInMutation = useMutation({
    mutationFn: async (bookingId: string) => {
      const res = await api.checkIn(bookingId);
      if (!res.success) throw new Error(res.error?.message ?? 'Einchecken fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['checkin-status'] });
      toast('Eingecheckt', 'success');
    },
    onError: (err: Error) => toast(err.message, 'error'),
  });

  const checkOutMutation = useMutation({
    mutationFn: async (bookingId: string) => {
      const res = await api.checkOut(bookingId);
      if (!res.success) throw new Error(res.error?.message ?? 'Auschecken fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['checkin-status'] });
      toast('Ausgecheckt', 'success');
    },
    onError: (err: Error) => toast(err.message, 'error'),
  });

  const status = statusQuery.data;
  const isCheckedIn = !!status?.checked_in && !status?.checked_out_at;

  const [elapsed, setElapsed] = useState('00:00:00');
  useEffect(() => {
    if (!isCheckedIn || !status?.checked_in_at) {
      setElapsed('00:00:00');
      return;
    }
    const iso = status.checked_in_at;
    setElapsed(elapsedSince(iso));
    const tick = setInterval(() => setElapsed(elapsedSince(iso)), 1000);
    return () => clearInterval(tick);
  }, [isCheckedIn, status?.checked_in_at]);

  if (bookingsQuery.isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto' }}>
        <div style={{ height: 140, borderRadius: 14, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite', marginBottom: 12 }} />
        <div style={{ height: 90, borderRadius: 14, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite' }} />
      </div>
    );
  }

  if (bookingsQuery.isError) {
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

  if (!activeBooking) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 36, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10, maxWidth: 380 }}>
          <div style={{ width: 48, height: 48, borderRadius: 14, background: 'var(--v5-acc-muted)', border: '1.5px dashed color-mix(in oklch, var(--v5-acc) 50%, transparent)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <V5NamedIcon name="check" size={20} color="var(--v5-acc)" />
          </div>
          <div style={{ textAlign: 'center' }}>
            <div style={{ fontWeight: 600, color: 'var(--v5-txt)', fontSize: 13 }}>Keine aktive Buchung</div>
            <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 3 }}>Sie können einchecken, sobald eine Buchung läuft.</div>
          </div>
          <button
            type="button"
            onClick={() => navigate('buchen')}
            className="v5-btn"
            style={{ padding: '8px 16px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)', border: 'none', fontSize: 11, fontWeight: 600, cursor: 'pointer' }}
          >
            + Platz buchen
          </button>
        </Card>
      </div>
    );
  }

  const isActing = checkInMutation.isPending || checkOutMutation.isPending;

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Einchecken</span>
        <Badge variant={isCheckedIn ? 'success' : 'gray'} dot>
          {isCheckedIn ? 'Eingecheckt' : 'Bereit'}
        </Badge>
      </div>

      <Card className="v5-ani" style={{ padding: 18, animationDelay: '0.06s' }} data-testid="booking-card">
        <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.4, color: 'var(--v5-mut)', textTransform: 'uppercase' }}>
          Aktive Buchung
        </div>
        <div style={{ marginTop: 6, fontSize: 18, fontWeight: 700, color: 'var(--v5-txt)' }}>
          {activeBooking.lot_name ?? '—'}
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 10, marginTop: 12 }}>
          <Info label="Platz" value={activeBooking.slot_number ?? '—'} mono />
          <Info label="Datum" value={formatDate(activeBooking.start_time)} />
          <Info label="Zeitfenster" value={`${formatTime(activeBooking.start_time)}–${formatTime(activeBooking.end_time)}`} />
        </div>
      </Card>

      {isCheckedIn ? (
        <Card className="v5-ani" style={{ padding: 18, animationDelay: '0.12s', background: 'var(--v5-acc-muted)' }} data-testid="checked-in-card">
          <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.4, color: 'var(--v5-acc)', textTransform: 'uppercase' }}>
            Laufzeit
          </div>
          <div
            className="v5-mono"
            data-testid="elapsed"
            style={{ marginTop: 6, fontSize: 36, fontWeight: 800, color: 'var(--v5-acc)', letterSpacing: -1, fontVariantNumeric: 'tabular-nums' }}
          >
            {elapsed}
          </div>
          {status?.checked_in_at && (
            <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 4 }}>
              Seit {formatTime(status.checked_in_at)}
            </div>
          )}
          <button
            type="button"
            disabled={isActing}
            onClick={() => checkOutMutation.mutate(activeBooking.id)}
            style={{
              marginTop: 14,
              padding: '10px 18px',
              borderRadius: 10,
              background: 'color-mix(in oklch, var(--v5-err) 10%, transparent)',
              color: 'var(--v5-err)',
              border: '1px solid color-mix(in oklch, var(--v5-err) 30%, transparent)',
              fontSize: 12,
              fontWeight: 700,
              cursor: isActing ? 'default' : 'pointer',
              opacity: isActing ? 0.6 : 1,
            }}
            data-testid="checkout-btn"
          >
            {checkOutMutation.isPending ? 'Auschecken…' : 'Auschecken'}
          </button>
        </Card>
      ) : (
        <Card className="v5-ani" style={{ padding: 18, animationDelay: '0.12s', textAlign: 'center' }} data-testid="checkin-card">
          <V5NamedIcon name="check" size={36} color="var(--v5-acc)" />
          <div style={{ marginTop: 8, fontSize: 13, fontWeight: 600, color: 'var(--v5-txt)' }}>
            Bereit zum Einchecken
          </div>
          <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 4 }}>
            Scannen Sie den QR-Code am Stellplatz oder tippen Sie auf Einchecken.
          </div>
          <button
            type="button"
            disabled={isActing}
            onClick={() => checkInMutation.mutate(activeBooking.id)}
            className="v5-btn"
            style={{
              marginTop: 14,
              padding: '10px 22px',
              borderRadius: 10,
              background: 'var(--v5-acc)',
              color: 'var(--v5-accent-fg)',
              border: 'none',
              fontSize: 12,
              fontWeight: 700,
              cursor: isActing ? 'default' : 'pointer',
              opacity: isActing ? 0.6 : 1,
            }}
            data-testid="checkin-btn"
          >
            {checkInMutation.isPending ? 'Einchecken…' : 'Einchecken'}
          </button>
        </Card>
      )}
    </div>
  );
}

function Info({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div>
      <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.2, color: 'var(--v5-mut)', textTransform: 'uppercase' }}>
        {label}
      </div>
      <div
        className={mono ? 'v5-mono' : ''}
        style={{ marginTop: 4, fontSize: 13, fontWeight: 600, color: 'var(--v5-txt)' }}
      >
        {value}
      </div>
    </div>
  );
}
