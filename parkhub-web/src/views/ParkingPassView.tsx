import { useEffect, useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Ticket, QrCode, Clock, MapPin, Question, CalendarBlank } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';

interface ParkingPass {
  id: string;
  booking_id: string;
  user_id: string;
  user_name: string;
  lot_name: string;
  slot_number: string;
  valid_from: string;
  valid_until: string;
  verification_code: string;
  qr_data: string;
  status: 'active' | 'expired' | 'revoked' | 'used';
  created_at: string;
}

const statusColors: Record<string, string> = {
  active: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400',
  expired: 'bg-surface-100 text-surface-500 dark:bg-surface-800 dark:text-surface-400',
  revoked: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400',
  used: 'bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400',
};

function formatDate(iso: string) {
  return new Date(iso).toLocaleString(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  });
}

export function ParkingPassPage() {
  const { t } = useTranslation();
  const [passes, setPasses] = useState<ParkingPass[]>([]);
  const [loading, setLoading] = useState(true);
  const [selectedPass, setSelectedPass] = useState<ParkingPass | null>(null);
  const [showHelp, setShowHelp] = useState(false);

  const loadPasses = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch('/api/v1/me/passes').then(r => r.json());
      if (res.success) setPasses(res.data || []);
    } catch { /* ignore */ }
    setLoading(false);
  }, []);

  useEffect(() => { loadPasses(); }, [loadPasses]);

  return (
    <div className="space-y-6 p-4 max-w-4xl mx-auto">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-surface-50">
            {t('parkingPass.title')}
          </h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">
            {t('parkingPass.subtitle')}
          </p>
        </div>
        <button
          onClick={() => setShowHelp(!showHelp)}
          className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800"
          aria-label={t('parkingPass.helpLabel')}
        >
          <Question size={24} />
        </button>
      </div>

      {/* Help tooltip */}
      {showHelp && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          className="p-4 rounded-xl bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800"
        >
          <p className="text-sm text-blue-700 dark:text-blue-300">
            {t('parkingPass.help')}
          </p>
        </motion.div>
      )}

      {loading ? (
        <div className="flex justify-center py-12">
          <div className="animate-spin rounded-full h-8 w-8 border-2 border-primary-500 border-t-transparent" />
        </div>
      ) : selectedPass ? (
        /* Full-screen pass display */
        <motion.div
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          className="bg-gradient-to-b from-primary-500 to-primary-700 rounded-2xl p-6 text-white shadow-xl max-w-sm mx-auto"
        >
          <div className="text-center space-y-4">
            <div className="flex items-center justify-center gap-2 text-primary-100">
              <Ticket size={20} />
              <span className="text-sm font-medium uppercase tracking-wide">
                {t('parkingPass.digitalPass')}
              </span>
            </div>

            <h2 className="text-xl font-bold">{selectedPass.user_name}</h2>

            {/* QR Code */}
            {selectedPass.qr_data && (
              <div className="bg-white rounded-xl p-4 inline-block mx-auto">
                <img
                  src={selectedPass.qr_data}
                  alt="QR Code"
                  className="w-48 h-48"
                />
              </div>
            )}

            {/* Pass details */}
            <div className="space-y-2 text-primary-100">
              <div className="flex items-center justify-center gap-2">
                <MapPin size={16} />
                <span>{selectedPass.lot_name} — {t('parkingPass.slot')} {selectedPass.slot_number}</span>
              </div>
              <div className="flex items-center justify-center gap-2">
                <CalendarBlank size={16} />
                <span>{formatDate(selectedPass.valid_from)}</span>
              </div>
              <div className="flex items-center justify-center gap-2">
                <Clock size={16} />
                <span>{t('parkingPass.validUntil')} {formatDate(selectedPass.valid_until)}</span>
              </div>
            </div>

            <span className={`inline-block px-3 py-1 rounded-full text-xs font-medium ${statusColors[selectedPass.status]}`}>
              {t(`parkingPass.status.${selectedPass.status}`)}
            </span>

            <div className="text-xs text-primary-200 font-mono mt-2">
              {selectedPass.verification_code}
            </div>

            <button
              onClick={() => setSelectedPass(null)}
              className="mt-4 px-4 py-2 bg-white/20 rounded-lg hover:bg-white/30 text-sm"
            >
              {t('common.close')}
            </button>
          </div>
        </motion.div>
      ) : (
        /* Pass list */
        <>
          {passes.length > 0 ? (
            <div className="space-y-3">
              {passes.map(pass => (
                <motion.div
                  key={pass.id}
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  onClick={() => setSelectedPass(pass)}
                  className="p-4 rounded-xl bg-white dark:bg-surface-800 border border-surface-200 dark:border-surface-700 shadow-sm cursor-pointer hover:shadow-md transition-shadow"
                >
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-3">
                      <div className="w-10 h-10 rounded-lg bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center">
                        <QrCode size={20} className="text-primary-600 dark:text-primary-400" />
                      </div>
                      <div>
                        <p className="font-medium text-surface-900 dark:text-surface-100">
                          {pass.lot_name}
                        </p>
                        <p className="text-xs text-surface-500">
                          {t('parkingPass.slot')} {pass.slot_number} — {formatDate(pass.valid_from)}
                        </p>
                      </div>
                    </div>
                    <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${statusColors[pass.status]}`}>
                      {t(`parkingPass.status.${pass.status}`)}
                    </span>
                  </div>
                </motion.div>
              ))}
            </div>
          ) : (
            <div className="text-center py-12 text-surface-400">
              <Ticket size={48} className="mx-auto mb-3 opacity-40" />
              <p>{t('parkingPass.empty')}</p>
            </div>
          )}
        </>
      )}
    </div>
  );
}

export default ParkingPassPage;
