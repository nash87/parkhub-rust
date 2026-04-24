import { useEffect, useState, type CSSProperties, type ReactNode } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Badge, Card, SectionLabel, V5NamedIcon } from '../primitives';
import { HelpTip } from '../primitives/HelpTip';
import { useV5Toast } from '../Toast';
import { useV5Theme, V5_MODES, V5_MODE_LABELS, type V5Mode } from '../ThemeProvider';
import { api, type User } from '../../api/client';
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

function Field({ label, children, hint }: { label: string; children: ReactNode; hint?: ReactNode }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
      <label style={{ fontSize: 11, fontWeight: 500, color: 'var(--v5-mut)', display: 'inline-flex', alignItems: 'center' }}>
        {label}
        {hint}
      </label>
      {children}
    </div>
  );
}

export function ProfilV5({ navigate: _navigate }: { navigate: (id: ScreenId) => void }) {
  const toast = useV5Toast();
  const qc = useQueryClient();
  const { mode, setMode } = useV5Theme();

  const { data: user, isLoading, isError } = useQuery({
    queryKey: ['profil'],
    queryFn: async () => {
      const res = await api.me();
      if (!res.success) throw new Error(res.error?.message ?? 'Profil konnte nicht geladen werden');
      return res.data;
    },
    staleTime: 30_000,
    refetchOnWindowFocus: true,
  });

  const [name, setName] = useState('');
  const [email, setEmail] = useState('');
  const [lang, setLang] = useState<string>(() => {
    if (typeof window === 'undefined') return 'de';
    return window.localStorage.getItem(LANG_STORAGE_KEY) ?? 'de';
  });
  const [pwOpen, setPwOpen] = useState(false);
  const [pwCurrent, setPwCurrent] = useState('');
  const [pwNew, setPwNew] = useState('');
  const [pwConfirm, setPwConfirm] = useState('');

  useEffect(() => {
    if (user) {
      setName(user.name ?? '');
      setEmail(user.email ?? '');
    }
  }, [user]);

  const saveMutation = useMutation({
    mutationFn: async (payload: Partial<User>) => {
      const res = await api.updateMe(payload);
      if (!res.success) throw new Error(res.error?.message ?? 'Speichern fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['profil'] });
      toast('Profil aktualisiert', 'success');
    },
    onError: (err: Error) => toast(err.message || 'Speichern fehlgeschlagen', 'error'),
  });

  const passwordMutation = useMutation({
    mutationFn: async (payload: { current: string; next: string; confirm: string }) => {
      const res = await api.changePassword(payload.current, payload.next, payload.confirm);
      if (!res.success) throw new Error(res.error?.message ?? 'Passwortänderung fehlgeschlagen');
      return res.data;
    },
    onSuccess: () => {
      toast('Passwort geändert', 'success');
      setPwCurrent('');
      setPwNew('');
      setPwConfirm('');
      setPwOpen(false);
    },
    onError: (err: Error) => toast(err.message || 'Passwortänderung fehlgeschlagen', 'error'),
  });

  function handleLangChange(code: string) {
    setLang(code);
    if (typeof window !== 'undefined') {
      window.localStorage.setItem(LANG_STORAGE_KEY, code);
    }
    toast('Sprache aktualisiert', 'success');
  }

  function canSaveProfile(): boolean {
    return !!user && (name.trim() !== (user.name ?? '') || email.trim() !== (user.email ?? ''));
  }

  function canSavePassword(): boolean {
    return pwCurrent.length > 0 && pwNew.length >= 12 && pwNew === pwConfirm;
  }

  if (isLoading) {
    return (
      <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
        {[220, 160, 160, 140].map((h, i) => (
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
          <div style={{ fontSize: 12, color: 'var(--v5-mut)', marginTop: 4 }}>Profil konnte nicht geladen werden.</div>
        </Card>
      </div>
    );
  }

  return (
    <div style={{ padding: 16, flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', gap: 12 }}>
      <div className="v5-ani" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <span style={{ fontWeight: 700, fontSize: 15, color: 'var(--v5-txt)' }}>Mein Profil</span>
        <Badge variant={user.role === 'admin' || user.role === 'superadmin' ? 'purple' : 'primary'}>
          {user.role}
        </Badge>
      </div>

      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 12, animationDelay: '0.06s' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <SectionLabel>Kontoinformation</SectionLabel>
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
          <Field label="Name">
            <input
              id="profil-name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              style={inputStyle}
            />
          </Field>
          <Field
            label="E-Mail"
            hint={
              <HelpTip label="Hinweis zur E-Mail">
                E-Mail-Bestätigung erforderlich
              </HelpTip>
            }
          >
            <input
              id="profil-email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              style={inputStyle}
            />
          </Field>
        </div>
        <div style={{ fontSize: 11, color: 'var(--v5-mut)' }}>
          Benutzername: <span className="v5-mono" style={{ color: 'var(--v5-txt)' }}>{user.username}</span>
          {user.department && (
            <>
              {' · '}Abteilung: <span style={{ color: 'var(--v5-txt)' }}>{user.department}</span>
            </>
          )}
        </div>
        <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
          <button
            type="button"
            disabled={!canSaveProfile() || saveMutation.isPending}
            onClick={() => saveMutation.mutate({ name: name.trim(), email: email.trim() })}
            data-testid="profil-save"
            style={{
              padding: '8px 16px',
              borderRadius: 9,
              background: 'var(--v5-acc)',
              color: 'var(--v5-accent-fg)',
              border: 'none',
              fontSize: 12,
              fontWeight: 600,
              cursor: canSaveProfile() && !saveMutation.isPending ? 'pointer' : 'not-allowed',
              opacity: canSaveProfile() && !saveMutation.isPending ? 1 : 0.5,
            }}
          >
            {saveMutation.isPending ? 'Speichert …' : 'Speichern'}
          </button>
        </div>
      </Card>

      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 12, animationDelay: '0.12s' }}>
        <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
          <div style={{ display: 'flex', alignItems: 'center' }}>
            <SectionLabel>Sicherheit</SectionLabel>
            <HelpTip label="Hinweis zum Passwort">
              Mindestens 12 Zeichen
            </HelpTip>
          </div>
          <button
            type="button"
            onClick={() => setPwOpen((o) => !o)}
            data-testid="profil-pw-toggle"
            style={{ padding: '6px 12px', borderRadius: 8, background: 'var(--v5-sur2)', border: '1px solid var(--v5-bor)', color: 'var(--v5-txt)', fontSize: 11, fontWeight: 500, cursor: 'pointer' }}
          >
            {pwOpen ? 'Abbrechen' : 'Passwort ändern'}
          </button>
        </div>
        {pwOpen && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            <Field label="Aktuelles Passwort">
              <input
                type="password"
                value={pwCurrent}
                onChange={(e) => setPwCurrent(e.target.value)}
                style={inputStyle}
                autoComplete="current-password"
              />
            </Field>
            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
              <Field label="Neues Passwort">
                <input
                  type="password"
                  value={pwNew}
                  onChange={(e) => setPwNew(e.target.value)}
                  style={inputStyle}
                  autoComplete="new-password"
                />
              </Field>
              <Field label="Bestätigen">
                <input
                  type="password"
                  value={pwConfirm}
                  onChange={(e) => setPwConfirm(e.target.value)}
                  style={inputStyle}
                  autoComplete="new-password"
                />
              </Field>
            </div>
            {pwNew.length > 0 && pwNew.length < 12 && (
              <div style={{ fontSize: 11, color: 'var(--v5-warn)' }}>
                Mindestens 12 Zeichen erforderlich.
              </div>
            )}
            {pwNew.length >= 12 && pwConfirm.length > 0 && pwNew !== pwConfirm && (
              <div style={{ fontSize: 11, color: 'var(--v5-err)' }}>
                Passwörter stimmen nicht überein.
              </div>
            )}
            <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
              <button
                type="button"
                disabled={!canSavePassword() || passwordMutation.isPending}
                onClick={() => passwordMutation.mutate({ current: pwCurrent, next: pwNew, confirm: pwConfirm })}
                data-testid="profil-pw-submit"
                style={{
                  padding: '8px 16px',
                  borderRadius: 9,
                  background: 'var(--v5-acc)',
                  color: 'var(--v5-accent-fg)',
                  border: 'none',
                  fontSize: 12,
                  fontWeight: 600,
                  cursor: canSavePassword() && !passwordMutation.isPending ? 'pointer' : 'not-allowed',
                  opacity: canSavePassword() && !passwordMutation.isPending ? 1 : 0.5,
                }}
              >
                {passwordMutation.isPending ? 'Ändert …' : 'Passwort speichern'}
              </button>
            </div>
          </div>
        )}
      </Card>

      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 12, animationDelay: '0.18s' }}>
        <div style={{ display: 'flex', alignItems: 'center' }}>
          <SectionLabel>Sprache</SectionLabel>
          <HelpTip label="Hinweis zur Sprache">
            Greift sofort
          </HelpTip>
        </div>
        <div role="group" aria-label="Sprachen" style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          {LANGUAGES.map((l) => {
            const active = lang === l.code;
            return (
              <button
                key={l.code}
                type="button"
                aria-pressed={active}
                data-testid="profil-lang"
                onClick={() => handleLangChange(l.code)}
                style={{
                  padding: '6px 14px',
                  borderRadius: 999,
                  fontSize: 11,
                  fontWeight: 500,
                  cursor: 'pointer',
                  border: `1.5px solid ${active ? 'var(--v5-acc)' : 'var(--v5-bor)'}`,
                  background: active ? 'var(--v5-acc-muted)' : 'transparent',
                  color: active ? 'var(--v5-acc)' : 'var(--v5-mut)',
                }}
              >
                {l.label}
              </button>
            );
          })}
        </div>
      </Card>

      <Card className="v5-ani" style={{ padding: 18, display: 'flex', flexDirection: 'column', gap: 12, animationDelay: '0.24s' }}>
        <SectionLabel>Darstellung</SectionLabel>
        <div role="group" aria-label="Darstellungsmodus" style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
          {V5_MODES.map((m: V5Mode) => {
            const active = mode === m;
            return (
              <button
                key={m}
                type="button"
                aria-pressed={active}
                data-testid="profil-theme"
                onClick={() => setMode(m)}
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
                {V5_MODE_LABELS[m]}
              </button>
            );
          })}
        </div>
      </Card>
    </div>
  );
}
