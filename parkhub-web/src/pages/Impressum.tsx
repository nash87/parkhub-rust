import { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { Buildings, Envelope, Phone, Article, Warning } from '@phosphor-icons/react';

interface ImpressumData {
  provider_name: string;
  provider_legal_form: string;
  street: string;
  zip_city: string;
  country: string;
  email: string;
  phone: string;
  register_court: string;
  register_number: string;
  vat_id: string;
  responsible_person: string;
  custom_text: string;
}

export function ImpressumPage() {
  const [data, setData] = useState<ImpressumData | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch('/api/v1/legal/impressum')
      .then(r => r.json())
      .then(d => setData(d))
      .catch(() => setData(null))
      .finally(() => setLoading(false));
  }, []);

  const hasContent = data && (data.provider_name || data.street || data.email);

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="max-w-3xl mx-auto space-y-8 py-8 px-4"
    >
      <div className="flex items-center gap-3">
        <Article weight="fill" className="w-8 h-8 text-primary-600" />
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Impressum
        </h1>
      </div>

      <p className="text-sm text-gray-500 dark:text-gray-400">
        Angaben gemäß § 5 DDG (Digitale-Dienste-Gesetz)
      </p>

      {loading && (
        <div className="card p-6 animate-pulse">
          <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-1/2 mb-3" />
          <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-2/3" />
        </div>
      )}

      {!loading && !hasContent && (
        <div className="card p-6 flex items-start gap-3 border-l-4 border-amber-500">
          <Warning weight="fill" className="w-5 h-5 text-amber-500 mt-0.5 shrink-0" />
          <div>
            <p className="text-sm font-medium text-gray-900 dark:text-white">
              Impressum noch nicht konfiguriert
            </p>
            <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
              Als Administrator können Sie die Impressum-Daten in den Einstellungen hinterlegen.
            </p>
          </div>
        </div>
      )}

      {!loading && hasContent && data && (
        <>
          {/* Provider */}
          <div className="card p-6">
            <div className="flex items-center gap-3 mb-4">
              <Buildings weight="fill" className="w-5 h-5 text-primary-600" />
              <h2 className="text-lg font-semibold text-gray-900 dark:text-white">Anbieter</h2>
            </div>
            <div className="space-y-2 text-sm text-gray-700 dark:text-gray-300">
              {data.provider_name && (
                <p className="font-medium text-gray-900 dark:text-white">
                  {data.provider_name}
                  {data.provider_legal_form && (
                    <span className="font-normal text-gray-500"> ({data.provider_legal_form})</span>
                  )}
                </p>
              )}
              {data.street && <p>{data.street}</p>}
              {data.zip_city && <p>{data.zip_city}</p>}
              {data.country && <p>{data.country}</p>}
            </div>
          </div>

          {/* Contact */}
          {(data.email || data.phone) && (
            <div className="card p-6">
              <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">Kontakt</h2>
              <div className="space-y-2 text-sm">
                {data.email && (
                  <div className="flex items-center gap-2 text-gray-700 dark:text-gray-300">
                    <Envelope weight="fill" className="w-4 h-4 text-primary-600 shrink-0" />
                    <a href={`mailto:${data.email}`} className="hover:text-primary-600 hover:underline">
                      {data.email}
                    </a>
                  </div>
                )}
                {data.phone && (
                  <div className="flex items-center gap-2 text-gray-700 dark:text-gray-300">
                    <Phone weight="fill" className="w-4 h-4 text-primary-600 shrink-0" />
                    <a href={`tel:${data.phone}`} className="hover:text-primary-600 hover:underline">
                      {data.phone}
                    </a>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Register & VAT */}
          {(data.register_court || data.register_number || data.vat_id) && (
            <div className="card p-6">
              <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
                Handelsregister &amp; Steuern
              </h2>
              <div className="space-y-2 text-sm text-gray-700 dark:text-gray-300">
                {data.register_court && (
                  <p>
                    <span className="font-medium text-gray-900 dark:text-white">Registergericht:</span>{' '}
                    {data.register_court}
                  </p>
                )}
                {data.register_number && (
                  <p>
                    <span className="font-medium text-gray-900 dark:text-white">Registernummer:</span>{' '}
                    {data.register_number}
                  </p>
                )}
                {data.vat_id && (
                  <p>
                    <span className="font-medium text-gray-900 dark:text-white">USt-IdNr.:</span>{' '}
                    {data.vat_id}
                  </p>
                )}
              </div>
            </div>
          )}

          {/* Responsible person */}
          {data.responsible_person && (
            <div className="card p-6">
              <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-2">
                Verantwortliche Person
              </h2>
              <p className="text-sm text-gray-700 dark:text-gray-300">{data.responsible_person}</p>
            </div>
          )}

          {/* Custom text */}
          {data.custom_text && (
            <div className="card p-6">
              <p className="text-sm text-gray-700 dark:text-gray-300 whitespace-pre-line leading-relaxed">
                {data.custom_text}
              </p>
            </div>
          )}
        </>
      )}

      <div className="card p-4 bg-gray-50 dark:bg-gray-800/50">
        <p className="text-xs text-gray-500 dark:text-gray-400">
          Angaben gemäß § 5 DDG. Für die Richtigkeit und Vollständigkeit ist der Betreiber dieser
          ParkHub-Instanz verantwortlich.
        </p>
      </div>
    </motion.div>
  );
}
