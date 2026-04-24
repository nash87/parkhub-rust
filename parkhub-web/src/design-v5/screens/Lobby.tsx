import { useEffect, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Card, SectionLabel, Toggle, V5NamedIcon } from '../primitives';
import { useV5Toast } from '../Toast';
import { api, type LobbyConfig, type LobbyScreenKey } from '../../api/client';
import type { ScreenId } from '../nav';

const SCREEN_OPTIONS: { key: LobbyScreenKey; label: string; hint: string }[] = [
  { key: 'queue', label: 'Warteschlange', hint: 'Nächste Einfahrten' },
  { key: 'map', label: 'Karte', hint: 'Live-Auslastung' },
  { key: 'announcements', label: 'Ankündigungen', hint: 'Aktive Hinweise' },
  { key: 'welcome', label: 'Willkommen', hint: 'Logo + Begrüßung' },
];

export function LobbyV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();

  const { data: cfg, isLoading, isError } = useQuery({
    queryKey: ['lobby'],
    queryFn: async () => {
      const res = await api.getLobbyConfig();
      if (!res.success) throw new Error(res.error?.message ?? 'Lobby konnte nicht geladen werden');
      return res.data;
    },
    staleTime: 30_000,
  });

  const [local, setLocal] = useState<LobbyConfig | null>(null);
  useEffect(() => { if (cfg) setLocal(cfg); }, [cfg]);

  const save = useMutation({
    mutationFn: async (payload: Partial<LobbyConfig>) => {
      const res = await api.updateLobbyConfig(payload);
      if (!res.success) throw new Error(res.error?.message ?? 'Speichern fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['lobby'] });
      toast('Lobby aktualisiert', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Speichern fehlgeschlagen', 'error'),
  });

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
        {[200, 160, 140].map((h, i) => (
          <div key={i} style={{ height: h, borderRadius: 14, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.1}s` }} />
        ))}
      </div>
    );
  }

  if (isError || !local) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 28, textAlign: 'center', maxWidth: 360 }}>
          <V5NamedIcon name="x" size={26} color="var(--v5-err)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Fehler beim Laden</div>
        </Card>
      </div>
    );
  }

  function update<K extends keyof LobbyConfig>(key: K, value: LobbyConfig[K]) {
    if (!local) return;
    const next = { ...local, [key]: value };
    setLocal(next);
    save.mutate({ [key]: value } as Partial<LobbyConfig>);
  }

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Lobby-Display</div>

      <Card className="v5-ani" style={{ padding: 18, animationDelay: '0.06s' }}>
        <SectionLabel>Vorschau</SectionLabel>
        <div
          data-testid="lobby-preview"
          style={{
            aspectRatio: '16 / 9', width: '100%', borderRadius: 12,
            background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)',
            display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 8,
          }}
        >
          <V5NamedIcon name="monitor" size={36} color="var(--v5-acc)" />
          <div style={{ fontWeight: 600, fontSize: 14, color: 'var(--v5-txt)' }}>
            {SCREEN_OPTIONS.find((s) => s.key === local.active_screen)?.label}
          </div>
          <div style={{ fontSize: 11, color: 'var(--v5-mut)' }}>
            Wechselt alle {local.rotate_interval_seconds}s
          </div>
        </div>
      </Card>

      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 10, animationDelay: '0.12s' }}>
        <SectionLabel>Aktiver Screen</SectionLabel>
        <div role="radiogroup" aria-label="Aktiver Screen" style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(160px, 1fr))', gap: 8 }}>
          {SCREEN_OPTIONS.map((opt) => {
            const active = local.active_screen === opt.key;
            return (
              <button
                key={opt.key} type="button" role="radio" aria-checked={active}
                data-testid="lobby-screen"
                onClick={() => update('active_screen', opt.key)}
                style={{
                  padding: '12px 14px', borderRadius: 10, textAlign: 'left', cursor: 'pointer',
                  border: `1.5px solid ${active ? 'var(--v5-acc)' : 'var(--v5-bor)'}`,
                  background: active ? 'var(--v5-acc-muted)' : 'var(--v5-sur2)',
                  color: active ? 'var(--v5-acc)' : 'var(--v5-txt)',
                }}
              >
                <div style={{ fontSize: 12, fontWeight: 600 }}>{opt.label}</div>
                <div style={{ fontSize: 10, color: 'var(--v5-mut)', marginTop: 2 }}>{opt.hint}</div>
              </button>
            );
          })}
        </div>
      </Card>

      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 10, animationDelay: '0.18s' }}>
        <SectionLabel>Optionen</SectionLabel>
        <label style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          <span style={{ fontSize: 11, color: 'var(--v5-mut)' }}>Rotationsintervall (Sekunden)</span>
          <input
            type="number" min="5" max="600" step="5"
            data-testid="lobby-interval"
            value={local.rotate_interval_seconds}
            onChange={(e) => setLocal({ ...local, rotate_interval_seconds: Number(e.target.value) })}
            onBlur={() => update('rotate_interval_seconds', Math.max(5, Number(local.rotate_interval_seconds)))}
            style={{ padding: '8px 11px', borderRadius: 9, background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 12, outline: 'none', fontFamily: 'inherit', maxWidth: 160 }}
          />
        </label>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <span style={{ fontSize: 12, color: 'var(--v5-txt)' }}>Uhr anzeigen</span>
          <Toggle checked={local.show_clock} onChange={(v) => update('show_clock', v)} ariaLabel="Uhr anzeigen" />
        </div>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <span style={{ fontSize: 12, color: 'var(--v5-txt)' }}>Wetter anzeigen</span>
          <Toggle checked={local.show_weather} onChange={(v) => update('show_weather', v)} ariaLabel="Wetter anzeigen" />
        </div>
      </Card>
    </div>
  );
}
