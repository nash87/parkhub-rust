import { useEffect, useMemo, useState, type CSSProperties, type ReactNode } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Card, SectionLabel } from '../primitives';
import { useV5Toast } from '../Toast';
import { V5_MODE_LABELS } from '../ThemeProvider';
import { api } from '../../api/client';
import type { ScreenId } from '../nav';
import {
  V5_APPEARANCE_MODES,
  V5_DENSITIES,
  V5_FONT_SCALES,
  V5_FONT_VARIANTS,
  V5_SIDEBAR_VARIANTS,
  useV5Settings,
  type UserSettings,
  type V5AppearanceMode,
  type V5Density,
  type V5FontScale,
  type V5FontVariant,
  type V5SidebarVariant,
} from '../settings';
import { V5_FONT_LABELS } from '../fonts/fontVariants';

/**
 * Einstellungen — multi-tab settings hub for the v5 customization framework.
 *
 * Tabs:
 *   1. Erscheinungsbild  — theme mode, sidebar variant, density, font, font scale
 *   2. Funktionen         — 11 feature toggles, each with description + impact
 *   3. Barrierefreiheit   — reduced motion, high contrast, font scale
 *   4. Benachrichtigungen — push, email, sound
 *   5. Datenschutz        — analytics opt-in, crash reports
 *   6. System             — tenant-level admin settings (legacy v4 fields)
 *
 * Each toggle persists immediately on change (no save button) — user
 * settings are debounced to the backend via V5SettingsProvider.syncToServer.
 *
 * Branding: NO "AI" anywhere. Names use "Smart" / "Lokal" / "Musterbasiert".
 */

type TabId = 'appearance' | 'features' | 'a11y' | 'notifications' | 'privacy' | 'system';

