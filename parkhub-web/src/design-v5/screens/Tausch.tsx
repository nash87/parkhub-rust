import { useMemo, useState, type CSSProperties, type ReactNode } from 'react';
import NumberFlow from '@number-flow/react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge, Card, V5NamedIcon } from '../primitives';
import { useV5Toast } from '../Toast';
import { api, type Booking, type SwapRequest } from '../../api/client';
import type { ScreenId } from '../nav';

const STATUS_LABEL: Record<SwapRequest['status'], string> = {
  pending: 'Offen',
  accepted: 'Angenommen',
  declined: 'Abgelehnt',
};

function statusVariant(s: SwapRequest['status']) {
  switch (s) {
    case 'pending': return 'warning' as const;
    case 'accepted': return 'success' as const;
    case 'declined': return 'error' as const;
  }
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString('de-DE', { day: '2-digit', month: '2-digit', year: '2-digit' });
}
function formatTime(iso: string): string {
  return new Date(iso).toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' });
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

function CreateSwapModal({
  bookings,
  onClose,
  onSubmit,
  isSubmitting,
}: {
  bookings: Booking[];
  onClose: () => void;
  onSubmit: (sourceId: string, targetId: string, message: string | null) => void;
  isSubmitting: boolean;
}) {
  const [sourceId, setSourceId] = useState('');
  const [targetId, setTargetId] = useState('');
  const [message, setMessage] = useState('');
  const disabled = !sourceId || !targetId || isSubmitting;
  return (
    <div role="dialog" aria-modal="true" aria-label="Tausch anfragen" style={{ position: 'fixed', inset: 0, zIndex: 1000, display: 'flex', alignItems: 'center', justifyContent: 'center', padding: 16 }}>
      <div aria-hidden="true" onClick={onClose} style={{ position: 'absolute', inset: 0, background: 'oklch(0 0 0 / 0.45)', backdropFilter: 'blur(6px)' }} />
      <Card lift={false} style={{ position: 'relative', width: '100%', maxWidth: 440, padding: 22, display: 'flex', flexDirection: 'column', gap: 14, zIndex: 1 }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div style={{ fontWeight: 700, fontSize: 14, color: 'var(--v5-txt)' }}>Tausch anfragen</div>
          <button type="button" aria-label="Schließen" onClick={onClose} style={{ width: 28, height: 28, borderRadius: 8, background: 'var(--v5-sur2)', border: 'none', cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
            <V5NamedIcon name="x" size={13} color="var(--v5-mut)" />
          </button>
        </div>
        <Field label="Ihre Buchung *">
          <select value={sourceId} onChange={(e) => setSourceId(e.target.value)} style={inputStyle} data-testid="swap-source">
            <option value="">Bitte wählen…</option>
            {bookings.map((b) => (
              <option key={b.id} value={b.id}>
                {b.lot_name} · Platz {b.slot_number} ({formatDate(b.start_time)} {formatTime(b.start_time)})
              </option>
            ))}
          </select>
        </Field>
        <Field label="Ziel-Buchungs-ID *">
          <input type="text" value={targetId} onChange={(e) => setTargetId(e.target.value)} placeholder="b-…" style={inputStyle} data-testid="swap-target" />
        </Field>
        <Field label="Nachricht (optional)">
          <textarea value={message} onChange={(e) => setMessage(e.target.value)} rows={3} style={{ ...inputStyle, resize: 'vertical' }} data-testid="swap-message" />
        </Field>
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, paddingTop: 4 }}>
          <button type="button" onClick={onClose} style={{ padding: '8px 14px', borderRadius: 9, background: 'transparent', border: '1px solid var(--v5-bor)', color: 'var(--v5-mut)', fontSize: 12, cursor: 'pointer' }}>
            Abbrechen
          </button>
          <button
            type="button"
            disabled={disabled}
            onClick={() => onSubmit(sourceId, targetId, message.trim() || null)}
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
            data-testid="swap-submit"
          >
            {isSubmitting ? 'Senden…' : 'Anfrage senden'}
          </button>
        </div>
      </Card>
    </div>
  );
}

export function TauschV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();
  const [showCreate, setShowCreate] = useState(false);

  const requestsQuery = useQuery({
    queryKey: ['swap-requests'],
    queryFn: async () => {
      const res = await api.getSwapRequests();
      if (!res.success) throw new Error(res.error?.message ?? 'Tauschanfragen konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  });

  const bookingsQuery = useQuery({
    queryKey: ['swap-bookings'],
    queryFn: async () => {
      const res = await api.getBookings();
      if (!res.success) throw new Error(res.error?.message ?? 'Buchungen konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  });

  const acceptMutation = useMutation({
    mutationFn: async (id: string) => {
      const res = await api.acceptSwap(id);
      if (!res.success) throw new Error(res.error?.message ?? 'Annahme fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['swap-requests'] });
      toast('Tausch angenommen', 'success');
    },
    onError: (err: Error) => toast(err.message, 'error'),
  });

  const declineMutation = useMutation({
    mutationFn: async (id: string) => {
      const res = await api.declineSwap(id);
      if (!res.success) throw new Error(res.error?.message ?? 'Ablehnung fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['swap-requests'] });
      toast('Tausch abgelehnt', 'success');
    },
    onError: (err: Error) => toast(err.message, 'error'),
  });

  const createMutation = useMutation({
    mutationFn: async (vars: { source: string; target: string; message: string | null }) => {
      const res = await api.createSwapRequest(vars.source, vars.target, vars.message);
      if (!res.success) throw new Error(res.error?.message ?? 'Anfrage senden fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['swap-requests'] });
      toast('Tauschanfrage gesendet', 'success');
      setShowCreate(false);
    },
    onError: (err: Error) => toast(err.message, 'error'),
  });

  const isLoading = requestsQuery.isLoading || bookingsQuery.isLoading;
  const isError = requestsQuery.isError || bookingsQuery.isError;

  const requests: SwapRequest[] = requestsQuery.data ?? [];
  const bookings: Booking[] = bookingsQuery.data ?? [];
  const activeBookings = useMemo(
    () => bookings.filter((b) => b.status === 'active' || b.status === 'confirmed'),
    [bookings],
  );
  const pendingCount = useMemo(() => requests.filter((r) => r.status === 'pending').length, [requests]);

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto' }}>
        <div style={{ height: 32, width: 200, borderRadius: 8, background: 'var(--v5-sur2)', marginBottom: 14, animation: 'ph-v5-pulse 1.6s ease infinite' }} />
        {[0, 1, 2].map((i) => (
          <div key={i} style={{ height: 90, borderRadius: 12, background: 'var(--v5-sur2)', marginBottom: 8, animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.1}s` }} />
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
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>Tauschanfragen konnten nicht geladen werden.</div>
        </Card>
      </div>
    );
  }

  return (
    <>
      <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
        <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
            <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Tausch</span>
            <Badge variant="gray"><NumberFlow value={requests.length} /></Badge>
            {pendingCount > 0 && <Badge variant="warning" dot>{pendingCount} offen</Badge>}
          </div>
          <button
            type="button"
            onClick={() => setShowCreate(true)}
            className="v5-btn"
            disabled={activeBookings.length === 0}
            style={{
              padding: '7px 14px',
              borderRadius: 9,
              background: 'var(--v5-acc)',
              color: 'var(--v5-accent-fg)',
              border: 'none',
              fontSize: 11,
              fontWeight: 600,
              cursor: activeBookings.length === 0 ? 'default' : 'pointer',
              opacity: activeBookings.length === 0 ? 0.5 : 1,
              display: 'flex', alignItems: 'center', gap: 5,
            }}
            data-testid="open-create-swap"
          >
            <V5NamedIcon name="swap" size={12} />
            Neue Anfrage
          </button>
        </div>

        {requests.length === 0 ? (
          <Card className="v5-ani" style={{ padding: 36, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10, animationDelay: '0.06s' }}>
            <div style={{ width: 48, height: 48, borderRadius: 14, background: 'var(--v5-acc-muted)', border: '1.5px dashed color-mix(in oklch, var(--v5-acc) 50%, transparent)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <V5NamedIcon name="swap" size={20} color="var(--v5-acc)" />
            </div>
            <div style={{ textAlign: 'center' }}>
              <div style={{ fontWeight: 600, color: 'var(--v5-txt)', fontSize: 13 }}>Keine Tauschanfragen</div>
              <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 3 }}>Tauschen Sie Ihren Stellplatz mit einem Teammitglied.</div>
            </div>
          </Card>
        ) : (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            {requests.map((r, i) => {
              const isAccepting = acceptMutation.isPending && acceptMutation.variables === r.id;
              const isDeclining = declineMutation.isPending && declineMutation.variables === r.id;
              return (
                <Card
                  key={r.id}
                  data-testid="swap-row"
                  className="v5-ani"
                  style={{ padding: 14, animationDelay: `${i * 0.05}s` }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 10 }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8, minWidth: 0 }}>
                      <V5NamedIcon name="swap" size={14} color="var(--v5-acc)" />
                      <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--v5-txt)' }}>
                        {r.source_booking.lot_name}
                      </span>
                      <span style={{ color: 'var(--v5-mut)' }}>→</span>
                      <span style={{ fontSize: 12, fontWeight: 600, color: 'var(--v5-txt)' }}>
                        {r.target_booking.lot_name}
                      </span>
                    </div>
                    <Badge variant={statusVariant(r.status)} dot>{STATUS_LABEL[r.status]}</Badge>
                  </div>
                  <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10, marginBottom: 10 }}>
                    <SlotPanel label="Ihr Platz" lot={r.source_booking.lot_name} slot={r.source_booking.slot_number} start={r.source_booking.start_time} end={r.source_booking.end_time} />
                    <SlotPanel label="Ziel" lot={r.target_booking.lot_name} slot={r.target_booking.slot_number} start={r.target_booking.start_time} end={r.target_booking.end_time} />
                  </div>
                  {r.message && (
                    <div style={{ fontSize: 11, color: 'var(--v5-mut)', fontStyle: 'italic', marginBottom: 10 }}>
                      „{r.message}"
                    </div>
                  )}
                  {r.status === 'pending' && (
                    <div style={{ display: 'flex', gap: 6 }}>
                      <button
                        type="button"
                        disabled={isAccepting || isDeclining}
                        onClick={() => acceptMutation.mutate(r.id)}
                        style={{
                          padding: '6px 12px',
                          borderRadius: 8,
                          background: 'var(--v5-acc)',
                          color: 'var(--v5-accent-fg)',
                          border: 'none',
                          fontSize: 11,
                          fontWeight: 600,
                          cursor: isAccepting ? 'default' : 'pointer',
                          opacity: isAccepting ? 0.6 : 1,
                        }}
                        data-testid={`accept-${r.id}`}
                      >
                        {isAccepting ? '…' : 'Annehmen'}
                      </button>
                      <button
                        type="button"
                        disabled={isAccepting || isDeclining}
                        onClick={() => declineMutation.mutate(r.id)}
                        style={{
                          padding: '6px 12px',
                          borderRadius: 8,
                          background: 'color-mix(in oklch, var(--v5-err) 10%, transparent)',
                          color: 'var(--v5-err)',
                          border: '1px solid color-mix(in oklch, var(--v5-err) 30%, transparent)',
                          fontSize: 11,
                          fontWeight: 600,
                          cursor: isDeclining ? 'default' : 'pointer',
                          opacity: isDeclining ? 0.6 : 1,
                        }}
                        data-testid={`decline-${r.id}`}
                      >
                        {isDeclining ? '…' : 'Ablehnen'}
                      </button>
                    </div>
                  )}
                </Card>
              );
            })}
          </div>
        )}
      </div>
      {showCreate && (
        <CreateSwapModal
          bookings={activeBookings}
          onClose={() => setShowCreate(false)}
          onSubmit={(source, target, message) => createMutation.mutate({ source, target, message })}
          isSubmitting={createMutation.isPending}
        />
      )}
    </>
  );
}

function SlotPanel({ label, lot, slot, start, end }: { label: string; lot: string; slot: string; start: string; end: string }) {
  return (
    <div style={{ padding: 10, borderRadius: 10, background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)' }}>
      <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.2, color: 'var(--v5-mut)', textTransform: 'uppercase', marginBottom: 4 }}>{label}</div>
      <div style={{ fontSize: 12, fontWeight: 500, color: 'var(--v5-txt)' }}>{lot}</div>
      <div style={{ fontSize: 10, color: 'var(--v5-mut)', marginTop: 2 }}>
        Platz {slot} · {formatDate(start)} {formatTime(start)}–{formatTime(end)}
      </div>
    </div>
  );
}
