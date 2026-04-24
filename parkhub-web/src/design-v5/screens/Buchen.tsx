import { useMemo, useState, type CSSProperties, type ReactNode } from 'react';
import NumberFlow from '@number-flow/react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge, Card, SectionLabel, V5NamedIcon } from '../primitives';
import { useV5Toast } from '../Toast';
import {
  api,
  type ParkingLot,
  type ParkingSlot,
  type Vehicle,
  type CreateBookingPayload,
} from '../../api/client';
import type { ScreenId } from '../nav';

type Step = 1 | 2 | 3;

const DURATIONS: { label: string; hours: number }[] = [
  { label: '1h', hours: 1 },
  { label: '2h', hours: 2 },
  { label: '4h', hours: 4 },
  { label: '8h', hours: 8 },
];

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

function defaultStart(): string {
  const now = new Date();
  now.setMinutes(0, 0, 0);
  now.setHours(now.getHours() + 1);
  // datetime-local expects local-zone ISO (no TZ suffix). Build manually.
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}T${pad(now.getHours())}:${pad(now.getMinutes())}`;
}

function formatDateTime(d: Date): string {
  return d.toLocaleString('de-DE', {
    weekday: 'short', day: '2-digit', month: '2-digit',
    hour: '2-digit', minute: '2-digit',
  });
}

function formatTime(d: Date): string {
  return d.toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' });
}

export function BuchenV5({ navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();

  const [step, setStep] = useState<Step>(1);
  const [selectedLot, setSelectedLot] = useState<ParkingLot | null>(null);
  const [selectedSlot, setSelectedSlot] = useState<ParkingSlot | null>(null);
  const [selectedVehicle, setSelectedVehicle] = useState<string>('');
  const [startDate, setStartDate] = useState<string>(defaultStart);
  const [duration, setDuration] = useState<number>(2);

  const {
    data: lotsResp,
    isLoading: lotsLoading,
    isError: lotsError,
  } = useQuery({
    queryKey: ['buchen-lots'],
    queryFn: async () => {
      const res = await api.getLots();
      if (!res.success) throw new Error(res.error?.message ?? 'Parkplätze konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  });

  const { data: vehiclesResp } = useQuery({
    queryKey: ['buchen-vehicles'],
    queryFn: async () => {
      const res = await api.getVehicles();
      if (!res.success) throw new Error(res.error?.message ?? 'Fahrzeuge konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  });

  const lots = useMemo(() => (lotsResp ?? []).filter((l) => l.status === 'open'), [lotsResp]);
  const vehicles: Vehicle[] = vehiclesResp ?? [];

  const { data: slotsResp, isLoading: slotsLoading } = useQuery({
    queryKey: ['buchen-slots', selectedLot?.id],
    enabled: !!selectedLot,
    queryFn: async () => {
      const res = await api.getLotSlots(selectedLot!.id);
      if (!res.success) throw new Error(res.error?.message ?? 'Stellplätze konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 15_000,
  });
  const slots = slotsResp ?? [];

  const createMutation = useMutation({
    mutationFn: async (payload: CreateBookingPayload) => {
      const res = await api.createBooking(payload);
      if (!res.success) {
        const msg = res.error?.code === 'INSUFFICIENT_CREDITS'
          ? 'Nicht genug Credits'
          : res.error?.message || 'Buchung fehlgeschlagen';
        throw new Error(msg);
      }
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['buchungen'] });
      toast('Buchung bestätigt', 'success');
      navigate('buchungen');
    },
    onError: (err: Error) => toast(err.message || 'Buchung fehlgeschlagen', 'error'),
  });

  if (lotsLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto' }}>
        <div style={{ height: 32, width: 220, borderRadius: 8, background: 'var(--v5-sur2)', marginBottom: 14, animation: 'ph-v5-pulse 1.6s ease infinite' }} />
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))', gap: 10 }}>
          {[0, 1, 2].map((i) => (
            <div key={i} style={{ height: 120, borderRadius: 14, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.1}s` }} />
          ))}
        </div>
      </div>
    );
  }

  if (lotsError) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 28, textAlign: 'center', maxWidth: 360 }}>
          <V5NamedIcon name="x" size={26} color="var(--v5-err)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Fehler beim Laden</div>
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>Stellplätze konnten nicht geladen werden.</div>
        </Card>
      </div>
    );
  }

  const start = new Date(startDate);
  const end = new Date(start.getTime() + duration * 60 * 60 * 1000);
  const rate = selectedLot?.hourly_rate;
  const currency = selectedLot?.currency || '€';
  const estimated = rate != null ? (rate * duration).toFixed(2) : null;

  function handleSelectLot(lot: ParkingLot) {
    setSelectedLot(lot);
    setSelectedSlot(null);
    setStep(2);
  }

  function handleConfirm() {
    if (!selectedLot || !selectedSlot) return;
    createMutation.mutate({
      lot_id: selectedLot.id,
      slot_id: selectedSlot.id,
      start_time: start.toISOString(),
      end_time: end.toISOString(),
      vehicle_id: selectedVehicle || undefined,
    });
  }

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Platz buchen</span>
          <Badge variant="gray">Schritt {step}/3</Badge>
        </div>
        {step > 1 && (
          <button
            type="button"
            onClick={() => setStep((s) => (s === 3 ? 2 : 1) as Step)}
            style={{ padding: '6px 12px', borderRadius: 9, background: 'transparent', border: '1px solid var(--v5-bor)', color: 'var(--v5-mut)', fontSize: 11, cursor: 'pointer', fontWeight: 500 }}
          >
            ← Zurück
          </button>
        )}
      </div>

      <div className="v5-ani" role="list" aria-label="Fortschritt" style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 6, animationDelay: '0.06s' }}>
        {([1, 2, 3] as const).map((s) => {
          const active = s === step;
          const done = s < step;
          return (
            <div
              key={s}
              role="listitem"
              aria-current={active ? 'step' : undefined}
              style={{
                padding: '8px 12px',
                borderRadius: 10,
                border: `1.5px solid ${active || done ? 'var(--v5-acc)' : 'var(--v5-bor)'}`,
                background: active ? 'var(--v5-acc-muted)' : done ? 'color-mix(in oklch, var(--v5-acc) 6%, transparent)' : 'transparent',
                color: active || done ? 'var(--v5-acc)' : 'var(--v5-mut)',
                fontSize: 11,
                fontWeight: 500,
                display: 'flex',
                alignItems: 'center',
                gap: 6,
              }}
            >
              <span className="v5-mono" style={{ fontSize: 10 }}>0{s}</span>
              <span>
                {s === 1 ? 'Stellplatz' : s === 2 ? 'Zeit & Platz' : 'Bestätigen'}
              </span>
            </div>
          );
        })}
      </div>

      {step === 1 && (
        <StepLot lots={lots} onSelect={handleSelectLot} />
      )}
      {step === 2 && selectedLot && (
        <StepSlot
          lot={selectedLot}
          slots={slots}
          loading={slotsLoading}
          selectedSlot={selectedSlot}
          onSelectSlot={setSelectedSlot}
          startDate={startDate}
          onStartDateChange={setStartDate}
          duration={duration}
          onDurationChange={setDuration}
          vehicles={vehicles}
          selectedVehicle={selectedVehicle}
          onVehicleChange={setSelectedVehicle}
          onContinue={() => selectedSlot && setStep(3)}
          currency={currency}
          estimated={estimated}
        />
      )}
      {step === 3 && selectedLot && selectedSlot && (
        <StepConfirm
          lot={selectedLot}
          slot={selectedSlot}
          start={start}
          end={end}
          duration={duration}
          estimated={estimated}
          currency={currency}
          vehicle={vehicles.find((v) => v.id === selectedVehicle)}
          submitting={createMutation.isPending}
          onConfirm={handleConfirm}
        />
      )}
    </div>
  );
}

