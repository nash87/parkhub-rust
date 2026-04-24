import { useEffect, useMemo, useState, type CSSProperties } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Card, SectionLabel, V5NamedIcon } from '../primitives';
import { useV5Toast } from '../Toast';
import { useV5Theme, V5_MODES, V5_MODE_LABELS, type V5Mode } from '../ThemeProvider';
import { api } from '../../api/client';
import type { ScreenId } from '../nav';

/**
 * Einstellungen — admin view of tenant / system settings.
 *
 * This screen lives under the `admin` nav section (see design-v5/nav.ts),
 * so it must surface system-wide configuration — `/api/v1/admin/settings`
 * — instead of personal profile fields. The personal Profil screen stays
 * wired to `api.me`/`api.updateMe`.
 *
 * Codex #376: the v5 draft accidentally wired this admin entry to
 * `api.me()`, which meant the tenant-scoped keys (company name, default
 * currency, booking window, cost-center policy) were unreachable for
 * admins in the new shell. Re-wiring to `adminGetSettings` +
 * `adminUpdateSettings` closes the regression.
 *
 * UI-side preferences that don't touch the backend — language chip and
 * the `useV5Theme` mode selector — remain local because they're a
 * per-browser thing, not tenant-level config.
 */

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

/**
 * Tenant settings keys surfaced as structured fields. The admin settings
 * endpoint returns an open `Record<string, string>` so callers can extend
 * without a schema migration; this array is the allowlist the v5 shell
 * renders + writes. Unknown keys returned by the backend are preserved on
 * save so we don't clobber values another tool set.
 */
const SETTING_FIELDS: Array<{
  key: string;
  label: string;
  hint: string;
  testId: string;
  type: 'text' | 'number' | 'boolean';
}> = [
  { key: 'company_name',         label: 'Firma',           hint: 'Wird in Exporten + Rechnungen verwendet.', testId: 'einst-company-name',   type: 'text' },
  { key: 'default_currency',     label: 'Währung',         hint: 'ISO-4217-Code, z. B. EUR, CHF, USD.',       testId: 'einst-currency',       type: 'text' },
  { key: 'booking_window_days',  label: 'Buchungsfenster', hint: 'Wie weit im Voraus gebucht werden darf.',   testId: 'einst-booking-window', type: 'number' },
  { key: 'cost_center_required', label: 'Kostenstelle Pflicht', hint: '`true` / `false` — erzwingt Kostenstelle pro Buchung.', testId: 'einst-cost-center-required', type: 'text' },
];

export function EinstellungenV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();
  const { mode, setMode } = useV5Theme();

  const { data: settings, isLoading, isError } = useQuery({
    queryKey: ['admin-settings'],
    queryFn: async () => {
      const res = await api.adminGetSettings();
      if (!res.success) throw new Error(res.error?.message ?? 'Einstellungen konnten nicht geladen werden');
      return res.data ?? {};
    },
    staleTime: 30_000,
  });

  const [lang, setLang] = useState<string>(() => {
    if (typeof window === 'undefined') return 'de';
    return window.localStorage.getItem(LANG_STORAGE_KEY) ?? 'de';
  });
  const [draft, setDraft] = useState<Record<string, string>>({});

  useEffect(() => {
    if (settings) setDraft(settings);
  }, [settings]);

  const saveMutation = useMutation({
    mutationFn: async (payload: Record<string, string>) => {
      const res = await api.adminUpdateSettings(payload);
      if (!res.success) throw new Error(res.error?.message ?? 'Speichern fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['admin-settings'] });
      toast('Einstellungen gespeichert', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Speichern fehlgeschlagen', 'error'),
  });

  const isDirty = useMemo(() => {
    if (!settings) return false;
    const keys = new Set([...Object.keys(settings), ...Object.keys(draft)]);
    for (const k of keys) {
      if ((settings[k] ?? '') !== (draft[k] ?? '')) return true;
    }
    return false;
  }, [settings, draft]);

  function handleLangChange(code: string) {
    setLang(code);
    if (typeof window !== 'undefined') window.localStorage.setItem(LANG_STORAGE_KEY, code);
    toast('Sprache aktualisiert', 'success');
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

  if (isError || !settings) {
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
        <SectionLabel>System</SectionLabel>
        {SETTING_FIELDS.map((f) => (
          <label key={f.key} style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            <span style={{ fontSize: 11, color: 'var(--v5-mut)' }}>{f.label}</span>
            <input
              data-testid={f.testId}
              type={f.type === 'number' ? 'number' : 'text'}
              value={draft[f.key] ?? ''}
              onChange={(e) => setDraft({ ...draft, [f.key]: e.target.value })}
              style={inputStyle}
            />
            <span style={{ fontSize: 10, color: 'var(--v5-mut)' }}>{f.hint}</span>
          </label>
        ))}
        <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
          <button
            type="button"
            disabled={!isDirty || saveMutation.isPending}
            onClick={() => saveMutation.mutate(draft)}
            data-testid="einst-save"
            style={{
              padding: '8px 16px', borderRadius: 9, background: 'var(--v5-acc)', color: 'var(--v5-accent-fg)',
              border: 'none', fontSize: 12, fontWeight: 600,
              cursor: isDirty && !saveMutation.isPending ? 'pointer' : 'not-allowed',
              opacity: isDirty && !saveMutation.isPending ? 1 : 0.5,
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
    </div>
  );
}
