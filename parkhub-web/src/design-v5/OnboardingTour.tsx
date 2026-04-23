import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Badge, Card, SectionLabel, Toggle, V5NamedIcon } from './primitives';
import { HelpTip } from './primitives/HelpTip';
import { V5ThemeProvider } from './ThemeProvider';
import { V5ToastProvider, useV5Toast } from './Toast';
import './fonts';
import './tokens.css';

const STORAGE_SEEN = 'parkhub_onboarding_v5_seen';
const STORAGE_PREFS = 'parkhub_onboarding_v5_prefs';

/**
 * Has the user finished the v5 onboarding tour? Callers can use this to
 * decide whether to redirect freshly-logged-in users through the tour.
 */
export function hasSeenOnboardingTour(): boolean {
  if (typeof window === 'undefined') return true;
  return window.localStorage.getItem(STORAGE_SEEN) === '1';
}

export function markOnboardingTourSeen(): void {
  window.localStorage.setItem(STORAGE_SEEN, '1');
}

interface FeatureToggle {
  id: string;
  label: string;
  description: string;
  defaultOn: boolean;
  /** Features the user can't disable without breaking core flows. */
  required?: boolean;
}

const FEATURES: readonly FeatureToggle[] = [
  {
    id: 'bookings',
    label: 'Buchungen',
    description: 'Plätze für Mitarbeiter reservieren — das Kernmodul.',
    defaultOn: true,
    required: true,
  },
  {
    id: 'credits',
    label: 'Credits-System',
    description: 'Monatliches Kontingent pro Nutzer, fairer Zugang bei knappen Plätzen.',
    defaultOn: true,
  },
  {
    id: 'ev',
    label: 'EV-Laden',
    description: 'Ladestationen + Sessions tracken, Ladevorgang live anzeigen.',
    defaultOn: true,
  },
  {
    id: 'swap',
    label: 'Tausch-Anfragen',
    description: 'Nutzer:innen können Plätze untereinander tauschen.',
    defaultOn: true,
  },
  {
    id: 'waitlist',
    label: 'Warteliste',
    description: 'Automatische Benachrichtigung wenn gewünschter Platz frei wird.',
    defaultOn: true,
  },
  {
    id: 'guest_pass',
    label: 'Gäste-Pässe',
    description: 'Zeitlich begrenzte Pässe für Besucher:innen per Link.',
    defaultOn: false,
  },
  {
    id: 'analytics',
    label: 'Analytics',
    description: 'Auslastung, Trends, Spitzenzeiten — nur für Admins.',
    defaultOn: true,
  },
  {
    id: 'ai_suggestions',
    label: 'KI-Empfehlungen',
    description: 'Vorschläge zu Buchungszeiten und EV-Plänen. Läuft lokal im Browser.',
    defaultOn: false,
  },
];

type Step = 'privacy' | 'features' | 'trust';
const STEPS: readonly Step[] = ['privacy', 'features', 'trust'];

function StepIndicator({ current }: { current: Step }) {
  const idx = STEPS.indexOf(current);
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 0, padding: '0 4px' }}>
      {STEPS.map((s, i) => {
        const done = i < idx;
        const here = i === idx;
        return (
          <div key={s} style={{ display: 'flex', alignItems: 'center', flex: i < STEPS.length - 1 ? 1 : 0 }}>
            <div
              style={{
                width: 28,
                height: 28,
                borderRadius: '50%',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                background: done || here ? 'var(--v5-acc)' : 'var(--v5-bor)',
                color: done || here ? 'var(--v5-accent-fg)' : 'var(--v5-mut)',
                fontSize: 12,
                fontWeight: 700,
              }}
            >
              {done ? '✓' : i + 1}
            </div>
            {i < STEPS.length - 1 && (
              <div
                style={{
                  flex: 1,
                  height: 2,
                  background: done ? 'var(--v5-acc)' : 'var(--v5-bor)',
                  margin: '0 10px',
                }}
              />
            )}
          </div>
        );
      })}
    </div>
  );
}

