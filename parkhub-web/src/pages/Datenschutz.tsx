import { motion } from 'framer-motion';
import { ShieldCheck, Lock, Database, UserCircle, FileText, Envelope } from '@phosphor-icons/react';

export function DatenschutzPage() {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="max-w-3xl mx-auto space-y-8 py-8 px-4"
    >
      <div className="flex items-center gap-3">
        <ShieldCheck weight="fill" className="w-8 h-8 text-primary-600" />
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
            Datenschutzerklärung
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
            Gemäß Art. 13/14 DSGVO &amp; § 25 TTDSG
          </p>
        </div>
      </div>

      <div className="card p-6 border-l-4 border-l-primary-500">
        <p className="text-sm text-gray-600 dark:text-gray-300">
          <span className="font-semibold">Hinweis für Betreiber:</span> Diese Datenschutzerklärung ist eine
          Vorlage. Als Betreiber dieser ParkHub-Instanz sind Sie für die Richtigkeit und Vollständigkeit
          der Angaben (insbesondere Kontaktdaten des Verantwortlichen) verantwortlich.
        </p>
      </div>

      {/* Section 1 */}
      <div className="card p-6">
        <div className="flex items-center gap-3 mb-4">
          <UserCircle weight="fill" className="w-5 h-5 text-primary-600" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            1. Erhobene Daten und Verarbeitungszwecke
          </h2>
        </div>

        <div className="space-y-6">
          <div>
            <h3 className="font-semibold text-gray-900 dark:text-white mb-2">
              1.1 Registrierung und Nutzerkonto
            </h3>
            <dl className="grid grid-cols-1 gap-1 text-sm text-gray-700 dark:text-gray-300">
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Daten:</dt>
                <dd>Name, E-Mail-Adresse, Benutzername, verschlüsseltes Passwort</dd>
              </div>
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Zweck:</dt>
                <dd>Kontoerstellung, Authentifizierung, Zugriffskontrolle</dd>
              </div>
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Rechtsgrundlage:</dt>
                <dd>Art. 6 Abs. 1 lit. b DSGVO (Vertragserfüllung)</dd>
              </div>
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Speicherdauer:</dt>
                <dd>Bis zur Löschung des Nutzerkontos</dd>
              </div>
            </dl>
          </div>

          <div className="border-t border-gray-100 dark:border-gray-800 pt-4">
            <h3 className="font-semibold text-gray-900 dark:text-white mb-2">
              1.2 Buchungsdaten
            </h3>
            <dl className="grid grid-cols-1 gap-1 text-sm text-gray-700 dark:text-gray-300">
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Daten:</dt>
                <dd>Parkhaus, Stellplatz, Zeitraum, Kennzeichen, Buchungs-ID</dd>
              </div>
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Zweck:</dt>
                <dd>Durchführung der Parkplatzbuchung, Abrechnung</dd>
              </div>
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Rechtsgrundlage:</dt>
                <dd>Art. 6 Abs. 1 lit. b DSGVO (Vertragserfüllung)</dd>
              </div>
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Speicherdauer:</dt>
                <dd>10 Jahre (§ 147 AO — gesetzliche Aufbewahrungspflicht)</dd>
              </div>
            </dl>
            <p className="text-sm text-gray-500 dark:text-gray-400 mt-3 bg-gray-50 dark:bg-gray-800/50 rounded-lg p-3">
              <span className="font-medium">Hinweis:</span> Buchungseinträge werden nach Löschung eines
              Nutzerkontos anonymisiert (Kennzeichen → [GELÖSCHT]), aber nicht vollständig gelöscht,
              da steuerrechtliche Aufbewahrungspflichten bestehen.
            </p>
          </div>

          <div className="border-t border-gray-100 dark:border-gray-800 pt-4">
            <h3 className="font-semibold text-gray-900 dark:text-white mb-2">
              1.3 Fahrzeugdaten
            </h3>
            <dl className="grid grid-cols-1 gap-1 text-sm text-gray-700 dark:text-gray-300">
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Daten:</dt>
                <dd>Kennzeichen, Fahrzeugmarke, Modell, Farbe</dd>
              </div>
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Zweck:</dt>
                <dd>Vereinfachte Buchung, Kennzeichenerkennung</dd>
              </div>
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Rechtsgrundlage:</dt>
                <dd>Art. 6 Abs. 1 lit. b DSGVO</dd>
              </div>
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Speicherdauer:</dt>
                <dd>Bis zur Löschung des Fahrzeugs oder des Nutzerkontos</dd>
              </div>
            </dl>
          </div>

          <div className="border-t border-gray-100 dark:border-gray-800 pt-4">
            <h3 className="font-semibold text-gray-900 dark:text-white mb-2">
              1.4 Protokolldaten (Server-Logs)
            </h3>
            <dl className="grid grid-cols-1 gap-1 text-sm text-gray-700 dark:text-gray-300">
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Daten:</dt>
                <dd>IP-Adresse, Zeitstempel, aufgerufene URL, HTTP-Statuscode</dd>
              </div>
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Zweck:</dt>
                <dd>Betriebssicherheit, Fehleranalyse</dd>
              </div>
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Rechtsgrundlage:</dt>
                <dd>Art. 6 Abs. 1 lit. f DSGVO (berechtigtes Interesse)</dd>
              </div>
              <div className="flex gap-2">
                <dt className="font-medium text-gray-900 dark:text-white shrink-0">Speicherdauer:</dt>
                <dd>30 Tage (rollierend)</dd>
              </div>
            </dl>
          </div>
        </div>
      </div>

      {/* Section 2 */}
      <div className="card p-6">
        <div className="flex items-center gap-3 mb-4">
          <Database weight="fill" className="w-5 h-5 text-primary-600" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            2. Keine Weitergabe an Dritte
          </h2>
        </div>
        <div className="space-y-3 text-sm text-gray-700 dark:text-gray-300">
          <p>
            <span className="font-semibold">ParkHub wird On-Premise betrieben.</span> Alle Daten verbleiben
            auf den Servern des Betreibers. Es findet keine Übermittlung an externe Dienstleister,
            Cloud-Dienste oder Dritte statt, sofern der Betreiber keine externen E-Mail-Dienste (SMTP)
            konfiguriert hat.
          </p>
          <p>
            Falls E-Mail-Benachrichtigungen aktiviert sind, werden Name, E-Mail und Buchungsdetails
            an den konfigurierten SMTP-Anbieter übermittelt. Näheres entnehmen Sie den Angaben des
            Betreibers.
          </p>
        </div>
      </div>

      {/* Section 3 */}
      <div className="card p-6">
        <div className="flex items-center gap-3 mb-4">
          <FileText weight="fill" className="w-5 h-5 text-primary-600" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            3. Ihre Rechte (Art. 15–22 DSGVO)
          </h2>
        </div>
        <div className="overflow-x-auto">
          <table className="w-full text-sm text-left">
            <thead>
              <tr className="border-b border-gray-200 dark:border-gray-700">
                <th className="pb-3 pr-4 font-semibold text-gray-900 dark:text-white w-1/3">Recht</th>
                <th className="pb-3 font-semibold text-gray-900 dark:text-white">Beschreibung</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100 dark:divide-gray-800">
              <tr>
                <td className="py-3 pr-4 font-medium text-gray-900 dark:text-white">Auskunft (Art. 15)</td>
                <td className="py-3 text-gray-600 dark:text-gray-300">
                  Sie können jederzeit alle über Sie gespeicherten Daten exportieren
                  (Profil → Datenschutz → Daten exportieren)
                </td>
              </tr>
              <tr>
                <td className="py-3 pr-4 font-medium text-gray-900 dark:text-white">Berichtigung (Art. 16)</td>
                <td className="py-3 text-gray-600 dark:text-gray-300">
                  Korrekturen von Profildaten unter Einstellungen
                </td>
              </tr>
              <tr>
                <td className="py-3 pr-4 font-medium text-gray-900 dark:text-white">Löschung (Art. 17)</td>
                <td className="py-3 text-gray-600 dark:text-gray-300">
                  Kontoauflösung unter Profil → Konto löschen (anonymisiert Buchungen, löscht PII)
                </td>
              </tr>
              <tr>
                <td className="py-3 pr-4 font-medium text-gray-900 dark:text-white">Einschränkung (Art. 18)</td>
                <td className="py-3 text-gray-600 dark:text-gray-300">
                  Auf Anfrage an den Betreiber
                </td>
              </tr>
              <tr>
                <td className="py-3 pr-4 font-medium text-gray-900 dark:text-white">Datenübertragbarkeit (Art. 20)</td>
                <td className="py-3 text-gray-600 dark:text-gray-300">
                  Export als JSON-Datei verfügbar
                </td>
              </tr>
              <tr>
                <td className="py-3 pr-4 font-medium text-gray-900 dark:text-white">Widerspruch (Art. 21)</td>
                <td className="py-3 text-gray-600 dark:text-gray-300">
                  Für auf berechtigtem Interesse beruhende Verarbeitungen
                </td>
              </tr>
              <tr>
                <td className="py-3 pr-4 font-medium text-gray-900 dark:text-white">Beschwerde</td>
                <td className="py-3 text-gray-600 dark:text-gray-300">
                  Zuständige Aufsichtsbehörde: der für den Betreiber zuständige Landesdatenschutzbeauftragte
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      {/* Section 4 */}
      <div className="card p-6">
        <div className="flex items-center gap-3 mb-4">
          <Lock weight="fill" className="w-5 h-5 text-primary-600" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            4. Technische Sicherheit
          </h2>
        </div>
        <ul className="space-y-2 text-sm text-gray-700 dark:text-gray-300">
          <li className="flex items-start gap-2">
            <span className="text-primary-600 font-bold mt-0.5">•</span>
            <span><span className="font-medium text-gray-900 dark:text-white">Transport:</span> TLS 1.3 (HTTPS)</span>
          </li>
          <li className="flex items-start gap-2">
            <span className="text-primary-600 font-bold mt-0.5">•</span>
            <span><span className="font-medium text-gray-900 dark:text-white">Passwort-Hashing:</span> Argon2id (Rust-Version) / bcrypt (PHP-Version)</span>
          </li>
          <li className="flex items-start gap-2">
            <span className="text-primary-600 font-bold mt-0.5">•</span>
            <span><span className="font-medium text-gray-900 dark:text-white">Authentifizierung:</span> JWT-Token / Laravel Sanctum</span>
          </li>
          <li className="flex items-start gap-2">
            <span className="text-primary-600 font-bold mt-0.5">•</span>
            <span><span className="font-medium text-gray-900 dark:text-white">Keine Tracking-Cookies:</span> Nur technisch notwendige Session-Tokens (localStorage)</span>
          </li>
        </ul>
      </div>

      {/* Section 5 */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          5. Cookies und lokaler Speicher
        </h2>
        <div className="space-y-3 text-sm text-gray-700 dark:text-gray-300">
          <p>
            <span className="font-semibold text-gray-900 dark:text-white">Technisch notwendige Token:</span>{' '}
            ParkHub verwendet localStorage (keine Cookies) für den Authentifizierungstoken.
            Dieser Token ist technisch notwendig für den Betrieb und enthält keine Tracking-Informationen.
          </p>
          <p className="font-medium text-gray-900 dark:text-white">
            Keine Analyse-Cookies, keine Werbe-Cookies, kein Google Analytics.
          </p>
        </div>
      </div>

      {/* Section 6 */}
      <div className="card p-6">
        <div className="flex items-center gap-3 mb-4">
          <Envelope weight="fill" className="w-5 h-5 text-primary-600" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            6. Kontakt Datenschutz
          </h2>
        </div>
        <p className="text-sm text-gray-700 dark:text-gray-300">
          Bei Fragen zur Datenverarbeitung wenden Sie sich an den Betreiber dieser ParkHub-Instanz.
          Die Kontaktdaten finden Sie im{' '}
          <a href="/impressum" className="text-primary-600 hover:underline dark:text-primary-400">
            Impressum
          </a>.
        </p>
      </div>

      <div className="card p-4 bg-gray-50 dark:bg-gray-800/50">
        <p className="text-xs text-gray-500 dark:text-gray-400">
          Diese Datenschutzerklärung basiert auf einer Vorlage für ParkHub-Betreiber und stellt keinen
          Rechtsrat dar. Für die Richtigkeit und Vollständigkeit ist der Betreiber dieser ParkHub-Instanz
          verantwortlich.
        </p>
      </div>
    </motion.div>
  );
}
