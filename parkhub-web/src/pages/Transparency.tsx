import { Link } from 'react-router-dom';
import { motion } from 'framer-motion';
import {
  ShieldCheck,
  Database,
  Lock,
  Globe,
  UserCircle,
  FileText,
  ArrowSquareOut,
  CheckCircle,
  Warning,
  Buildings,
} from '@phosphor-icons/react';

interface DataRow {
  category: string;
  data: string;
  purpose: string;
  basis: string;
  retention: string;
}

const DATA_TABLE: DataRow[] = [
  {
    category: 'Konto',
    data: 'Name, E-Mail, Benutzername, Passwort-Hash',
    purpose: 'Authentifizierung, Zugriffskontrolle',
    basis: 'Art. 6 Abs. 1 lit. b DSGVO',
    retention: 'Bis zur Kontolöschung',
  },
  {
    category: 'Buchungen',
    data: 'Stellplatz, Zeitraum, Kennzeichen, Buchungs-ID',
    purpose: 'Buchungsdurchführung, Abrechnung',
    basis: 'Art. 6 Abs. 1 lit. b DSGVO',
    retention: '10 Jahre (§ 147 AO)',
  },
  {
    category: 'Fahrzeuge',
    data: 'Kennzeichen, Marke, Modell, Farbe',
    purpose: 'Vereinfachte Buchung',
    basis: 'Art. 6 Abs. 1 lit. b DSGVO',
    retention: 'Bis zur Löschung des Fahrzeugs',
  },
  {
    category: 'Server-Logs',
    data: 'IP-Adresse, Zeitstempel, aufgerufene URL, HTTP-Statuscode',
    purpose: 'Betriebssicherheit, Fehleranalyse',
    basis: 'Art. 6 Abs. 1 lit. f DSGVO',
    retention: '30 Tage (rollierend)',
  },
];

const SECURITY_FEATURES = [
  { label: 'Transport', value: 'TLS 1.3 (HTTPS)' },
  { label: 'Passwort-Hashing', value: 'Argon2id' },
  { label: 'Authentifizierung', value: 'JWT (HS256, Ablauf 7 Tage)' },
  { label: 'Session-Speicher', value: 'localStorage (kein HTTP-Cookie)' },
  { label: 'Security-Header', value: 'X-Content-Type-Options, X-Frame-Options, CSP, Referrer-Policy' },
  { label: 'Rate Limiting', value: '5 Login-Versuche/Minute pro IP' },
  { label: 'Anfragegröße', value: 'Max. 1 MiB (DoS-Schutz)' },
  { label: 'CORS', value: 'Gleiche Origin; kein Wildcard' },
];

const THIRD_PARTY = [
  { service: 'Google Analytics', used: false },
  { service: 'Meta Pixel / Facebook', used: false },
  { service: 'Tracking-Cookies', used: false },
  { service: 'Cloud-Dienste (AWS, Azure, GCP)', used: false },
  { service: 'CDN (Cloudflare etc.)', used: false },
  { service: 'Externe Fonts (Google Fonts etc.)', used: false },
  { service: 'Push-Dienste (Firebase etc.)', used: false },
];