function PrivacyStep() {
  const { t } = useTranslation();
  const items = [
    {
      icon: 'shield' as const,
      title: t('tour.privacy.self.title', 'Self-hosted — Ihre Daten bleiben bei Ihnen'),
      body: t(
        'tour.privacy.self.body',
        'ParkHub läuft auf Ihrer eigenen Infrastruktur. Wir haben keinen Zugriff auf Ihre Buchungen, Fahrzeuge oder Nutzerdaten — weder live noch in Backups.'
      ),
    },
    {
      icon: 'key' as const,
      title: t('tour.privacy.encryption.title', 'Verschlüsselung by default'),
      body: t(
        'tour.privacy.encryption.body',
        'Alle Verbindungen TLS 1.3. Passwörter als Argon2-Hashes. Session-Tokens mit Family-Rotation + optionaler Redis-Revocation für Multi-Replica.'
      ),
    },
    {
      icon: 'info' as const,
      title: t('tour.privacy.gdpr.title', 'GDPR-konform seit Tag 1'),
      body: t(
        'tour.privacy.gdpr.body',
        'Auskunftsrecht (Art. 15), Löschrecht (Art. 17), Datenportabilität (Art. 20) als Self-Service-Endpoints. Audit-Log für jeden Daten-Access. Impressum + Datenschutz-Seite integriert.'
      ),
    },
    {
      icon: 'check' as const,
      title: t('tour.privacy.minimization.title', 'Datensparsamkeit'),
      body: t(
        'tour.privacy.minimization.body',
        'Nur was für den Parkbetrieb nötig ist: Name, Kennzeichen, Buchungszeit. Keine Tracking-Cookies, keine Drittanbieter-Analytics, keine Werbe-SDKs.'
      ),
    },
  ];
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
      <div>
        <SectionLabel>Schritt 1 von 3 · Ihre Daten</SectionLabel>
        <h2 style={{ fontSize: 22, fontWeight: 700, color: 'var(--v5-txt)', letterSpacing: '-0.5px', margin: '2px 0 6px' }}>
          {t('tour.privacy.title', 'Volle Transparenz zu Ihren Daten')}
        </h2>
        <p style={{ fontSize: 13, color: 'var(--v5-mut)', lineHeight: 1.6, margin: 0 }}>
          {t(
            'tour.privacy.intro',
            'Bevor Sie starten — so gehen wir mit Ihren Daten um. Keine versteckten Klauseln, keine Opt-Outs im Kleingedruckten.'
          )}
        </p>
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 10 }}>
        {items.map((it) => (
          <Card key={it.title} style={{ padding: 14 }}>
            <div
              style={{
                width: 32,
                height: 32,
                borderRadius: 10,
                background: 'var(--v5-acc-muted)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                marginBottom: 10,
              }}
            >
              <V5NamedIcon name={it.icon} size={15} color="var(--v5-acc)" />
            </div>
            <div style={{ fontSize: 13, fontWeight: 600, color: 'var(--v5-txt)', marginBottom: 4 }}>
              {it.title}
            </div>
            <p style={{ fontSize: 11, color: 'var(--v5-mut)', lineHeight: 1.55, margin: 0 }}>{it.body}</p>
          </Card>
        ))}
      </div>
    </div>
  );
}

function FeaturesStep({
  selection,
  onChange,
}: {
  selection: Record<string, boolean>;
  onChange: (id: string, next: boolean) => void;
}) {
  const { t } = useTranslation();
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
      <div>
        <SectionLabel>Schritt 2 von 3 · Module</SectionLabel>
        <h2 style={{ fontSize: 22, fontWeight: 700, color: 'var(--v5-txt)', letterSpacing: '-0.5px', margin: '2px 0 6px' }}>
          {t('tour.features.title', 'Wählen Sie die Features die Sie brauchen')}
        </h2>
        <p style={{ fontSize: 13, color: 'var(--v5-mut)', lineHeight: 1.6, margin: 0 }}>
          {t(
            'tour.features.intro',
            'Jedes Modul ist abschaltbar. Deaktivierte Features verschwinden aus der Navigation und verbrauchen keine Ressourcen. Sie können das jederzeit unter Einstellungen ändern.'
          )}
        </p>
      </div>
      <Card style={{ padding: 0, overflow: 'hidden' }}>
        {FEATURES.map((f, i) => {
          const checked = selection[f.id] ?? f.defaultOn;
          return (
            <div
              key={f.id}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 14,
                padding: '14px 18px',
                borderBottom: i < FEATURES.length - 1 ? '1px solid var(--v5-bor)' : 'none',
              }}
            >
              <div style={{ flex: 1 }}>
                <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--v5-txt)' }}>{f.label}</span>
                  {f.required && <Badge variant="primary">Pflicht</Badge>}
                  <HelpTip label={`Erklärung zu ${f.label}`}>{f.description}</HelpTip>
                </div>
                <p style={{ fontSize: 11, color: 'var(--v5-mut)', lineHeight: 1.5, margin: '2px 0 0' }}>
                  {f.description}
                </p>
              </div>
              <Toggle
                checked={f.required ? true : checked}
                onChange={f.required ? undefined : (next) => onChange(f.id, next)}
                ariaLabel={`${f.label} ${checked ? 'deaktivieren' : 'aktivieren'}`}
              />
            </div>
          );
        })}
      </Card>
    </div>
  );
}