const TABS: { id: TabId; label: string; testId: string }[] = [
  { id: 'appearance',    label: 'Erscheinungsbild',  testId: 'einst-tab-appearance' },
  { id: 'features',      label: 'Funktionen',        testId: 'einst-tab-features' },
  { id: 'a11y',          label: 'Barrierefreiheit',  testId: 'einst-tab-a11y' },
  { id: 'notifications', label: 'Benachrichtigungen', testId: 'einst-tab-notifications' },
  { id: 'privacy',       label: 'Datenschutz',        testId: 'einst-tab-privacy' },
  { id: 'system',        label: 'System',             testId: 'einst-tab-system' },
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

const SETTING_FIELDS: Array<{
  key: string;
  label: string;
  hint: string;
  testId: string;
  type: 'text' | 'number';
}> = [
  { key: 'company_name',         label: 'Firma',                hint: 'Wird in Exporten + Rechnungen verwendet.',         testId: 'einst-company-name',         type: 'text' },
  { key: 'default_currency',     label: 'Währung',              hint: 'ISO-4217-Code, z. B. EUR, CHF, USD.',              testId: 'einst-currency',             type: 'text' },
  { key: 'booking_window_days',  label: 'Buchungsfenster',      hint: 'Wie weit im Voraus gebucht werden darf.',          testId: 'einst-booking-window',       type: 'number' },
  { key: 'cost_center_required', label: 'Kostenstelle Pflicht', hint: '`true` / `false` — erzwingt Kostenstelle pro Buchung.', testId: 'einst-cost-center-required', type: 'text' },
];

const SIDEBAR_LABELS: Record<V5SidebarVariant, string> = {
  marble: 'Marble (Standard)',
  columns: 'Säulen (Live-Pass)',
  minimal: 'Minimal (nur Symbole)',
};

const DENSITY_LABELS: Record<V5Density, string> = {
  compact: 'Kompakt',
  comfortable: 'Komfortabel',
  spacious: 'Großzügig',
};

const FONT_SCALE_LABELS: Record<string, string> = {
  '0.875': 'Klein (87 %)',
  '1': 'Standard (100 %)',
  '1.125': 'Groß (112 %)',
  '1.25': 'Extra-Groß (125 %)',
};

const FEATURE_DESCRIPTIONS: Record<keyof UserSettings['features'], { label: string; hint: string; impact: string }> = {
  smartSuggestions: {
    label: 'Smarte Vorschläge',
    hint: 'Musterbasierte Buchungs-Chips auf dem Dashboard.',
    impact: 'Nutzt nur lokale Buchungs-Historie — keine Cloud-Anfrage.',
  },
  optimisticUI: {
    label: 'Optimistische UI',
    hint: 'Aktionen erscheinen sofort, bevor der Server bestätigt.',
    impact: 'Schneller wirkende Oberfläche; Rückgängig bei Fehlern.',
  },
  viewTransitions: {
    label: 'Übergänge',
    hint: 'Sanfte Bildschirmwechsel statt harter Sprünge.',
    impact: 'Deaktiviert bei reduzierten Bewegungen.',
  },
  voiceCommands: {
    label: 'Sprachbefehle',
    hint: 'Diktat über Web-Speech-API auf dem Dashboard.',
    impact: 'Verarbeitung lokal im Browser (keine Cloud).',
  },
  qrCheckin: {
    label: 'QR-Check-in',
    hint: 'QR-Code auf der Einchecken-Seite anzeigen.',
    impact: 'Aktiviert kontaktloses Einchecken.',
  },
  deepLinking: {
    label: 'Tiefe Links',
    hint: 'Bildschirmname als URL `/v5/<screen>` mitführen.',
    impact: 'Erleichtert das Teilen direkter Links.',
  },
  predictiveCard: {
    label: 'Musterkarte „Buchen"',
    hint: 'Wiederkehrende Buchungs-Muster vorschlagen.',
    impact: 'Erkennung läuft lokal; nichts wird hochgeladen.',
  },
  swAutoUpdate: {
    label: 'Service-Worker Auto-Update',
    hint: 'Neue Version automatisch übernehmen.',
    impact: 'Zeigt sonst eine manuelle Bestätigung.',
  },
  plateScan: {
    label: 'Kennzeichen-Scan',
    hint: 'Kamera-OCR auf der Fahrzeuge-Seite.',
    impact: 'Verarbeitung lokal im Gerät.',
  },
  semanticSearch: {
    label: 'Smarte Suche',
    hint: 'Bedeutungsbasierte Befehlssuche im Palette-Fenster.',
    impact: 'Lokales Embedding-Modell (~3 MB einmalig).',
  },
  fleetSSE: {
    label: 'Echtzeit-Flotte',
    hint: 'Server-sent events auf Flotten-Bildschirmen.',
    impact: 'Hält offene Verbindung; mehr Akku-Verbrauch.',
  },
};

const LANG_STORAGE_KEY = 'i18nextLng';
const LANGUAGES: { code: string; label: string }[] = [
  { code: 'de', label: 'Deutsch' },
  { code: 'en', label: 'English' },
  { code: 'fr', label: 'Français' },
];

export function EinstellungenV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();
  const { settings, updateSetting, updateSection, resetSettings, syncState } = useV5Settings();
  const [tab, setTab] = useState<TabId>('appearance');

  const { data: tenantSettings, isLoading, isError } = useQuery({
    queryKey: ['admin-settings'],
    queryFn: async () => {
      const res = await api.adminGetSettings();
      if (!res.success) throw new Error(res.error?.message ?? 'Einstellungen konnten nicht geladen werden');
      return res.data ?? {};
    },
    staleTime: 30_000,
  });

  const [lang, setLang] = useState<string>(() =>
    typeof window !== 'undefined' ? window.localStorage.getItem(LANG_STORAGE_KEY) ?? 'de' : 'de',
  );
  const [draft, setDraft] = useState<Record<string, string>>({});
  useEffect(() => {
    if (tenantSettings) setDraft(tenantSettings);
  }, [tenantSettings]);

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
    if (!tenantSettings) return false;
    const keys = new Set([...Object.keys(tenantSettings), ...Object.keys(draft)]);
    for (const k of keys) {
      if ((tenantSettings[k] ?? '') !== (draft[k] ?? '')) return true;
    }
    return false;
  }, [tenantSettings, draft]);

  function handleLangChange(code: string) {
    setLang(code);
    if (typeof window !== 'undefined') window.localStorage.setItem(LANG_STORAGE_KEY, code);
    toast('Sprache aktualisiert', 'success');
  }

  const syncBadge = useMemo(() => {
    switch (syncState) {
      case 'saving':
        return { label: 'Speichert …', tone: 'var(--v5-info)' };
      case 'saved':
        return { label: 'Gespeichert', tone: 'var(--v5-ok)' };
      case 'error':
        return { label: 'Speichern fehlgeschlagen', tone: 'var(--v5-err)' };
      default:
        return null;
    }
  }, [syncState]);

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
        <div style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Einstellungen</div>
        {syncBadge && (
          <span
            data-testid="einst-sync-badge"
            style={{
              fontSize: 10,
              fontWeight: 600,
              padding: '3px 8px',
              borderRadius: 999,
              background: `color-mix(in oklch, ${syncBadge.tone} 18%, transparent)`,
              color: syncBadge.tone,
              textTransform: 'uppercase',
              letterSpacing: '0.06em',
            }}
          >
            {syncBadge.label}
          </span>
        )}
      </div>

      <nav
        role="tablist"
        aria-label="Einstellungen"
        style={{
          display: 'flex',
          gap: 4,
          flexWrap: 'wrap',
          borderBottom: '1px solid var(--v5-bor)',
          paddingBottom: 4,
        }}
      >
        {TABS.map((t) => {
          const isActive = tab === t.id;
          return (
            <button
              key={t.id}
              type="button"
              role="tab"
              aria-selected={isActive}
              aria-controls={`einst-panel-${t.id}`}
              data-testid={t.testId}
              onClick={() => setTab(t.id)}
              style={{
                padding: '8px 14px',
                fontSize: 12,
                fontWeight: isActive ? 600 : 500,
                cursor: 'pointer',
                background: isActive ? 'var(--v5-acc-muted)' : 'transparent',
                color: isActive ? 'var(--v5-acc)' : 'var(--v5-mut)',
                border: 0,
                borderRadius: 9,
                borderBottom: `2px solid ${isActive ? 'var(--v5-acc)' : 'transparent'}`,
              }}
            >
              {t.label}
            </button>
          );
        })}
      </nav>

      <div role="tabpanel" id={`einst-panel-${tab}`} style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
        {tab === 'appearance' && (
          <AppearancePanel
            settings={settings}
            onModeChange={(m) => updateSetting('appearance', 'mode', m)}
            onSidebarChange={(s) => updateSetting('appearance', 'sidebar', s)}
            onDensityChange={(d) => updateSetting('appearance', 'density', d)}
            onFontChange={(f) => updateSetting('appearance', 'font', f)}
            lang={lang}
            onLangChange={handleLangChange}
          />
        )}

        {tab === 'features' && (
          <FeaturesPanel
            settings={settings}
            onChange={(key, value) => updateSetting('features', key, value)}
          />
        )}

        {tab === 'a11y' && (
          <A11yPanel
            settings={settings}
            onReducedMotion={(v) => updateSetting('appearance', 'reducedMotion', v)}
            onHighContrast={(v) => updateSetting('appearance', 'highContrast', v)}
            onFontScale={(v) => updateSetting('appearance', 'fontScale', v)}
          />
        )}

        {tab === 'notifications' && (
          <NotificationsPanel
            settings={settings}
            onChange={(patch) => updateSection('notifications', patch)}
          />
        )}

        {tab === 'privacy' && (
          <PrivacyPanel
            settings={settings}
            onChange={(patch) => updateSection('privacy', patch)}
            onReset={() => {
              resetSettings();
              toast('Einstellungen zurückgesetzt', 'success');
            }}
          />
        )}

        {tab === 'system' && (
          <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 12 }}>
            <SectionLabel>System</SectionLabel>
            {isLoading ? (
              <div style={{ height: 120, borderRadius: 12, background: 'var(--v5-sur2)' }} />
            ) : isError ? (
              <div style={{ fontSize: 12, color: 'var(--v5-err)' }}>Fehler beim Laden der System-Einstellungen.</div>
            ) : (
              <>
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
                      padding: '8px 16px',
                      borderRadius: 9,
                      background: 'var(--v5-acc)',
                      color: 'var(--v5-accent-fg)',
                      border: 'none',
                      fontSize: 12,
                      fontWeight: 600,
                      cursor: isDirty && !saveMutation.isPending ? 'pointer' : 'not-allowed',
                      opacity: isDirty && !saveMutation.isPending ? 1 : 0.5,
                    }}
                  >
                    {saveMutation.isPending ? 'Speichert …' : 'Speichern'}
                  </button>
                </div>
              </>
            )}
          </Card>
        )}
      </div>
    </div>
  );
}

