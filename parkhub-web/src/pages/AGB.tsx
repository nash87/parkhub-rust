import { motion } from 'framer-motion';
import { FileText, Warning } from '@phosphor-icons/react';

export function AGBPage() {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="max-w-3xl mx-auto space-y-8 py-8 px-4"
    >
      <div className="flex items-center gap-3">
        <FileText weight="fill" className="w-8 h-8 text-primary-600" />
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
            Allgemeine Geschäftsbedingungen (AGB)
          </h1>
          <p className="text-sm text-gray-500 dark:text-gray-400 mt-0.5">
            Gemäß §§ 305–310 BGB (AGB-Recht)
          </p>
        </div>
      </div>

      <div className="card p-6 border-l-4 border-l-amber-500 flex items-start gap-3">
        <Warning weight="fill" className="w-5 h-5 text-amber-500 mt-0.5 shrink-0" />
        <p className="text-sm text-gray-600 dark:text-gray-300">
          <span className="font-semibold">Hinweis für Betreiber:</span> Diese AGB sind eine Vorlage und
          müssen von Ihnen individuell angepasst werden (insbesondere Betreiberdaten, Stornobedingungen,
          Preise und Zahlungsmodalitäten). Eine rechtliche Überprüfung wird empfohlen.
        </p>
      </div>

      {/* § 1 */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          § 1 Geltungsbereich
        </h2>
        <div className="space-y-3 text-sm text-gray-700 dark:text-gray-300">
          <p>
            (1) Diese Allgemeinen Geschäftsbedingungen (AGB) gelten für alle Buchungen von Parkplätzen
            über das ParkHub-System des Betreibers.
          </p>
          <p>
            (2) Abweichende Bedingungen des Nutzers werden nicht anerkannt, es sei denn, der Betreiber
            stimmt ihrer Geltung ausdrücklich schriftlich zu.
          </p>
          <p>
            (3) Diese AGB gelten sowohl für Verbraucher (§ 13 BGB) als auch für Unternehmer (§ 14 BGB),
            soweit nicht ausdrücklich anders angegeben.
          </p>
        </div>
      </div>

      {/* § 2 */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          § 2 Vertragsschluss
        </h2>
        <div className="space-y-3 text-sm text-gray-700 dark:text-gray-300">
          <p>
            (1) Durch die Registrierung auf der ParkHub-Plattform erklärt der Nutzer, diese AGB
            anerkannt zu haben.
          </p>
          <p>
            (2) Eine Buchung kommt zustande, wenn der Nutzer den Buchungsprozess abschließt und
            eine Buchungsbestätigung (per E-Mail oder in der App) erhalten hat.
          </p>
          <p>
            (3) Der Betreiber ist berechtigt, Buchungsanfragen ohne Angabe von Gründen abzulehnen.
          </p>
        </div>
      </div>

      {/* § 3 */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          § 3 Buchung und Nutzung
        </h2>
        <div className="space-y-3 text-sm text-gray-700 dark:text-gray-300">
          <p>
            (1) Der Nutzer darf den gebuchten Stellplatz nur für das angegebene Fahrzeug und im
            gebuchten Zeitraum nutzen.
          </p>
          <p>
            (2) Eine Untermiete oder Weitergabe des Stellplatzes an Dritte ist nicht gestattet.
          </p>
          <p>
            (3) Das Überschreiten des gebuchten Zeitraums ist unverzüglich zu melden und kann
            zusätzliche Gebühren auslösen.
          </p>
        </div>
      </div>

      {/* § 4 */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          § 4 Stornierung und Widerrufsrecht
        </h2>
        <div className="space-y-4 text-sm text-gray-700 dark:text-gray-300">
          <div>
            <h3 className="font-semibold text-gray-900 dark:text-white mb-2">
              4.1 Stornierung durch den Nutzer
            </h3>
            <p className="mb-2">
              (1) Buchungen können bis zu einem vom Betreiber festgelegten Zeitpunkt vor Buchungsbeginn
              kostenfrei storniert werden. Die genaue Frist entnehmen Sie den Buchungsdetails.
            </p>
            <p>
              (2) Bei Stornierung nach dieser Frist können Stornogebühren anfallen. Näheres regelt der
              Betreiber in seinen Preisangaben.
            </p>
          </div>

          <div className="border-t border-gray-100 dark:border-gray-800 pt-4">
            <h3 className="font-semibold text-gray-900 dark:text-white mb-2">
              4.2 Widerrufsrecht (Verbraucher)
            </h3>
            <p className="mb-3">
              Wenn Sie Verbraucher sind (§ 13 BGB), steht Ihnen ein gesetzliches Widerrufsrecht zu:
            </p>
            <div className="bg-gray-50 dark:bg-gray-800/50 rounded-xl p-4 space-y-3">
              <p className="font-semibold text-gray-900 dark:text-white">Widerrufsbelehrung</p>
              <p>
                Sie haben das Recht, binnen vierzehn Tagen ohne Angabe von Gründen diesen Vertrag zu
                widerrufen. Die Widerrufsfrist beträgt vierzehn Tage ab dem Tag des Vertragsschlusses.
              </p>
              <p>
                Um Ihr Widerrufsrecht auszuüben, müssen Sie den Betreiber (Kontaktdaten im Impressum)
                mittels einer eindeutigen Erklärung (z. B. ein per Post versandter Brief oder eine E-Mail)
                über Ihren Entschluss, diesen Vertrag zu widerrufen, informieren.
              </p>
              <p className="font-semibold text-gray-900 dark:text-white">Erlöschen des Widerrufsrechts</p>
              <p>
                Das Widerrufsrecht erlischt bei Dienstleistungen, wenn der Vertrag vollständig erfüllt ist
                und mit der Ausführung erst begonnen wurde, nachdem der Verbraucher dazu seine ausdrückliche
                Zustimmung gegeben hat (§ 356 Abs. 4 BGB).
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* § 5 */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          § 5 Preise und Zahlung
        </h2>
        <div className="space-y-3 text-sm text-gray-700 dark:text-gray-300">
          <p>
            (1) Es gelten die zum Zeitpunkt der Buchung angezeigten Preise.
          </p>
          <p>
            (2) Alle Preise verstehen sich inklusive der gesetzlichen Mehrwertsteuer, sofern nicht
            anders angegeben.
          </p>
          <p>
            (3) Die Zahlungsmodalitäten werden vom Betreiber festgelegt und in der Buchungsmaske
            angezeigt.
          </p>
        </div>
      </div>

      {/* § 6 */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          § 6 Haftung
        </h2>
        <div className="space-y-3 text-sm text-gray-700 dark:text-gray-300">
          <p>
            (1) Der Betreiber haftet nicht für Schäden an Fahrzeugen oder eingebrachten Gegenständen,
            es sei denn, diese beruhen auf vorsätzlichem oder grob fahrlässigem Verhalten des
            Betreibers oder seiner Erfüllungsgehilfen.
          </p>
          <p>
            (2) Die Haftung für leichte Fahrlässigkeit ist auf vorhersehbare, vertragstypische Schäden
            begrenzt.
          </p>
          <p>
            (3) Die vorstehenden Haftungsbeschränkungen gelten nicht bei Verletzung von Leben, Körper
            oder Gesundheit.
          </p>
        </div>
      </div>

      {/* § 7 */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          § 7 Datenschutz
        </h2>
        <p className="text-sm text-gray-700 dark:text-gray-300">
          Informationen zur Verarbeitung personenbezogener Daten finden Sie in unserer{' '}
          <a href="/datenschutz" className="text-primary-600 hover:underline dark:text-primary-400">
            Datenschutzerklärung
          </a>.
        </p>
      </div>

      {/* § 8 */}
      <div className="card p-6">
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          § 8 Schlussbestimmungen
        </h2>
        <div className="space-y-3 text-sm text-gray-700 dark:text-gray-300">
          <p>
            (1) Es gilt das Recht der Bundesrepublik Deutschland.
          </p>
          <p>
            (2) Gerichtsstand für Kaufleute und juristische Personen des öffentlichen Rechts ist
            der Sitz des Betreibers (siehe Impressum).
          </p>
          <p>
            (3) Sollten einzelne Bestimmungen dieser AGB unwirksam sein, bleibt die Wirksamkeit der
            übrigen Bestimmungen davon unberührt.
          </p>
        </div>
      </div>

      <div className="card p-4 bg-gray-50 dark:bg-gray-800/50">
        <p className="text-xs text-gray-500 dark:text-gray-400">
          Diese AGB basieren auf einer Vorlage für ParkHub-Betreiber und stellen keinen Rechtsrat dar.
          Für die Richtigkeit und Vollständigkeit ist der Betreiber dieser ParkHub-Instanz verantwortlich.
          Individuelle rechtliche Beratung wird empfohlen.
        </p>
      </div>
    </motion.div>
  );
}