function TrustStep() {
  const { t } = useTranslation();
  const badges = [
    {
      icon: 'check' as const,
      label: '2147+ Tests',
      sub: 'Unit + Integration + E2E auf jedem PR',
    },
    {
      icon: 'shield' as const,
      label: 'GDPR',
      sub: 'Art. 15/17/20 als Self-Service',
    },
    {
      icon: 'key' as const,
      label: 'TLS 1.3 + Argon2',
      sub: 'Keine Klartext-Passwörter, keine SHA-1',
    },
    {
      icon: 'analytics' as const,
      label: 'Lighthouse CI',
      sub: 'Core Web Vitals gated auf jedem Build',
    },
    {
      icon: 'info' as const,
      label: 'OpenAPI',
      sub: '99% Coverage, Drift-Gate im CI',
    },
    {
      icon: 'users' as const,
      label: 'Audit-Log',
      sub: 'Jeder Daten-Access nachvollziehbar',
    },
  ];
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 14 }}>
      <div>
        <SectionLabel>Schritt 3 von 3 · Vertrauen</SectionLabel>
        <h2 style={{ fontSize: 22, fontWeight: 700, color: 'var(--v5-txt)', letterSpacing: '-0.5px', margin: '2px 0 6px' }}>
          {t('tour.trust.title', 'Warum Sie ParkHub vertrauen können')}
        </h2>
        <p style={{ fontSize: 13, color: 'var(--v5-mut)', lineHeight: 1.6, margin: 0 }}>
          {t(
            'tour.trust.intro',
            'ParkHub wird öffentlich entwickelt. Jede Zeile Code, jeder Build, jeder Audit ist einsehbar. Keine Marketing-Claims — hier ist die Substanz.'
          )}
        </p>
      </div>
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 10 }}>
        {badges.map((b) => (
          <Card key={b.label} style={{ padding: 14, textAlign: 'center' }}>
            <div
              style={{
                width: 40,
                height: 40,
                borderRadius: 12,
                background: 'var(--v5-acc-muted)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                margin: '0 auto 8px',
              }}
            >
              <V5NamedIcon name={b.icon} size={18} color="var(--v5-acc)" />
            </div>
            <div style={{ fontSize: 13, fontWeight: 700, color: 'var(--v5-txt)', marginBottom: 2 }}>
              {b.label}
            </div>
            <div style={{ fontSize: 10, color: 'var(--v5-mut)', lineHeight: 1.5 }}>{b.sub}</div>
          </Card>
        ))}
      </div>
      <Card
        style={{
          padding: 14,
          background: 'linear-gradient(135deg, var(--v5-acc-muted), transparent)',
          border: '1px solid color-mix(in oklch, var(--v5-acc) 30%, transparent)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 6 }}>
          <V5NamedIcon name="shield" size={14} color="var(--v5-acc)" />
          <span style={{ fontSize: 12, fontWeight: 700, color: 'var(--v5-acc)', letterSpacing: 0.3, textTransform: 'uppercase' }}>
            {t('tour.trust.openBadge', 'Open by default')}
          </span>
        </div>
        <p style={{ fontSize: 12, color: 'var(--v5-txt)', lineHeight: 1.6, margin: 0 }}>
          {t(
            'tour.trust.openBody',
            'Source auf GitHub, Security-Audits öffentlich, Vulnerability-Disclosure-Policy aktiv. Fragen Sie Ihr Team — oder uns direkt.'
          )}
        </p>
      </Card>
    </div>
  );
}

