import { useState } from 'react';
import NumberFlow from '@number-flow/react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge, Card, SectionLabel, V5NamedIcon } from '../primitives';
import { useV5Toast } from '../Toast';
import { api, type CreatedApiKey } from '../../api/client';
import type { ScreenId } from '../nav';

function formatWhen(iso: string | null): string {
  if (!iso) return 'Nie';
  return new Date(iso).toLocaleDateString('de-DE', { day: '2-digit', month: '2-digit', year: 'numeric' });
}

export function ApikeysV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();
  const [newLabel, setNewLabel] = useState('');
  const [revealed, setRevealed] = useState<CreatedApiKey | null>(null);

  const { data: keys = [], isLoading, isError } = useQuery({
    queryKey: ['apikeys'],
    queryFn: async () => {
      const res = await api.getApiKeys();
      if (!res.success) throw new Error(res.error?.message ?? 'API-Schlüssel konnten nicht geladen werden');
      return res.data ?? [];
    },
    staleTime: 30_000,
  });

  const create = useMutation({
    mutationFn: async (label: string) => {
      const res = await api.createApiKey(label);
      if (!res.success) throw new Error(res.error?.message ?? 'Erstellen fehlgeschlagen');
      return res.data;
    },
    onSuccess: (data) => {
      qc.invalidateQueries({ queryKey: ['apikeys'] });
      if (data) setRevealed(data);
      toast('API-Schlüssel erstellt', 'success');
      setNewLabel('');
    },
    onError: (err: Error) => toast(err.message || 'Erstellen fehlgeschlagen', 'error'),
  });

  const rotate = useMutation({
    mutationFn: async (id: string) => {
      const res = await api.rotateApiKey(id);
      if (!res.success) throw new Error(res.error?.message ?? 'Rotation fehlgeschlagen');
      return res.data;
    },
    onSuccess: (data) => {
      qc.invalidateQueries({ queryKey: ['apikeys'] });
      if (data) setRevealed(data);
      toast('Schlüssel rotiert', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Rotation fehlgeschlagen', 'error'),
  });

  const revoke = useMutation({
    mutationFn: async (id: string) => {
      const res = await api.revokeApiKey(id);
      if (!res.success) throw new Error(res.error?.message ?? 'Widerruf fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['apikeys'] });
      toast('Schlüssel widerrufen', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Widerruf fehlgeschlagen', 'error'),
  });

  async function copyToken() {
    if (!revealed?.token || typeof navigator === 'undefined' || !navigator.clipboard) return;
    try {
      await navigator.clipboard.writeText(revealed.token);
      toast('In Zwischenablage kopiert', 'success');
    } catch {
      toast('Kopieren fehlgeschlagen', 'error');
    }
  }

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
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
        <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>API-Schlüssel</span>
        <Badge variant="gray"><NumberFlow value={keys.length} /></Badge>
      </div>

      {revealed && (
        <Card className="v5-ani" data-testid="apikeys-revealed" style={{ padding: 16, borderLeft: '3px solid var(--v5-warn)' }}>
          <SectionLabel>Neuer Schlüssel – einmalig sichtbar</SectionLabel>
          <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginBottom: 8 }}>
            Kopieren Sie den Schlüssel jetzt – nach dem Schließen wird er maskiert.
          </div>
          <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            <code
              className="v5-mono"
              style={{
                flex: 1, padding: '8px 11px', borderRadius: 9,
                background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)',
                color: 'var(--v5-txt)', fontSize: 11, wordBreak: 'break-all',
              }}
            >{revealed.token}</code>
            <button
              type="button" onClick={copyToken} data-testid="apikeys-copy"
              style={{ padding: '7px 12px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)', border: 'none', fontSize: 11, fontWeight: 600, cursor: 'pointer' }}
            >Kopieren</button>
            <button
              type="button" onClick={() => setRevealed(null)} data-testid="apikeys-dismiss"
              style={{ padding: '7px 12px', borderRadius: 9, background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 11, fontWeight: 500, cursor: 'pointer' }}
            >Schließen</button>
          </div>
        </Card>
      )}

      <Card className="v5-ani" style={{ padding: 16, display: 'flex', flexDirection: 'column', gap: 10 }}>
        <SectionLabel>Neuen Schlüssel erstellen</SectionLabel>
        <div style={{ display: 'flex', gap: 8 }}>
          <input
            data-testid="apikeys-label"
            placeholder="Label (z. B. CI/CD)"
            value={newLabel}
            onChange={(e) => setNewLabel(e.target.value)}
            style={{ flex: 1, padding: '8px 11px', borderRadius: 9, background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 12, outline: 'none', fontFamily: 'inherit' }}
          />
          <button
            type="button"
            disabled={newLabel.trim().length < 2 || create.isPending}
            onClick={() => create.mutate(newLabel.trim())}
            data-testid="apikeys-create"
            style={{
              padding: '8px 16px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)',
              border: 'none', fontSize: 12, fontWeight: 600,
              cursor: newLabel.trim().length >= 2 && !create.isPending ? 'pointer' : 'not-allowed',
              opacity: newLabel.trim().length >= 2 && !create.isPending ? 1 : 0.5,
            }}
          >{create.isPending ? 'Erstellt …' : 'Erstellen'}</button>
        </div>
      </Card>

      {keys.length === 0 ? (
        <Card className="v5-ani" style={{ padding: 36, textAlign: 'center' }}>
          <V5NamedIcon name="key" size={20} color="var(--v5-mut)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Keine Schlüssel</div>
        </Card>
      ) : (
        <Card className="v5-ani" style={{ overflow: 'hidden' }}>
          {keys.map((k, i) => {
            const rotBusy = rotate.isPending && rotate.variables === k.id;
            const revBusy = revoke.isPending && revoke.variables === k.id;
            return (
              <div key={k.id} data-testid="apikeys-row" style={{
                display: 'grid', gridTemplateColumns: '1fr 1fr 120px 180px',
                padding: '12px 16px', alignItems: 'center', gap: 8,
                borderBottom: i < keys.length - 1 ? '1px solid var(--v5-bor)' : 'none',
              }}>
                <span style={{ fontSize: 12, fontWeight: 500, color: 'var(--v5-txt)' }}>{k.label}</span>
                <code className="v5-mono" style={{ fontSize: 11, color: 'var(--v5-mut)' }}>{k.masked_key}</code>
                <span style={{ fontSize: 10, color: 'var(--v5-mut)' }}>
                  Zuletzt: {formatWhen(k.last_used_at)}
                </span>
                <div style={{ display: 'flex', gap: 6, justifyContent: 'flex-end' }}>
                  <button
                    type="button" disabled={rotBusy}
                    onClick={() => rotate.mutate(k.id)}
                    aria-label={`Schlüssel ${k.label} rotieren`}
                    style={{ padding: '4px 10px', borderRadius: 7, background: 'var(--v5-acc-muted)', border: 'none', fontSize: 10, fontWeight: 500, color: 'var(--v5-acc)', cursor: rotBusy ? 'default' : 'pointer', opacity: rotBusy ? 0.5 : 1 }}
                  >{rotBusy ? '…' : 'Rotieren'}</button>
                  <button
                    type="button" disabled={revBusy}
                    onClick={() => revoke.mutate(k.id)}
                    aria-label={`Schlüssel ${k.label} widerrufen`}
                    style={{ padding: '4px 10px', borderRadius: 7, background: 'color-mix(in oklch, var(--v5-err) 8%, transparent)', border: 'none', fontSize: 10, fontWeight: 500, color: 'var(--v5-err)', cursor: revBusy ? 'default' : 'pointer', opacity: revBusy ? 0.5 : 1 }}
                  >{revBusy ? '…' : 'Widerrufen'}</button>
                </div>
              </div>
            );
          })}
        </Card>
      )}
    </div>
  );
}