function StepLot({
  lots,
  onSelect,
}: {
  lots: ParkingLot[];
  onSelect: (lot: ParkingLot) => void;
}) {
  if (lots.length === 0) {
    return (
      <Card className="v5-ani" style={{ padding: 36, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10, animationDelay: '0.12s' }}>
        <div style={{ width: 48, height: 48, borderRadius: 14, background: 'var(--v5-acc-muted)', border: '1.5px dashed color-mix(in oklch, var(--v5-acc) 50%, transparent)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
          <V5NamedIcon name="map" size={20} color="var(--v5-acc)" />
        </div>
        <div style={{ textAlign: 'center' }}>
          <div style={{ fontWeight: 600, color: 'var(--v5-txt)', fontSize: 13 }}>Keine offenen Stellplätze</div>
          <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 3 }}>Derzeit stehen keine Parkflächen zur Verfügung.</div>
        </div>
      </Card>
    );
  }

  return (
    <div
      className="v5-ani"
      style={{
        display: 'grid',
        gridTemplateColumns: 'repeat(auto-fill, minmax(240px, 1fr))',
        gap: 10,
        animationDelay: '0.12s',
      }}
    >
      {lots.map((lot, i) => {
        const occupancy = lot.total_slots > 0 ? Math.round(((lot.total_slots - lot.available_slots) / lot.total_slots) * 100) : 0;
        const full = lot.available_slots === 0;
        return (
          <button
            key={lot.id}
            type="button"
            data-testid="buchen-lot-card"
            disabled={full}
            onClick={() => !full && onSelect(lot)}
            className="v5-lift v5-ani"
            style={{
              background: 'var(--v5-sur)',
              border: '1px solid var(--v5-bor)',
              borderRadius: 14,
              boxShadow: 'var(--v5-shadow-card)',
              padding: 16,
              display: 'flex',
              flexDirection: 'column',
              gap: 10,
              textAlign: 'left',
              cursor: full ? 'not-allowed' : 'pointer',
              opacity: full ? 0.55 : 1,
              animationDelay: `${i * 0.05}s`,
              fontFamily: 'inherit',
              color: 'inherit',
            }}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: 8 }}>
              <div style={{ minWidth: 0 }}>
                <div style={{ fontSize: 13, fontWeight: 600, color: 'var(--v5-txt)' }}>{lot.name}</div>
                {lot.address && (
                  <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 2, display: 'flex', alignItems: 'center', gap: 4 }}>
                    <V5NamedIcon name="map" size={10} />
                    <span style={{ overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{lot.address}</span>
                  </div>
                )}
              </div>
              {full ? <Badge variant="error">Voll</Badge> : <Badge variant="success" dot>Offen</Badge>}
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end' }}>
              <div>
                <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.2, color: 'var(--v5-mut)', textTransform: 'uppercase' }}>Verfügbar</div>
                <div className="v5-mono" style={{ fontSize: 22, fontWeight: 700, color: 'var(--v5-acc)' }}>
                  <NumberFlow value={lot.available_slots} /> / {lot.total_slots}
                </div>
              </div>
              <div style={{ textAlign: 'right' }}>
                <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.2, color: 'var(--v5-mut)', textTransform: 'uppercase' }}>Belegung</div>
                <div style={{ fontSize: 13, fontWeight: 600, color: 'var(--v5-txt)' }}>{occupancy}%</div>
              </div>
            </div>
            <div style={{ height: 4, background: 'var(--v5-sur2)', borderRadius: 4, overflow: 'hidden' }}>
              <div style={{ height: '100%', width: `${Math.max(4, 100 - occupancy)}%`, background: 'var(--v5-acc)', borderRadius: 4, transition: 'width 0.4s ease' }} />
            </div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: 11, color: 'var(--v5-mut)' }}>
              <span>
                {lot.hourly_rate != null ? `${lot.currency || '€'}${lot.hourly_rate.toFixed(2)}/h` : 'Preis auf Anfrage'}
              </span>
              <span style={{ color: 'var(--v5-acc)', fontWeight: 500 }}>Auswählen →</span>
            </div>
          </button>
        );
      })}
    </div>
  );
}

