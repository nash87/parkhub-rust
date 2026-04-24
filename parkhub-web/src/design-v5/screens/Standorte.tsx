import { useState } from 'react';
import NumberFlow from '@number-flow/react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge, Card, SectionLabel, V5NamedIcon, type BadgeVariant } from '../primitives';
import { useV5Toast } from '../Toast';
import { api, type ParkingLot, type LotStatus } from '../../api/client';
import type { ScreenId } from '../nav';

function statusVariant(s: string): BadgeVariant {
  switch (s) {
    case 'open': return 'success';
    case 'closed': return 'gray';
    case 'full': return 'warning';
    case 'maintenance': return 'error';
    default: return 'gray';
  }
}

const STATUS_LABEL: Record<string, string> = {
  open: 'Offen', closed: 'Geschlossen', full: 'Voll', maintenance: 'Wartung',
};

export function StandorteV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();
  const [formOpen, setFormOpen] = useState(false);
  const [newName, setNewName] = useState('');
  const [newSlots, setNewSlots] = useState('');

  const { data: lots = [], isLoading, isError } = useQuery({
    queryKey: ['standorte'],
    queryFn: async () => {
      const res = await api.getLots();
      if (!res.success) throw new Error(res.error?.message ?? 'Standorte konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
  });

  const createLot = useMutation({
    mutationFn: async (payload: { name: string; total_slots: number }) => {
      const res = await api.createLot({ name: payload.name, total_slots: payload.total_slots });
      if (!res.success) throw new Error(res.error?.message ?? 'Anlegen fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['standorte'] });
      toast('Standort angelegt', 'success');
      setFormOpen(false); setNewName(''); setNewSlots('');
    },
    onError: (err: Error) => toast(err.message || 'Anlegen fehlgeschlagen', 'error'),
  });

  const updateStatus = useMutation({
    mutationFn: async (payload: { id: string; status: LotStatus }) => {
      const res = await api.updateLot(payload.id, { status: payload.status });
      if (!res.success) throw new Error(res.error?.message ?? 'Speichern fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['standorte'] });
      toast('Status aktualisiert', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Speichern fehlgeschlagen', 'error'),
  });

  const deleteLot = useMutation({
    mutationFn: async (id: string) => {
      const res = await api.deleteLot(id);
      if (!res.success) throw new Error(res.error?.message ?? 'Löschen fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['standorte'] });
      toast('Standort gelöscht', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Löschen fehlgeschlagen', 'error'),
  });

  const canCreate = newName.trim().length > 1 && Number(newSlots) > 0;

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto' }}>
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
        </Card>
      </div>
    );
  }

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Standorte</span>
          <Badge variant="gray"><NumberFlow value={lots.length} /></Badge>
        </div>
        <button
          type="button" onClick={() => setFormOpen((o) => !o)} data-testid="standorte-toggle-form"
          style={{
            padding: '7px 14px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)',
            border: 'none', fontSize: 11, fontWeight: 600, cursor: 'pointer',
            display: 'flex', alignItems: 'center', gap: 5,
          }}
        >
          <V5NamedIcon name="plus" size={12} />{formOpen ? 'Schließen' : 'Neuer Standort'}
        </button>
      </div>

      {formOpen && (
        <Card className="v5-ani" style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 10 }}>
          <SectionLabel>Neuer Standort</SectionLabel>
          <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: 10 }}>
            <label style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              <span style={{ fontSize: 11, color: 'var(--v5-mut)' }}>Name</span>
              <input
                data-testid="standorte-name"
                value={newName} onChange={(e) => setNewName(e.target.value)}
                style={{ padding: '8px 11px', borderRadius: 9, background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 12, outline: 'none', fontFamily: 'inherit' }}
              />
            </label>
            <label style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
              <span style={{ fontSize: 11, color: 'var(--v5-mut)' }}>Kapazität</span>
              <input
                data-testid="standorte-slots"
                type="number" min="1"
                value={newSlots} onChange={(e) => setNewSlots(e.target.value)}
                style={{ padding: '8px 11px', borderRadius: 9, background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 12, outline: 'none', fontFamily: 'inherit' }}
              />
            </label>
          </div>
          <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 6 }}>
            <button
              type="button"
              disabled={!canCreate || createLot.isPending}
              onClick={() => createLot.mutate({ name: newName.trim(), total_slots: Number(newSlots) })}
              data-testid="standorte-create"
              style={{
                padding: '8px 16px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)',
                border: 'none', fontSize: 12, fontWeight: 600,
                cursor: canCreate && !createLot.isPending ? 'pointer' : 'not-allowed',
                opacity: canCreate && !createLot.isPending ? 1 : 0.5,
              }}
            >{createLot.isPending ? 'Legt an …' : 'Anlegen'}</button>
          </div>
        </Card>
      )}

      {lots.length === 0 ? (
        <Card className="v5-ani" style={{ padding: 36, textAlign: 'center', animationDelay: '0.12s' }}>
          <V5NamedIcon name="map" size={20} color="var(--v5-mut)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Keine Standorte</div>
        </Card>
      ) : (
        <Card className="v5-ani" style={{ overflow: 'hidden', animationDelay: '0.12s' }}>
          {lots.map((l: ParkingLot, i) => {
            const delBusy = deleteLot.isPending && deleteLot.variables === l.id;
            const statBusy = updateStatus.isPending && updateStatus.variables?.id === l.id;
            return (
              <div key={l.id} data-testid="standorte-row" style={{
                display: 'grid', gridTemplateColumns: '1fr 100px 110px 140px 100px',
                padding: '12px 16px', alignItems: 'center', gap: 8,
                borderBottom: i < lots.length - 1 ? '1px solid var(--v5-bor)' : 'none',
              }}>
                <div>
                  <div style={{ fontSize: 12, fontWeight: 500, color: 'var(--v5-txt)' }}>{l.name}</div>
                  {l.address && <div style={{ fontSize: 10, color: 'var(--v5-mut)' }}>{l.address}</div>}
                </div>
                <span className="v5-mono" style={{ fontSize: 11, color: 'var(--v5-txt)' }}>
                  {l.available_slots} / {l.total_slots}
                </span>
                <Badge variant={statusVariant(l.status)} dot>{STATUS_LABEL[l.status] ?? l.status}</Badge>
                <select
                  data-testid="standorte-status"
                  disabled={statBusy}
                  value={l.status}
                  onChange={(e) => updateStatus.mutate({ id: l.id, status: e.target.value as LotStatus })}
                  aria-label={`Status für ${l.name}`}
                  style={{
                    padding: '5px 8px', borderRadius: 7, background: 'var(--v5-sur2)',
                    border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 11,
                    outline: 'none', fontFamily: 'inherit',
                  }}
                >
                  <option value="open">Offen</option>
                  <option value="closed">Geschlossen</option>
                  <option value="full">Voll</option>
                  <option value="maintenance">Wartung</option>
                </select>
                <button
                  type="button" disabled={delBusy}
                  aria-label={`Standort ${l.name} löschen`}
                  onClick={() => deleteLot.mutate(l.id)}
                  style={{
                    padding: '4px 10px', borderRadius: 7,
                    background: 'color-mix(in oklch, var(--v5-err) 8%, transparent)',
                    border: 'none', fontSize: 10, fontWeight: 500, color: 'var(--v5-err)',
                    cursor: delBusy ? 'default' : 'pointer', opacity: delBusy ? 0.5 : 1,
                  }}
                >{delBusy ? '…' : 'Löschen'}</button>
              </div>
            );
          })}
        </Card>
      )}
    </div>
  );
}