function TourInner() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const toast = useV5Toast();
  const [step, setStep] = useState<Step>('privacy');
  const [selection, setSelection] = useState<Record<string, boolean>>(() => {
    if (typeof window === 'undefined') return {};
    try {
      const stored = window.localStorage.getItem(STORAGE_PREFS);
      if (stored) return JSON.parse(stored) as Record<string, boolean>;
    } catch {
      /* ignore corrupted prefs */
    }
    return Object.fromEntries(FEATURES.map((f) => [f.id, f.defaultOn]));
  });

  useEffect(() => {
    window.localStorage.setItem(STORAGE_PREFS, JSON.stringify(selection));
  }, [selection]);

  const idx = STEPS.indexOf(step);
  const isLast = idx === STEPS.length - 1;

  const next = () => {
    if (isLast) {
      markOnboardingTourSeen();
      toast(t('tour.complete', 'Willkommen! Ihr ParkHub ist einsatzbereit.'), 'success');
      // Small delay so the toast is visible before we navigate away
      setTimeout(() => navigate('/', { replace: true }), 600);
      return;
    }
    setStep(STEPS[idx + 1]);
  };

  const skip = () => {
    markOnboardingTourSeen();
    navigate('/', { replace: true });
  };

  const stepContent = useMemo(() => {
    switch (step) {
      case 'privacy':
        return <PrivacyStep />;
      case 'features':
        return (
          <FeaturesStep
            selection={selection}
            onChange={(id, val) => setSelection((s) => ({ ...s, [id]: val }))}
          />
        );
      case 'trust':
        return <TrustStep />;
    }
  }, [step, selection]);

  return (
    <div
      style={{
        minHeight: '100dvh',
        background: 'var(--v5-bg)',
        color: 'var(--v5-txt)',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        padding: '48px 20px',
        fontFamily: "'Inter Variable', 'Inter', system-ui, sans-serif",
      }}
      data-testid="onboarding-tour"
    >
      <div style={{ width: '100%', maxWidth: 720 }}>
        <StepIndicator current={step} />
        <div key={step} className="v5-ani" style={{ marginTop: 28 }}>
          {stepContent}
        </div>
        <div
          style={{
            marginTop: 24,
            display: 'flex',
            alignItems: 'center',
            gap: 10,
          }}
        >
          <button
            type="button"
            onClick={skip}
            style={{
              padding: '9px 16px',
              borderRadius: 10,
              background: 'transparent',
              color: 'var(--v5-mut)',
              border: 0,
              fontSize: 12,
              cursor: 'pointer',
              fontFamily: 'inherit',
            }}
          >
            {t('tour.skip', 'Überspringen')}
          </button>
          <div style={{ flex: 1 }} />
          {idx > 0 && (
            <button
              type="button"
              onClick={() => setStep(STEPS[idx - 1])}
              style={{
                padding: '9px 16px',
                borderRadius: 10,
                background: 'var(--v5-sur2)',
                color: 'var(--v5-txt)',
                border: '1px solid var(--v5-bor)',
                fontSize: 12,
                cursor: 'pointer',
                fontFamily: 'inherit',
              }}
            >
              {t('tour.back', 'Zurück')}
            </button>
          )}
          <button
            type="button"
            onClick={next}
            className="v5-btn"
            style={{
              padding: '10px 22px',
              borderRadius: 10,
              background: 'var(--v5-acc)',
              color: 'var(--v5-accent-fg)',
              border: 0,
              fontSize: 12,
              fontWeight: 600,
              cursor: 'pointer',
              fontFamily: 'inherit',
            }}
          >
            {isLast ? t('tour.finish', 'Los geht\'s') : t('tour.next', 'Weiter')}
          </button>
        </div>
      </div>
    </div>
  );
}

/**
 * Stand-alone mount for the onboarding tour. Wraps its own V5ThemeProvider
 * and toast provider so it works even before the main app context tree is
 * mounted (e.g. rendered from a dedicated /welcome page).
 */
export function OnboardingTour() {
  return (
    <V5ThemeProvider>
      <V5ToastProvider>
        <TourInner />
      </V5ToastProvider>
    </V5ThemeProvider>
  );
}
