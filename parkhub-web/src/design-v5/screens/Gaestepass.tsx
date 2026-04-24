import { useEffect, useMemo, useState, type CSSProperties, type ReactNode } from 'react';
import NumberFlow from '@number-flow/react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge, Card, V5NamedIcon } from '../primitives';
import { useV5Toast } from '../Toast';
import {
  api,
  type GuestBooking,
  type ParkingLot,
  type ParkingSlot,
  type CreateGuestBookingPayload,
} from '../../api/client';
import type { ScreenId } from '../nav';

// Backend (Rust `BookingStatus`, PHP `Booking::STATUS_*`) can return the full
// booking status enum for guest passes. Rust creates fresh guest bookings with
// `BookingStatus::Confirmed`, so `confirmed` is the most common newly-created
// status. `pending`, `completed` and `no_show` are rare but legal, and we map
// them to sensible visual fallbacks so the row is never blank.
const STATUS_LABEL: Record<GuestBooking['status'], string> = {
  pending: 'Ausstehend',
  confirmed: 'Bestätigt',
  active: 'Aktiv',
  completed: 'Abgeschlossen',
  expired: 'Abgelaufen',
  cancelled: 'Storniert',
  no_show: 'Nicht erschienen',
};

function statusVariant(s: GuestBooking['status']) {
  switch (s) {
    case 'pending': return 'warning' as const;
    case 'confirmed': return 'info' as const;
    case 'active': return 'success' as const;
    case 'completed': return 'gray' as const;
    case 'expired': return 'gray' as const;
    case 'cancelled': return 'error' as const;
    case 'no_show': return 'error' as const;
  }
}

// A guest pass is "open" (pre-use or in-use) when it is still actionable from
// the user's perspective — fresh passes land in `confirmed`, then flip to
// `active` while the time window is open. Both states should count as active
// and expose the Storno action.
function isOpenStatus(s: GuestBooking['status']): boolean {
  return s === 'confirmed' || s === 'active' || s === 'pending';
}

function formatRange(start: string, end: string): string {
  const s = new Date(start);
  const e = new Date(end);
  const d = s.toLocaleDateString('de-DE', { day: '2-digit', month: '2-digit', year: '2-digit' });
  const t1 = s.toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' });
  const t2 = e.toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' });
  return `${d} ${t1}–${t2}`;
}

const inputStyle: CSSProperties = {
  padding: '8px 11px',
  borderRadius: 9,
  background: 'var(--v5-sur2)',
  border: '1px solid var(--v5-bor)',
  color: 'var(--v5-txt)',
  fontSize: 12,
  width: '100%',
  outline: 'none',
  boxSizing: 'border-box',
  fontFamily: 'inherit',
};

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
      <label style={{ fontSize: 11, fontWeight: 500, color: 'var(--v5-mut)' }}>{label}</label>
      {children}
    </div>
  );
}