export function TransparencyPage() {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="max-w-4xl mx-auto space-y-8 py-8 px-4"
    >
      {/* Header */}
      <div className="flex items-center gap-3">
        <ShieldCheck weight="fill" className="w-8 h-8 text-primary-600" />
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
            Datenschutz &amp; Transparenz
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
            Was wir speichern, wie wir es schützen und welche Rechte Sie haben
          </p>
        </div>
      </div>

      {/* On-premise badge */}
      <div className="card p-5 flex items-start gap-4 border-l-4 border-l-green-500 bg-green-50 dark:bg-green-900/10">
        <Buildings weight="fill" className="w-6 h-6 text-green-600 dark:text-green-400 shrink-0 mt-0.5" />
        <div>
          <p className="font-semibold text-gray-900 dark:text-white">
            On-Premise — Ihre Daten bleiben bei Ihnen
          </p>
          <p className="text-sm text-gray-600 dark:text-gray-300 mt-1">
            ParkHub wird auf den eigenen Servern Ihres Betreibers betrieben. Alle Daten werden
            ausschließlich dort gespeichert. Es findet keine Übermittlung an externe Cloud-Dienste
            oder Dritte statt (außer bei konfiguriertem SMTP-E-Mail-Versand).
          </p>
        </div>
      </div>

      {/* Data table */}
      <div className="card p-6">
        <div className="flex items-center gap-3 mb-5">
          <Database weight="fill" className="w-5 h-5 text-primary-600" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            Welche Daten werden gespeichert?
          </h2>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm text-left">
            <thead>
              <tr className="border-b border-gray-200 dark:border-gray-700">
                <th className="pb-3 pr-3 font-semibold text-gray-900 dark:text-white">Kategorie</th>
                <th className="pb-3 pr-3 font-semibold text-gray-900 dark:text-white">Daten</th>
                <th className="pb-3 pr-3 font-semibold text-gray-900 dark:text-white hidden sm:table-cell">Zweck</th>
                <th className="pb-3 pr-3 font-semibold text-gray-900 dark:text-white hidden md:table-cell">Rechtsgrundlage</th>
                <th className="pb-3 font-semibold text-gray-900 dark:text-white">Speicherdauer</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100 dark:divide-gray-800">
              {DATA_TABLE.map(row => (
                <tr key={row.category}>
                  <td className="py-3 pr-3 font-medium text-gray-900 dark:text-white whitespace-nowrap">
                    {row.category}
                  </td>
                  <td className="py-3 pr-3 text-gray-600 dark:text-gray-300">{row.data}</td>
                  <td className="py-3 pr-3 text-gray-600 dark:text-gray-300 hidden sm:table-cell">{row.purpose}</td>
                  <td className="py-3 pr-3 text-gray-600 dark:text-gray-300 hidden md:table-cell font-mono text-xs">
                    {row.basis}
                  </td>
                  <td className="py-3 text-gray-600 dark:text-gray-300 whitespace-nowrap">{row.retention}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p className="text-xs text-gray-500 dark:text-gray-400 mt-4 bg-gray-50 dark:bg-gray-800/50 rounded-lg p-3">
          <span className="font-medium">Hinweis zur Kontolöschung:</span> Bei Löschung eines Nutzerkontos werden
          Buchungseinträge anonymisiert (Kennzeichen und Name werden durch „[GELÖSCHT]" ersetzt),
          nicht vollständig gelöscht — da steuerrechtliche Aufbewahrungspflichten nach § 147 AO bestehen.
        </p>
      </div>

      {/* No third parties */}
      <div className="card p-6">
        <div className="flex items-center gap-3 mb-5">
          <Globe weight="fill" className="w-5 h-5 text-primary-600" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            Keine externen Dienste
          </h2>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
          {THIRD_PARTY.map(({ service, used }) => (
            <div key={service} className="flex items-center gap-2 text-sm">
              {used ? (
                <Warning weight="fill" className="w-4 h-4 text-amber-500 shrink-0" />
              ) : (
                <CheckCircle weight="fill" className="w-4 h-4 text-green-500 shrink-0" />
              )}
              <span className={used ? 'text-amber-700 dark:text-amber-400' : 'text-gray-600 dark:text-gray-300'}>
                {service}
                {used ? ' — vorhanden' : ' — nicht verwendet'}
              </span>
            </div>
          ))}
        </div>
        <p className="text-xs text-gray-500 dark:text-gray-400 mt-4">
          Alle Assets (JavaScript, CSS, Icons) werden vom eigenen Server ausgeliefert.
          Es werden keine externen URLs beim Laden der App kontaktiert.
        </p>
      </div>

      {/* Security features */}
      <div className="card p-6">
        <div className="flex items-center gap-3 mb-5">
          <Lock weight="fill" className="w-5 h-5 text-primary-600" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            Technische Sicherheitsmaßnahmen
          </h2>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          {SECURITY_FEATURES.map(({ label, value }) => (
            <div key={label} className="flex gap-2 text-sm">
              <span className="font-medium text-gray-900 dark:text-white shrink-0">{label}:</span>
              <span className="text-gray-600 dark:text-gray-300">{value}</span>
            </div>
          ))}
        </div>
      </div>

      {/* GDPR rights */}
      <div className="card p-6">
        <div className="flex items-center gap-3 mb-5">
          <UserCircle weight="fill" className="w-5 h-5 text-primary-600" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            Ihre DSGVO-Rechte (Art. 15–22)
          </h2>
        </div>
        <div className="space-y-3">
          <div className="flex items-start gap-3 p-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl text-sm">
            <CheckCircle weight="fill" className="w-4 h-4 text-green-500 shrink-0 mt-0.5" />
            <div>
              <span className="font-medium text-gray-900 dark:text-white">Auskunft (Art. 15) &amp; Datenportabilität (Art. 20):</span>
              <span className="text-gray-600 dark:text-gray-300 ml-1">
                Laden Sie alle Ihre Daten als JSON-Datei herunter unter{' '}
                <Link to="/profile" className="text-primary-600 hover:underline dark:text-primary-400">
                  Profil → Datenschutz → Daten exportieren
                </Link>.
              </span>
            </div>
          </div>
          <div className="flex items-start gap-3 p-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl text-sm">
            <CheckCircle weight="fill" className="w-4 h-4 text-green-500 shrink-0 mt-0.5" />
            <div>
              <span className="font-medium text-gray-900 dark:text-white">Löschung (Art. 17):</span>
              <span className="text-gray-600 dark:text-gray-300 ml-1">
                Konto löschen unter{' '}
                <Link to="/profile" className="text-primary-600 hover:underline dark:text-primary-400">
                  Profil → Konto löschen
                </Link>.
                Ihre persönlichen Daten werden anonymisiert; anonymisierte Buchungseinträge
                werden aus steuerrechtlichen Gründen aufbewahrt.
              </span>
            </div>
          </div>
          <div className="flex items-start gap-3 p-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl text-sm">
            <CheckCircle weight="fill" className="w-4 h-4 text-green-500 shrink-0 mt-0.5" />
            <div>
              <span className="font-medium text-gray-900 dark:text-white">Berichtigung (Art. 16):</span>
              <span className="text-gray-600 dark:text-gray-300 ml-1">
                Profildaten können jederzeit unter{' '}
                <Link to="/profile" className="text-primary-600 hover:underline dark:text-primary-400">
                  Profil
                </Link>{' '}
                geändert werden.
              </span>
            </div>
          </div>
          <div className="flex items-start gap-3 p-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl text-sm">
            <CheckCircle weight="fill" className="w-4 h-4 text-green-500 shrink-0 mt-0.5" />
            <div>
              <span className="font-medium text-gray-900 dark:text-white">Einschränkung (Art. 18), Widerspruch (Art. 21), Beschwerde:</span>
              <span className="text-gray-600 dark:text-gray-300 ml-1">
                Wenden Sie sich an den Betreiber dieser ParkHub-Instanz (Kontakt im{' '}
                <Link to="/impressum" className="text-primary-600 hover:underline dark:text-primary-400">
                  Impressum
                </Link>).
                Beschwerden können Sie an Ihre zuständige Landesdatenschutzbehörde richten.
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Links */}
      <div className="card p-6">
        <div className="flex items-center gap-3 mb-4">
          <FileText weight="fill" className="w-5 h-5 text-primary-600" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            Rechtliche Dokumente
          </h2>
        </div>
        <div className="flex flex-wrap gap-3">
          <Link
            to="/datenschutz"
            className="inline-flex items-center gap-2 text-sm text-primary-600 hover:underline dark:text-primary-400"
          >
            <FileText weight="fill" className="w-4 h-4" />
            Datenschutzerklärung (vollständig)
          </Link>
          <span className="text-gray-300 dark:text-gray-600">|</span>
          <Link
            to="/impressum"
            className="inline-flex items-center gap-2 text-sm text-primary-600 hover:underline dark:text-primary-400"
          >
            <Buildings weight="fill" className="w-4 h-4" />
            Impressum
          </Link>
          <span className="text-gray-300 dark:text-gray-600">|</span>
          <Link
            to="/agb"
            className="inline-flex items-center gap-2 text-sm text-primary-600 hover:underline dark:text-primary-400"
          >
            <FileText weight="fill" className="w-4 h-4" />
            AGB
          </Link>
          <span className="text-gray-300 dark:text-gray-600">|</span>
          <a
            href="https://github.com/nash87/parkhub-rust"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-2 text-sm text-primary-600 hover:underline dark:text-primary-400"
          >
            <ArrowSquareOut weight="fill" className="w-4 h-4" />
            Open Source (GitHub)
          </a>
        </div>
      </div>

      <div className="card p-4 bg-gray-50 dark:bg-gray-800/50">
        <p className="text-xs text-gray-500 dark:text-gray-400">
          Diese Seite gibt Auskunft über die Standardkonfiguration von ParkHub. Der Betreiber
          dieser Instanz ist für die tatsächliche Umsetzung und die Vollständigkeit aller
          Datenschutzangaben verantwortlich. Letzte Überprüfung der Softwareversion: 2026.
        </p>
      </div>
    </motion.div>
  );
}
