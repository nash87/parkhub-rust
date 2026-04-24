import { useEffect, useState, type CSSProperties } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Card, SectionLabel, Toggle, V5NamedIcon } from '../primitives';
import { useV5Toast } from '../Toast';
import { useV5Theme, V5_MODES, V5_MODE_LABELS, type V5Mode } from '../ThemeProvider';
import { api, type User, type NotificationPreferences } from '../../api/client';
import type { ScreenId } from '../nav';

const LANGUAGES: { code: string; label: string }[] = [
  { code: 'de', label: 'Deutsch' },
  { code: 'en', label: 'English' },
  { code: 'fr', label: 'Français' },
];

const LANG_STORAGE_KEY = 'i18nextLng';

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

export function EinstellungenV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();
  const { mode, setMode } = useV5Theme();

  const { data: user, isLoading, isError } = useQuery({
    queryKey: ['einstellungen-me'],
    queryFn: async () => {
      const res = await api.me();
      if (!res.success) throw new Error(res.error?.message ?? 'Einstellungen konnten nicht geladen werden');
      return res.data;
    },
    staleTime: 30_000,
  });

  const { data: prefs } = useQuery({
    queryKey: ['einstellungen-prefs'],
    queryFn: async () => {
      const res = await api.getNotificationPreferences();
      if (!res.success) throw new Error(res.error?.message ?? 'Benachrichtigungen konnten nicht geladen werden');
      return res.data;
    },
    staleTime: 30_000,
  });

  const [lang, setLang] = useState<string>(() => {
    if (typeof window === 'undefined') return 'de';
    return window.localStorage.getItem(LANG_STORAGE_KEY) ?? 'de';
  });
  const [department, setDepartment] = useState('');
  const [localPrefs, setLocalPrefs] = useState<NotificationPreferences | null>(null);

  useEffect(() => {
    if (user) setDepartment(user.department ?? '');
  }, [user]);

  useEffect(() => {
    if (prefs) setLocalPrefs(prefs);
  }, [prefs]);

  const saveMutation = useMutation({
    mutationFn: async (payload: Partial<User>) => {
      const res = await api.updateMe(payload);
      if (!res.success) throw new Error(res.error?.message ?? 'Speichern fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['einstellungen-me'] });
      toast('Einstellungen gespeichert', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Speichern fehlgeschlagen', 'error'),
  });

  const prefsMutation = useMutation({
    mutationFn: async (payload: NotificationPreferences) => {
      const res = await api.updateNotificationPreferences(payload);
      if (!res.success) throw new Error(res.error?.message ?? 'Speichern fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['einstellungen-prefs'] });
      toast('Benachrichtigungen gespeichert', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Speichern fehlgeschlagen', 'error'),
  });

  function handleLangChange(code: string) {
    setLang(code);
    if (typeof window !== 'undefined') window.localStorage.setItem(LANG_STORAGE_KEY, code);
    toast('Sprache aktualisiert', 'success');
  }

  function togglePref(key: keyof NotificationPreferences) {
    if (!localPrefs) return;
    const next = { ...localPrefs, [key]: !localPrefs[key] } as NotificationPreferences;
    setLocalPrefs(next);
    prefsMutation.mutate(next);
  }

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
        {[160, 220, 160, 160].map((h, i) => (
          <div key={i} style={{ height: h, borderRadius: 14, background: 'var(--v5-sur2)', animation: 'ph-v5-pulse 1.6s ease infinite', animationDelay: `${i * 0.1}s` }} />
        ))}
      </div>
    );
  }

  if (isError || !user) {
    return (
      <div style={{ padding: 16, flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <Card className="v5-ani" style={{ padding: 28, textAlign: 'center', maxWidth: 360 }}>
          <V5NamedIcon name="x" size={26} color="var(--v5-err)" />
          <div style={{ marginTop: 10, fontWeight: 600, color: 'var(--v5-txt)' }}>Fehler beim Laden</div>
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>Einstellungen konnten nicht geladen werden.</div>
        </Card>
      </div>
    );
  }

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Einstellungen</div>

      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 12, animationDelay: '0.06s' }}>
        <SectionLabel>Allgemein</SectionLabel>
        <label style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
          <span style={{ fontSize: 11, color: 'var(--v5-mut)' }}>Abteilung</span>
          <input
            data-testid="einst-department"
            type="text"
            value={department}
            onChange={(e) => setDepartment(e.target.value)}
            style={inputStyle}
          />
        </label>
        <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
          <button
            type="button"
            disabled={department === (user.department ?? '') || saveMutation.isPending}
            onClick={() => saveMutation.mutate({ department: department.trim() })}
            data-testid="einst-save"
            style={{
              padding: '8px 16px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)',
              border: 'none', fontSize: 12, fontWeight: 600,
              cursor: department !== (user.department ?? '') && !saveMutation.isPending ? 'pointer' : 'not-allowed',
              opacity: department !== (user.department ?? '') && !saveMutation.isPending ? 1 : 0.5,
            }}
          >
            {saveMutation.isPending ? 'Speichert …' : 'Speichern'}
          </button>
        </div>
      </Card>

      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 12, animationDelay: '0.12s' }}>
        <SectionLabel>Sprache</SectionLabel>
        <div role="group" aria-label="Sprache" style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          {LANGUAGES.map((l) => {
            const active = lang === l.code;
            return (
              <button
                key={l.code} type="button" aria-pressed={active} data-testid="einst-lang"
                onClick={() => handleLangChange(l.code)}
                style={{
                  padding: '6px 14px', borderRadius: 999, fontSize: 11, fontWeight: 500, cursor: 'pointer',
                  border: `1.5px solid ${active ? 'var(--v5-acc)' : 'var(--v5-bor)'}`,
                  background: active ? 'var(--v5-acc-muted)' : 'transparent',
                  color: active ? 'var(--v5-acc)' : 'var(--v5-mut)',
                }}
              >{l.label}</button>
            );
          })}
        </div>
      </Card>

      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 12, animationDelay: '0.18s' }}>
        <SectionLabel>Darstellung</SectionLabel>
        <div role="group" aria-label="Darstellungsmodus" style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          {V5_MODES.map((m: V5Mode) => {
            const active = mode === m;
            return (
              <button
                key={m} type="button" aria-pressed={active} data-testid="einst-theme"
                onClick={() => setMode(m)}
                style={{
                  padding: '8px 14px', borderRadius: 10, fontSize: 11, fontWeight: 500, cursor: 'pointer',
                  border: `1.5px solid ${active ? 'var(--v5-acc)' : 'var(--v5-bor)'}`,
                  background: active ? 'var(--v5-acc-muted)' : 'transparent',
                  color: active ? 'var(--v5-acc)' : 'var(--v5-mut)',
                }}
              >{V5_MODE_LABELS[m]}</button>
            );
          })}
        </div>
      </Card>

      {localPrefs && (
        <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 10, animationDelay: '0.24s' }}>
          <SectionLabel>Benachrichtigungen</SectionLabel>
          {([
            ['email_booking_confirm', 'E-Mail: Buchungsbestätigung'],
            ['email_booking_reminder', 'E-Mail: Erinnerung'],
            ['email_swap_request', 'E-Mail: Tauschanfragen'],
            ['push_enabled', 'Push aktiviert'],
          ] as Array<[keyof NotificationPreferences, string]>).map(([key, label]) => (
            <div key={key} style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <span style={{ fontSize: 12, color: 'var(--v5-txt)' }}>{label}</span>
              <Toggle
                checked={Boolean(localPrefs[key])}
                onChange={() => togglePref(key)}
                ariaLabel={label}
              />
            </div>
          ))}
        </Card>
      )}
    </div>
  );
}