function CreateGuestModal({
  lots,
  onClose,
  onSubmit,
  isSubmitting,
}: {
  lots: ParkingLot[];
  onClose: () => void;
  onSubmit: (payload: CreateGuestBookingPayload) => void;
  isSubmitting: boolean;
}) {
  const toast = useV5Toast();
  const [guestName, setGuestName] = useState('');
  const [guestEmail, setGuestEmail] = useState('');
  const [lotId, setLotId] = useState('');
  const [slotId, setSlotId] = useState('');
  const [start, setStart] = useState('');
  const [end, setEnd] = useState('');

  const slotsQuery = useQuery({
    queryKey: ['guest-slots', lotId],
    enabled: !!lotId,
    queryFn: async () => {
      const res = await api.getLotSlots(lotId);
      if (!res.success) throw new Error(res.error?.message ?? 'Stellplätze konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
  });
  const slots: ParkingSlot[] = slotsQuery.data ?? [];
  const availableSlots = useMemo(() => slots.filter((s) => s.status === 'available'), [slots]);

  function submit() {
    if (!guestName.trim() || !lotId || !slotId || !start || !end) {
      toast('Bitte alle Pflichtfelder ausfüllen', 'error');
      return;
    }
    onSubmit({
      lot_id: lotId,
      slot_id: slotId,
      start_time: new Date(start).toISOString(),
      end_time: new Date(end).toISOString(),
      guest_name: guestName.trim(),
      guest_email: guestEmail.trim() || null,
    });
  }

  const disabled = isSubmitting || !guestName.trim() || !lotId || !slotId || !start || !end;

  return (
    <div role="dialog" aria-modal="true" aria-label="Gäste-Pass erstellen" style={{ position: 'fixed', inset: 0, zIndex: 1000, display: 'flex', alignItems: 'center', justifyContent: 'center', padding: 16 }}>
      <div aria-hidden="true" onClick={onClose} style={{ position: 'absolute', inset: 0, background: 'oklch(0 0 0 / 0.45)', backdropFilter: 'blur(6px)' }} />
      <Card lift={false} style={{ position: 'relative', width: '100%', maxWidth: 460, padding: 22, display: 'flex', flexDirection: 'column', gap: 12, zIndex: 1 }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div style={{ fontWeight: 700, fontSize: 14, color: 'var(--v5-txt)' }}>Gäste-Pass erstellen</div>
          <button type="button" aria-label="Schließen" onClick={onClose} style={{ width: 28, height: 28, borderRadius: 8, background: 'var(--v5-sur2)', border: 'none', cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <V5NamedIcon name="x" size={13} color="var(--v5-mut)" />
          </button>
        </div>
        <Field label="Gast-Name *">
          <input type="text" value={guestName} onChange={(e) => setGuestName(e.target.value)} autoFocus style={inputStyle} data-testid="guest-name" />
        </Field>
        <Field label="Gast-E-Mail">
          <input type="email" value={guestEmail} onChange={(e) => setGuestEmail(e.target.value)} style={inputStyle} data-testid="guest-email" />
        </Field>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
          <Field label="Parkhaus *">
            <select value={lotId} onChange={(e) => { setLotId(e.target.value); setSlotId(''); }} style={inputStyle} data-testid="guest-lot">
              <option value="">Bitte wählen…</option>
              {lots.map((l) => (
                <option key={l.id} value={l.id}>{l.name}</option>
              ))}
            </select>
          </Field>
          <Field label="Stellplatz *">
            <select value={slotId} onChange={(e) => setSlotId(e.target.value)} disabled={!lotId} style={inputStyle} data-testid="guest-slot">
              <option value="">{lotId ? 'Bitte wählen…' : 'Erst Parkhaus wählen'}</option>
              {availableSlots.map((s) => (
                <option key={s.id} value={s.id}>{s.slot_number}</option>
              ))}
            </select>
          </Field>
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
          <Field label="Start *">
            <input type="datetime-local" value={start} onChange={(e) => setStart(e.target.value)} style={inputStyle} data-testid="guest-start" />
          </Field>
          <Field label="Ende *">
            <input type="datetime-local" value={end} onChange={(e) => setEnd(e.target.value)} style={inputStyle} data-testid="guest-end" />
          </Field>
        </div>
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, paddingTop: 4 }}>
          <button type="button" onClick={onClose} style={{ padding: '8px 14px', borderRadius: 9, background: 'transparent', border: '1px solid var(--v5-bor)', color: 'var(--v5-mut)', fontSize: 12, cursor: 'pointer' }}>
            Abbrechen
          </button>
          <button
            type="button"
            disabled={disabled}
            onClick={submit}
            style={{
              padding: '8px 16px',
              borderRadius: 9,
              background: 'var(--v5-acc)',
              color: 'var(--v5-accent-fg)',
              border: 'none',
              fontSize: 12,
              fontWeight: 600,
              cursor: disabled ? 'default' : 'pointer',
              opacity: disabled ? 0.5 : 1,
            }}
            data-testid="guest-submit"
          >
            {isSubmitting ? 'Erstellen…' : 'Pass erstellen'}
          </button>
        </div>
      </Card>
    </div>
  );
}

export function GaestepassV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();
  const [showCreate, setShowCreate] = useState(false);
  const [createdCode, setCreatedCode] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const bookingsQuery = useQuery({
    queryKey: ['guest-bookings'],
    queryFn: async () => {
      const res = await api.getGuestBookings();
      if (!res.success) throw new Error(res.error?.message ?? 'Gäste-Pässe konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  });

  const lotsQuery = useQuery({
    queryKey: ['guest-lots'],
    queryFn: async () => {
      const res = await api.getLots();
      if (!res.success) throw new Error(res.error?.message ?? 'Parkhäuser konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 60_000,
  });

  const createMutation = useMutation({
    mutationFn: async (payload: CreateGuestBookingPayload) => {
      const res = await api.createGuestBooking(payload);
      if (!res.success) throw new Error(res.error?.message ?? 'Gäste-Pass erstellen fehlgeschlagen');
      return res.data;
    },
    onSuccess: (data) => {
      qc.invalidateQueries({ queryKey: ['guest-bookings'] });
      toast('Gäste-Pass erstellt', 'success');
      setShowCreate(false);
      if (data?.guest_code) setCreatedCode(data.guest_code);
    },
    onError: (err: Error) => toast(err.message, 'error'),
  });

  const cancelMutation = useMutation({
    mutationFn: async (id: string) => {
      const res = await api.cancelGuestBooking(id);
      if (!res.success) throw new Error(res.error?.message ?? 'Stornierung fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['guest-bookings'] });
      toast('Pass storniert', 'success');
    },
    onError: (err: Error) => toast(err.message, 'error'),
  });

  useEffect(() => {
    if (copied) {
      const timer = setTimeout(() => setCopied(false), 2000);
      return () => clearTimeout(timer);
    }
  }, [copied]);

  const bookings: GuestBooking[] = bookingsQuery.data ?? [];
  const lots: ParkingLot[] = lotsQuery.data ?? [];
  const activeCount = useMemo(() => bookings.filter((b) => isOpenStatus(b.status)).length, [bookings]);

  function copyCode(code: string) {
    if (typeof navigator !== 'undefined' && navigator.clipboard) {
      void navigator.clipboard.writeText(code).then(
        () => {
          setCopied(true);
          toast('Code kopiert', 'info');
        },
        () => toast('Kopieren fehlgeschlagen', 'error'),
      );
    }
  }

  if (bookingsQuery.isLoading || lotsQuery.isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto' }}>
        <div style={{ height: 32, width: 220, borderRadius: 8, background: 'var(--v5-sur2)', marginBottom: 14, animation: 'ph-v5-pulse 1.6s ease infinite' }} />
        {[0, 1, 2].map((i) => (
          <div key={i} style={{ height: 70, borderRadius: 12, background: 'var(--v5-sur2)', marginBottom: 8, animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.1}s` }} />
        ))}
      </div>
    );
  }

  if (bookingsQuery.isError || lotsQuery.isError) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 28, textAlign: 'center', maxWidth: 360 }}>
          <V5NamedIcon name="x" size={26} color="var(--v5-err)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Fehler beim Laden</div>
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>Gäste-Pässe konnten nicht geladen werden.</div>
        </Card>
      </div>
    );
  }

  return (
    <>
      <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
        <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Gäste-Pass</span>
            <Badge variant="gray"><NumberFlow value={bookings.length} /></Badge>
            {activeCount > 0 && <Badge variant="success" dot>{activeCount} aktiv</Badge>}
          </div>
          <button
            type="button"
            onClick={() => { setShowCreate(true); setCreatedCode(null); }}
            className="v5-btn"
            style={{ padding: '7px 14px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)', border: 'none', fontSize: 11, fontWeight: 600, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 5 }}
            data-testid="open-create-guest"
          >
            <V5NamedIcon name="plus" size={12} />
            Neuer Pass
          </button>
        </div>

        {createdCode && (
          <Card className="v5-ani" style={{ padding: 16, background: 'var(--v5-acc-muted)', border: '1px solid color-mix(in oklch, var(--v5-acc) 30%, transparent)' }} data-testid="created-code">
            <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.4, color: 'var(--v5-acc)', textTransform: 'uppercase' }}>
              Neuer Code
            </div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginTop: 6 }}>
              <code
                className="v5-mono"
                style={{ fontSize: 22, fontWeight: 800, color: 'var(--v5-acc)', letterSpacing: 2, background: 'var(--v5-sur)', padding: '6px 12px', borderRadius: 8 }}
              >
                {createdCode}
              </code>
              <button
                type="button"
                onClick={() => copyCode(createdCode)}
                style={{ padding: '6px 12px', borderRadius: 8, background: 'var(--v5-sur)', border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 11, fontWeight: 600, cursor: 'pointer' }}
                data-testid="copy-created-code"
              >
                {copied ? 'Kopiert' : 'Kopieren'}
              </button>
            </div>
            <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 6 }}>
              Teilen Sie diesen Code mit Ihrem Gast, um den Stellplatz freizuschalten.
            </div>
          </Card>
        )}

        {bookings.length === 0 ? (
          <Card className="v5-ani" style={{ padding: 36, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10, animationDelay: '0.06s' }}>
            <div style={{ width: 48, height: 48, borderRadius: 14, background: 'var(--v5-acc-muted)', border: '1.5px dashed color-mix(in oklch, var(--v5-acc) 50%, transparent)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <V5NamedIcon name="guest" size={20} color="var(--v5-acc)" />
            </div>
            <div style={{ textAlign: 'center' }}>
              <div style={{ fontWeight: 600, color: 'var(--v5-txt)', fontSize: 13 }}>Noch keine Gäste-Pässe</div>
              <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 3 }}>Erstellen Sie einen temporären Zugang für Besucher.</div>
            </div>
            <button
              type="button"
              onClick={() => { setShowCreate(true); setCreatedCode(null); }}
              className="v5-btn"
              style={{ padding: '8px 16px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)', border: 'none', fontSize: 11, fontWeight: 600, cursor: 'pointer' }}
            >
              + Pass erstellen
            </button>
          </Card>
        ) : (
          <Card className="v5-ani" style={{ overflow: 'hidden', animationDelay: '0.12s' }}>
            <div
              className="v5-mono"
              style={{
                display: 'grid',
                gridTemplateColumns: '1fr 110px 1fr 180px 90px',
                padding: '8px 16px',
                fontSize: 9,
                letterSpacing: 1.2,
                textTransform: 'uppercase',
                color: 'var(--v5-mut)',
                borderBottom: '1px solid var(--v5-bor)',
              }}
            >
              <span>Gast</span>
              <span>Code</span>
              <span>Parkhaus / Platz</span>
              <span>Zeitraum</span>
              <span>Aktion</span>
            </div>
            {bookings.map((b, i) => {
              const isCancelling = cancelMutation.isPending && cancelMutation.variables === b.id;
              return (
                <div
                  key={b.id}
                  data-testid="guest-row"
                  className="v5-row"
                  style={{
                    display: 'grid',
                    gridTemplateColumns: '1fr 110px 1fr 180px 90px',
                    padding: '10px 16px',
                    borderBottom: i < bookings.length - 1 ? '1px solid var(--v5-bor)' : 'none',
                    alignItems: 'center',
                    gap: 6,
                  }}
                >
                  <div>
                    <div style={{ fontSize: 12, fontWeight: 500, color: 'var(--v5-txt)' }}>{b.guest_name}</div>
                    <div style={{ fontSize: 10, color: 'var(--v5-mut)' }}>
                      {b.guest_email ?? 'Keine E-Mail'}
                    </div>
                  </div>
                  <code
                    className="v5-mono"
                    style={{ fontSize: 11, fontWeight: 700, color: 'var(--v5-acc)', letterSpacing: 1, cursor: 'pointer' }}
                    onClick={() => copyCode(b.guest_code)}
                    title="Klicken zum Kopieren"
                  >
                    {b.guest_code}
                  </code>
                  <div>
                    <div style={{ fontSize: 11, color: 'var(--v5-txt)' }}>{b.lot_name}</div>
                    <div style={{ fontSize: 10, color: 'var(--v5-mut)' }}>Platz {b.slot_number}</div>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                    <Badge variant={statusVariant(b.status)} dot>{STATUS_LABEL[b.status]}</Badge>
                    <span style={{ fontSize: 10, color: 'var(--v5-mut)' }}>{formatRange(b.start_time, b.end_time)}</span>
                  </div>
                  {isOpenStatus(b.status) ? (
                    <button
                      type="button"
                      disabled={isCancelling}
                      onClick={() => cancelMutation.mutate(b.id)}
                      style={{
                        padding: '4px 10px',
                        borderRadius: 7,
                        background: 'color-mix(in oklch, var(--v5-err) 8%, transparent)',
                        border: 'none',
                        fontSize: 10,
                        color: 'var(--v5-err)',
                        cursor: isCancelling ? 'default' : 'pointer',
                        opacity: isCancelling ? 0.5 : 1,
                        fontWeight: 500,
                      }}
                      data-testid={`cancel-${b.id}`}
                    >
                      {isCancelling ? '…' : 'Storno'}
                    </button>
                  ) : (
                    <span style={{ fontSize: 10, color: 'var(--v5-mut)' }}>—</span>
                  )}
                </div>
              );
            })}
          </Card>
        )}
      </div>
      {showCreate && (
        <CreateGuestModal
          lots={lots}
          onClose={() => setShowCreate(false)}
          onSubmit={(payload) => createMutation.mutate(payload)}
          isSubmitting={createMutation.isPending}
        />
      )}
    </>
  );
}