function AppearancePanel({
  settings,
  onModeChange,
  onSidebarChange,
  onDensityChange,
  onFontChange,
  lang,
  onLangChange,
}: {
  settings: UserSettings;
  onModeChange: (m: V5AppearanceMode) => void;
  onSidebarChange: (s: V5SidebarVariant) => void;
  onDensityChange: (d: V5Density) => void;
  onFontChange: (f: V5FontVariant) => void;
  lang: string;
  onLangChange: (code: string) => void;
}) {
  return (
    <>
      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 12 }}>
        <SectionLabel>Modus</SectionLabel>
        <ChipGroup
          value={settings.appearance.mode}
          options={V5_APPEARANCE_MODES.map((m) => ({ value: m, label: V5_MODE_LABELS[m] }))}
          onChange={onModeChange}
          testId="einst-theme"
        />
      </Card>

      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 12 }}>
        <SectionLabel>Seitenleiste</SectionLabel>
        <ChipGroup
          value={settings.appearance.sidebar}
          options={V5_SIDEBAR_VARIANTS.map((s) => ({ value: s, label: SIDEBAR_LABELS[s] }))}
          onChange={onSidebarChange}
          testId="einst-sidebar"
        />
      </Card>

      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 12 }}>
        <SectionLabel>Dichte</SectionLabel>
        <ChipGroup
          value={settings.appearance.density}
          options={V5_DENSITIES.map((d) => ({ value: d, label: DENSITY_LABELS[d] }))}
          onChange={onDensityChange}
          testId="einst-density"
        />
      </Card>

      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 12 }}>
        <SectionLabel>Schriftart</SectionLabel>
        <ChipGroup
          value={settings.appearance.font}
          options={V5_FONT_VARIANTS.map((f) => ({ value: f, label: V5_FONT_LABELS[f] }))}
          onChange={onFontChange}
          testId="einst-font"
        />
        <div style={{ fontSize: 11, color: 'var(--v5-mut)' }}>
          Plex und Atkinson werden bei Auswahl nachgeladen (~30 KB).
        </div>
      </Card>

      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 12 }}>
        <SectionLabel>Sprache</SectionLabel>
        <ChipGroup
          value={lang}
          options={LANGUAGES.map((l) => ({ value: l.code, label: l.label }))}
          onChange={onLangChange}
          testId="einst-lang"
        />
      </Card>
    </>
  );
}

