import { useState, type CSSProperties, type ReactNode } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge, Card, V5NamedIcon } from '../primitives';
import { useV5Toast } from '../Toast';
import { api, type Vehicle, type CreateVehiclePayload } from '../../api/client';
import type { ScreenId } from '../nav';

const COLOR_SWATCH: Record<string, string> = {
  black: 'oklch(0.15 0 0)',
  white: 'oklch(0.99 0 0)',
  silver: 'oklch(0.72 0 0)',
  gray: 'oklch(0.55 0 0)',
  blue: 'oklch(0.52 0.18 255)',
  red: 'oklch(0.52 0.22 25)',
  green: 'oklch(0.55 0.17 160)',
  brown: 'oklch(0.42 0.10 55)',
  beige: 'oklch(0.82 0.06 80)',
  other: 'oklch(0.60 0 0)',
};

const COLOR_DEFS: { key: string; label: string }[] = [
  { key: 'black', label: 'Schwarz' },
  { key: 'white', label: 'Weiß' },
  { key: 'silver', label: 'Silber' },
  { key: 'gray', label: 'Grau' },
  { key: 'blue', label: 'Blau' },
  { key: 'red', label: 'Rot' },
  { key: 'green', label: 'Grün' },
  { key: 'brown', label: 'Braun' },
  { key: 'beige', label: 'Beige' },
  { key: 'other', label: 'Sonstiges' },
];

const EMPTY_FORM: CreateVehiclePayload = { plate: '', make: '', model: '', color: '' };

function isEvVehicle(v: Vehicle): boolean {
  const mm = `${v.make ?? ''} ${v.model ?? ''}`.toLowerCase();
  return /tesla|electric|e-tron|ioniq|id\.|taycan|leaf|bolt|ev/.test(mm);
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

function AddVehicleModal({
  onClose,
  onAdd,
  isAdding,
}: {
  onClose: () => void;
  onAdd: (payload: CreateVehiclePayload) => void;
  isAdding: boolean;
}) {
  const [form, setForm] = useState<CreateVehiclePayload>(EMPTY_FORM);
  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Fahrzeug hinzufügen"
      style={{ position: 'fixed', inset: 0, zIndex: 1000, display: 'flex', alignItems: 'center', justifyContent: 'center', padding: 16 }}
    >
      <div
        aria-hidden="true"
        onClick={onClose}
        style={{ position: 'absolute', inset: 0, background: 'oklch(0 0 0 / 0.45)', backdropFilter: 'blur(6px)' }}
      />
      <Card
        lift={false}
        style={{ position: 'relative', width: '100%', maxWidth: 420, padding: 22, display: 'flex', flexDirection: 'column', gap: 14, zIndex: 1 }}
      >
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div style={{ fontWeight: 700, fontSize: 14, color: 'var(--v5-txt)' }}>Fahrzeug hinzufügen</div>
          <button
            type="button"
            aria-label="Schließen"
            onClick={onClose}
            style={{ width: 28, height: 28, borderRadius: 8, background: 'var(--v5-sur2)', border: 'none', cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
          >
            <V5NamedIcon name="x" size={13} color="var(--v5-mut)" />
          </button>
        </div>
        <Field label="Kennzeichen *">
          <input
            type="text"
            value={form.plate}
            onChange={(e) => setForm({ ...form, plate: e.target.value.toUpperCase() })}
            placeholder="M-AB 1234"
            autoFocus
            style={inputStyle}
          />
        </Field>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
          <Field label="Marke">
            <input type="text" value={form.make ?? ''} onChange={(e) => setForm({ ...form, make: e.target.value })} placeholder="BMW" style={inputStyle} />
          </Field>
          <Field label="Modell">
            <input type="text" value={form.model ?? ''} onChange={(e) => setForm({ ...form, model: e.target.value })} placeholder="3er" style={inputStyle} />
          </Field>
        </div>
        <Field label="Farbe">
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 7, marginTop: 4 }}>
            {COLOR_DEFS.map((c) => {
              const selected = form.color === c.key;
              return (
                <button
                  key={c.key}
                  type="button"
                  aria-label={c.label}
                  aria-pressed={selected}
                  onClick={() => setForm({ ...form, color: selected ? '' : c.key })}
                  style={{
                    width: 26,
                    height: 26,
                    borderRadius: '50%',
                    background: COLOR_SWATCH[c.key] ?? 'var(--v5-mut)',
                    border: selected ? '2.5px solid var(--v5-acc)' : '1.5px solid var(--v5-bor)',
                    cursor: 'pointer',
                    transform: selected ? 'scale(1.15)' : 'scale(1)',
                    transition: 'transform 0.12s, border 0.12s',
                    outline: 'none',
                    boxSizing: 'border-box',
                    padding: 0,
                  }}
                />
              );
            })}
          </div>
        </Field>
        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, paddingTop: 4 }}>
          <button
            type="button"
            onClick={onClose}
            style={{ padding: '8px 14px', borderRadius: 9, background: 'transparent', border: '1px solid var(--v5-bor)', color: 'var(--v5-mut)', fontSize: 12, cursor: 'pointer' }}
          >
            Abbrechen
          </button>
          <button
            type="button"
            disabled={isAdding || !form.plate.trim()}
            onClick={() => onAdd(form)}
            style={{
              padding: '8px 16px',
              borderRadius: 9,
              background: 'var(--v5-acc)',
              color: 'var(--v5-accent-fg)',
              border: 'none',
              fontSize: 12,
              fontWeight: 600,
              cursor: isAdding || !form.plate.trim() ? 'default' : 'pointer',
              opacity: !form.plate.trim() ? 0.5 : 1,
            }}
          >
            {isAdding ? 'Speichern…' : 'Speichern'}
          </button>
        </div>
      </Card>
    </div>
  );
}