function StepSlot({
  lot,
  slots,
  loading,
  selectedSlot,
  onSelectSlot,
  startDate,
  onStartDateChange,
  duration,
  onDurationChange,
  vehicles,
  selectedVehicle,
  onVehicleChange,
  onContinue,
  currency,
  estimated,
}: {
  lot: ParkingLot;
  slots: ParkingSlot[];
  loading: boolean;
  selectedSlot: ParkingSlot | null;
  onSelectSlot: (s: ParkingSlot) => void;
  startDate: string;
  onStartDateChange: (v: string) => void;
  duration: number;
  onDurationChange: (v: number) => void;
  vehicles: Vehicle[];
  selectedVehicle: string;
  onVehicleChange: (v: string) => void;
  onContinue: () => void;
  currency: string;
  estimated: string | null;
}) {
  const available = slots.filter((s) => s.status === 'available');

  return (
    <div className="v5-ani" style={{ display: 'grid', gridTemplateColumns: 'minmax(0, 1fr) 280px', gap: 12, animationDelay: '0.12s' }}>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
        <Card style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 12 }}>
          <SectionLabel>{lot.name}</SectionLabel>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
            <Field label="Startzeit">
              <input
                id="buchen-start"
                type="datetime-local"
                value={startDate}
                onChange={(e) => onStartDateChange(e.target.value)}
                style={inputStyle}
              />
            </Field>
            <Field label="Dauer">
              <div role="group" aria-label="Dauer" style={{ display: 'flex', gap: 5 }}>
                {DURATIONS.map((d) => {
                  const active = duration === d.hours;
                  return (
                    <button
                      key={d.hours}
                      type="button"
                      aria-pressed={active}
                      onClick={() => onDurationChange(d.hours)}
                      style={{
                        flex: 1,
                        padding: '7px 0',
                        borderRadius: 8,
                        fontSize: 11,
                        fontWeight: 500,
                        cursor: 'pointer',
                        border: `1.5px solid ${active ? 'var(--v5-acc)' : 'var(--v5-bor)'}`,
                        background: active ? 'var(--v5-acc-muted)' : 'transparent',
                        color: active ? 'var(--v5-acc)' : 'var(--v5-mut)',
                        transition: 'all 0.15s',
                      }}
                    >
                      {d.label}
                    </button>
                  );
                })}
              </div>
            </Field>
          </div>
          {vehicles.length > 0 && (
            <Field label="Fahrzeug">
              <select
                id="buchen-vehicle"
                value={selectedVehicle}
                onChange={(e) => onVehicleChange(e.target.value)}
                style={inputStyle}
              >
                <option value="">— Kein Fahrzeug —</option>
                {vehicles.map((v) => (
                  <option key={v.id} value={v.id}>
                    {v.plate}{v.make ? ` · ${v.make}${v.model ? ` ${v.model}` : ''}` : ''}
                  </option>
                ))}
              </select>
            </Field>
          )}
        </Card>

        <Card style={{ padding: 16 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 10 }}>
            <SectionLabel>Stellplatz wählen</SectionLabel>
            <span style={{ fontSize: 11, color: 'var(--v5-mut)' }}>
              {available.length} von {slots.length} verfügbar
            </span>
          </div>
          {loading ? (
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(64px, 1fr))', gap: 6 }}>
              {Array.from({ length: 12 }, (_, i) => (
                <div key={i} style={{ height: 44, borderRadius: 8, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.03}s` }} />
              ))}
            </div>
          ) : available.length === 0 ? (
            <div style={{ padding: 24, textAlign: 'center', fontSize: 12, color: 'var(--v5-mut)' }}>
              Keine freien Plätze in diesem Zeitfenster.
            </div>
          ) : (
            <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(64px, 1fr))', gap: 6 }}>
              {slots.map((s) => {
                const isAvail = s.status === 'available';
                const isSelected = selectedSlot?.id === s.id;
                return (
                  <button
                    key={s.id}
                    type="button"
                    disabled={!isAvail}
                    aria-pressed={isSelected}
                    aria-label={`Stellplatz ${s.slot_number}${isAvail ? '' : ' (belegt)'}`}
                    onClick={() => isAvail && onSelectSlot(s)}
                    data-testid="buchen-slot"
                    style={{
                      padding: '10px 0',
                      borderRadius: 8,
                      fontSize: 12,
                      fontWeight: 600,
                      cursor: isAvail ? 'pointer' : 'not-allowed',
                      border: `1.5px solid ${isSelected ? 'var(--v5-acc)' : 'var(--v5-bor)'}`,
                      background: isSelected ? 'var(--v5-acc)' : isAvail ? 'var(--v5-sur)' : 'var(--v5-sur2)',
                      color: isSelected ? 'var(--v5-accent-fg)' : isAvail ? 'var(--v5-txt)' : 'var(--v5-mut)',
                      opacity: isAvail ? 1 : 0.55,
                      transition: 'all 0.12s',
                    }}
                  >
                    {s.slot_number}
                  </button>
                );
              })}
            </div>
          )}
        </Card>
      </div>

      <Card style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 10, alignSelf: 'start' }}>
        <SectionLabel>Zusammenfassung</SectionLabel>
        <SummaryRow label="Stellplatz" value={lot.name} />
        <SummaryRow label="Platz" value={selectedSlot?.slot_number ?? '—'} />
        <SummaryRow label="Von" value={formatDateTime(new Date(startDate))} />
        <SummaryRow label="Dauer" value={`${duration}h`} />
        <SummaryRow
          label="Tarif"
          value={lot.hourly_rate != null ? `${currency}${lot.hourly_rate.toFixed(2)}/h` : '—'}
        />
        <SummaryRow
          label="Kosten"
          value={estimated ? `${currency}${estimated}` : '—'}
          bold
        />
        <button
          type="button"
          disabled={!selectedSlot}
          onClick={onContinue}
          style={{
            padding: '10px 14px',
            borderRadius: 10,
            background: 'var(--v5-acc)',
            color: 'var(--v5-accent-fg)',
            border: 'none',
            fontSize: 12,
            fontWeight: 600,
            cursor: selectedSlot ? 'pointer' : 'not-allowed',
            opacity: selectedSlot ? 1 : 0.5,
            marginTop: 4,
          }}
        >
          Weiter →
        </button>
      </Card>
    </div>
  );
}

function StepConfirm({
  lot,
  slot,
  start,
  end,
  duration,
  estimated,
  currency,
  vehicle,
  submitting,
  onConfirm,
}: {
  lot: ParkingLot;
  slot: ParkingSlot;
  start: Date;
  end: Date;
  duration: number;
  estimated: string | null;
  currency: string;
  vehicle?: Vehicle;
  submitting: boolean;
  onConfirm: () => void;
}) {
  return (
    <Card className="v5-ani" style={{ padding: 22, display: 'flex', flexDirection: 'column', gap: 14, animationDelay: '0.12s' }}>
      <div>
        <SectionLabel>Bestätigung</SectionLabel>
        <div style={{ fontSize: 18, fontWeight: 700, color: 'var(--v5-txt)', marginTop: 4 }}>
          Bereit zum Buchen
        </div>
        <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>
          Überprüfen Sie alle Details vor der Bestätigung.
        </div>
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
        <SummaryTile label="Stellplatz" value={lot.name} />
        <SummaryTile label="Platz" value={slot.slot_number} />
        <SummaryTile label="Von" value={formatDateTime(start)} />
        <SummaryTile label="Bis" value={formatTime(end)} />
        <SummaryTile label="Dauer" value={`${duration}h`} />
        <SummaryTile label="Fahrzeug" value={vehicle ? vehicle.plate : '—'} />
        <SummaryTile
          label="Tarif"
          value={lot.hourly_rate != null ? `${currency}${lot.hourly_rate.toFixed(2)}/h` : '—'}
        />
        <SummaryTile
          label="Geschätzte Kosten"
          value={estimated ? `${currency}${estimated}` : '—'}
          emphasis
        />
      </div>
      <button
        type="button"
        disabled={submitting}
        onClick={onConfirm}
        data-testid="buchen-confirm"
        style={{
          padding: '11px 16px',
          borderRadius: 10,
          background: 'var(--v5-acc)',
          color: 'var(--v5-accent-fg)',
          border: 'none',
          fontSize: 13,
          fontWeight: 600,
          cursor: submitting ? 'default' : 'pointer',
          opacity: submitting ? 0.7 : 1,
          marginTop: 4,
        }}
      >
        {submitting ? 'Bestätige …' : 'Buchung bestätigen'}
      </button>
    </Card>
  );
}

function SummaryRow({ label, value, bold = false }: { label: string; value: string; bold?: boolean }) {
  return (
    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', fontSize: 12 }}>
      <span style={{ color: 'var(--v5-mut)' }}>{label}</span>
      <span style={{ color: 'var(--v5-txt)', fontWeight: bold ? 700 : 500 }}>{value}</span>
    </div>
  );
}

function SummaryTile({ label, value, emphasis = false }: { label: string; value: string; emphasis?: boolean }) {
  return (
    <div
      style={{
        padding: '10px 12px',
        borderRadius: 10,
        background: emphasis ? 'var(--v5-acc-muted)' : 'var(--v5-sur2)',
        border: `1px solid ${emphasis ? 'color-mix(in oklch, var(--v5-acc) 30%, transparent)' : 'var(--v5-bor)'}`,
      }}
    >
      <div className="v5-mono" style={{ fontSize: 9, letterSpacing: 1.2, color: 'var(--v5-mut)', textTransform: 'uppercase' }}>
        {label}
      </div>
      <div style={{ fontSize: 13, fontWeight: 600, color: emphasis ? 'var(--v5-acc)' : 'var(--v5-txt)', marginTop: 3 }}>
        {value}
      </div>
    </div>
  );
}