function FeaturesPanel({
  settings,
  onChange,
}: {
  settings: UserSettings;
  onChange: <K extends keyof UserSettings['features']>(key: K, value: UserSettings['features'][K]) => void;
}) {
  return (
    <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 14 }}>
      <SectionLabel>Funktionen</SectionLabel>
      <div style={{ fontSize: 11, color: 'var(--v5-mut)', marginBottom: 4 }}>
        Alle Verarbeitung läuft, wo möglich, lokal im Browser. Schalter sofort wirksam.
      </div>
      {(Object.keys(FEATURE_DESCRIPTIONS) as Array<keyof UserSettings['features']>).map((key) => {
        const meta = FEATURE_DESCRIPTIONS[key];
        return (
          <ToggleRow
            key={key}
            testId={`einst-feature-${key}`}
            label={meta.label}
            hint={meta.hint}
            impact={meta.impact}
            checked={settings.features[key]}
            onChange={(v) => onChange(key, v)}
          />
        );
      })}
    </Card>
  );
}

function A11yPanel({
  settings,
  onReducedMotion,
  onHighContrast,
  onFontScale,
}: {
  settings: UserSettings;
  onReducedMotion: (v: boolean) => void;
  onHighContrast: (v: boolean) => void;
  onFontScale: (v: V5FontScale) => void;
}) {
  return (
    <>
      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 14 }}>
        <SectionLabel>Bewegung & Kontrast</SectionLabel>
        <ToggleRow
          testId="einst-reduced-motion"
          label="Bewegungen reduzieren"
          hint="Deaktiviert Übergänge und Animationen."
          impact="Empfohlen bei Reisekrankheit oder vestibulären Empfindlichkeiten."
          checked={settings.appearance.reducedMotion}
          onChange={onReducedMotion}
        />
        <ToggleRow
          testId="einst-high-contrast"
          label="Hoher Kontrast"
          hint="Verstärkt Rahmen, Akzentfarben und Trennlinien."
          impact="Erhöht die Lesbarkeit auf hellen Außenbildschirmen."
          checked={settings.appearance.highContrast}
          onChange={onHighContrast}
        />
      </Card>

      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 12 }}>
        <SectionLabel>Schriftgröße</SectionLabel>
        <ChipGroup
          value={String(settings.appearance.fontScale) as '0.875' | '1' | '1.125' | '1.25'}
          options={V5_FONT_SCALES.map((s) => ({
            value: String(s) as '0.875' | '1' | '1.125' | '1.25',
            label: FONT_SCALE_LABELS[String(s)] ?? String(s),
          }))}
          onChange={(v) => onFontScale(Number(v) as V5FontScale)}
          testId="einst-fontscale"
        />
      </Card>
    </>
  );
}