export function FahrzeugeV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();
  const [showAdd, setShowAdd] = useState(false);

  const { data: vehicles = [], isLoading, isError } = useQuery({
    queryKey: ['fahrzeuge'],
    queryFn: async () => {
      const res = await api.getVehicles();
      if (!res.success) throw new Error(res.error?.message ?? 'Fahrzeuge konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  });

  const addMutation = useMutation({
    mutationFn: async (payload: CreateVehiclePayload) => {
      const res = await api.createVehicle(payload);
      if (!res.success) throw new Error(res.error?.message ?? 'Fahrzeug hinzufügen fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['fahrzeuge'] });
      toast('Fahrzeug hinzugefügt', 'success');
      setShowAdd(false);
    },
    onError: () => toast('Hinzufügen fehlgeschlagen', 'error'),
  });

  // Optimistic delete: drop the vehicle from the cache immediately so the
  // card vanishes without waiting for the server. onError restores the
  // previous snapshot + toasts; onSettled re-invalidates to pick up any
  // server-side computed fields (e.g. `is_default` promotion).
  const deleteMutation = useMutation({
    mutationFn: async (id: string) => {
      const res = await api.deleteVehicle(id);
      if (!res.success) throw new Error(res.error?.message ?? 'Fahrzeug löschen fehlgeschlagen');
      return res.data;
    },
    onMutate: async (id: string) => {
      await qc.cancelQueries({ queryKey: ['fahrzeuge'] });
      const previous = qc.getQueryData<Vehicle[]>(['fahrzeuge']);
      if (previous) {
        qc.setQueryData<Vehicle[]>(['fahrzeuge'], previous.filter((v) => v.id !== id));
      }
      return { previous };
    },
    onError: (_err, _id, ctx) => {
      if (ctx?.previous) qc.setQueryData(['fahrzeuge'], ctx.previous);
      toast('Löschen fehlgeschlagen', 'error');
    },
    onSuccess: () => {
      toast('Fahrzeug entfernt', 'success');
    },
    onSettled: () => {
      qc.invalidateQueries({ queryKey: ['fahrzeuge'] });
    },
  });

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))', gap: 10, alignContent: 'start' }}>
        {[0, 1, 2, 3].map((i) => (
          <div key={i} style={{ height: 110, borderRadius: 14, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.08}s` }} />
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
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>Fahrzeuge konnten nicht geladen werden.</div>
        </Card>
      </div>
    );
  }

  return (
    <>
      <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
        <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Meine Fahrzeuge</span>
          <button
            type="button"
            onClick={() => setShowAdd(true)}
            className="v5-btn"
            style={{ padding: '7px 14px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)', border: 'none', fontSize: 11, fontWeight: 600, cursor: 'pointer', display: 'flex', alignItems: 'center', gap: 5 }}
          >
            <V5NamedIcon name="plus" size={12} />
            Hinzufügen
          </button>
        </div>
        {vehicles.length === 0 ? (
          <Card className="v5-ani" style={{ padding: 40, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 12, animationDelay: '0.06s' }}>
            <div style={{ width: 52, height: 52, borderRadius: 16, background: 'var(--v5-acc-muted)', border: '1.5px dashed color-mix(in oklch, var(--v5-acc) 50%, transparent)', display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
              <V5NamedIcon name="car" size={22} color="var(--v5-acc)" />
            </div>
            <div style={{ textAlign: 'center' }}>
              <div style={{ fontWeight: 600, color: 'var(--v5-txt)', fontSize: 13 }}>Noch keine Fahrzeuge</div>
              <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 3 }}>Fügen Sie Ihr erstes Fahrzeug hinzu.</div>
            </div>
            <button type="button" onClick={() => setShowAdd(true)} className="v5-btn" style={{ padding: '8px 16px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)', border: 'none', fontSize: 11, fontWeight: 600, cursor: 'pointer' }}>
              + Fahrzeug hinzufügen
            </button>
          </Card>
        ) : (
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))', gap: 10 }}>
            {vehicles.map((v, i) => {
              const ev = isEvVehicle(v);
              const swatch = v.color ? (COLOR_SWATCH[v.color] ?? 'var(--v5-mut)') : undefined;
              return (
                <Card
                  key={v.id}
                  data-testid="fahrzeug-card"
                  className="v5-ani"
                  style={{ padding: '14px 16px', display: 'flex', flexDirection: 'column', gap: 10, animationDelay: `${i * 0.06}s` }}
                >
                  <div style={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between' }}>
                    <div>
                      <div className="v5-mono" style={{ fontSize: 18, fontWeight: 700, color: 'var(--v5-txt)', letterSpacing: 1 }}>{v.plate}</div>
                      {(v.make || v.model) && (
                        <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginTop: 2 }}>
                          {[v.make, v.model].filter(Boolean).join(' ')}
                        </div>
                      )}
                    </div>
                    <button
                      type="button"
                      aria-label={`Fahrzeug ${v.plate} löschen`}
                      onClick={() => deleteMutation.mutate(v.id)}
                      disabled={deleteMutation.isPending}
                      style={{ width: 28, height: 28, borderRadius: 8, background: 'transparent', border: '1px solid var(--v5-bor)', cursor: 'pointer', display: 'flex', alignItems: 'center', justifyContent: 'center' }}
                    >
                      <V5NamedIcon name="x" size={12} color="var(--v5-err)" />
                    </button>
                  </div>
                  <div style={{ display: 'flex', gap: 6, alignItems: 'center', flexWrap: 'wrap' }}>
                    {swatch && (
                      <span
                        aria-hidden="true"
                        style={{ width: 12, height: 12, borderRadius: '50%', background: swatch, border: '1px solid var(--v5-bor)', display: 'inline-block', flexShrink: 0 }}
                      />
                    )}
                    {v.color && <span style={{ fontSize: 10, color: 'var(--v5-mut)' }}>{COLOR_DEFS.find((c) => c.key === v.color)?.label ?? v.color}</span>}
                    {ev && <Badge variant="ev">EV</Badge>}
                    {v.is_default && <Badge variant="primary">Standard</Badge>}
                  </div>
                </Card>
              );
            })}
          </div>
        )}
      </div>
      {showAdd && (
        <AddVehicleModal
          onClose={() => setShowAdd(false)}
          onAdd={(payload) => addMutation.mutate(payload)}
          isAdding={addMutation.isPending}
        />
      )}
    </>
  );
}