function NotificationsPanel({
  settings,
  onChange,
}: {
  settings: UserSettings;
  onChange: (patch: Partial<UserSettings['notifications']>) => void;
}) {
  return (
    <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 14 }}>
      <SectionLabel>Benachrichtigungen</SectionLabel>
      <ToggleRow
        testId="einst-notify-push"
        label="Push-Benachrichtigungen"
        hint="Browser-Push für Buchungs-Updates und Erinnerungen."
        impact="Erfordert einmalige Berechtigung im Browser."
        checked={settings.notifications.pushEnabled}
        onChange={(v) => onChange({ pushEnabled: v })}
      />
      <ToggleRow
        testId="einst-notify-email"
        label="E-Mail"
        hint="Tägliche Zusammenfassung und wichtige Ereignisse."
        impact="Adresse aus Profil; jederzeit kündbar."
        checked={settings.notifications.emailEnabled}
        onChange={(v) => onChange({ emailEnabled: v })}
      />
      <ToggleRow
        testId="einst-notify-sound"
        label="Klang"
        hint="Subtiler Hinweiston bei eingehenden Updates."
        impact="Respektiert OS-Stummschaltung."
        checked={settings.notifications.soundEnabled}
        onChange={(v) => onChange({ soundEnabled: v })}
      />
    </Card>
  );
}

function PrivacyPanel({
  settings,
  onChange,
  onReset,
}: {
  settings: UserSettings;
  onChange: (patch: Partial<UserSettings['privacy']>) => void;
  onReset: () => void;
}) {
  return (
    <>
      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 14 }}>
        <SectionLabel>Datenschutz</SectionLabel>
        <ToggleRow
          testId="einst-privacy-analytics"
          label="Anonyme Nutzungs-Statistiken"
          hint="Hilft uns, häufige Wege schneller zu machen."
          impact="Keine personen­bezogenen Daten, keine externen Anbieter."
          checked={settings.privacy.analyticsOptIn}
          onChange={(v) => onChange({ analyticsOptIn: v })}
        />
        <ToggleRow
          testId="einst-privacy-crash"
          label="Absturz-Berichte"
          hint="Übermittelt Stack-Traces bei Fehlern."
          impact="Ohne Persona-Daten; ausschließlich technische Details."
          checked={settings.privacy.crashReportsOptIn}
          onChange={(v) => onChange({ crashReportsOptIn: v })}
        />
      </Card>

      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 8 }}>
        <SectionLabel>Zurücksetzen</SectionLabel>
        <div style={{ fontSize: 12, color: 'var(--v5-mut)' }}>
          Alle persönlichen Einstellungen auf Werkseinstellungen zurücksetzen. Tenant-Einstellungen
          bleiben unverändert.
        </div>
        <button
          type="button"
          data-testid="einst-reset"
          onClick={onReset}
          style={{
            alignSelf: 'flex-start',
            padding: '8px 14px',
            borderRadius: 9,
            background: 'transparent',
            color: 'var(--v5-err)',
            border: '1px solid var(--v5-err)',
            fontSize: 12,
            fontWeight: 600,
            cursor: 'pointer',
            marginTop: 6,
          }}
        >
          Zurücksetzen
        </button>
      </Card>
    </>
  );
}

function ChipGroup<T extends string>({
  value,
  options,
  onChange,
  testId,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
  testId: string;
}) {
  return (
    <div role="group" style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
      {options.map((opt) => {
        const active = opt.value === value;
        return (
          <button
            key={opt.value}
            type="button"
            aria-pressed={active}
            data-testid={testId}
            data-value={opt.value}
            onClick={() => onChange(opt.value)}
            style={{
              padding: '8px 14px',
              borderRadius: 10,
              fontSize: 11,
              fontWeight: 500,
              cursor: 'pointer',
              border: `1.5px solid ${active ? 'var(--v5-acc)' : 'var(--v5-bor)'}`,
              background: active ? 'var(--v5-acc-muted)' : 'transparent',
              color: active ? 'var(--v5-acc)' : 'var(--v5-mut)',
            }}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}

function ToggleRow({
  label,
  hint,
  impact,
  checked,
  onChange,
  testId,
}: {
  label: string;
  hint: ReactNode;
  impact: ReactNode;
  checked: boolean;
  onChange: (v: boolean) => void;
  testId: string;
}) {
  return (
    <label
      style={{
        display: 'grid',
        gridTemplateColumns: '1fr auto',
        alignItems: 'center',
        gap: 12,
        padding: '8px 0',
        borderBottom: '1px solid var(--v5-bor)',
      }}
    >
      <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        <span style={{ fontSize: 12.5, fontWeight: 600, color: 'var(--v5-txt)' }}>{label}</span>
        <span style={{ fontSize: 11, color: 'var(--v5-mut)' }}>{hint}</span>
        <span style={{ fontSize: 10.5, color: 'var(--v5-mut)', opacity: 0.85 }}>{impact}</span>
      </div>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={label}
        data-testid={testId}
        onClick={() => onChange(!checked)}
        style={{
          width: 38,
          height: 22,
          borderRadius: 999,
          padding: 2,
          background: checked ? 'var(--v5-acc)' : 'var(--v5-sur2)',
          border: `1px solid ${checked ? 'var(--v5-acc)' : 'var(--v5-bor)'}`,
          display: 'flex',
          alignItems: 'center',
          justifyContent: checked ? 'flex-end' : 'flex-start',
          cursor: 'pointer',
          transition: 'background 0.18s, border-color 0.18s',
        }}
      >
        <span
          aria-hidden="true"
          style={{
            display: 'block',
            width: 16,
            height: 16,
            borderRadius: '50%',
            background: checked ? 'var(--v5-accent-fg)' : 'var(--v5-mut)',
            boxShadow: '0 1px 2px rgba(0,0,0,0.18)',
          }}
        />
      </button>
    </label>
  );
}
