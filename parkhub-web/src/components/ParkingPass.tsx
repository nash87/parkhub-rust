import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { X, DownloadSimple, Printer } from '@phosphor-icons/react';
import type { Booking } from '../api/client';
import { getInMemoryToken } from '../api/client';
import { format } from 'date-fns';

interface ParkingPassProps {
  booking: Booking;
  onClose: () => void;
}

export function ParkingPass({ booking, onClose }: ParkingPassProps) {
  const { t } = useTranslation();
  const [imgSrc, setImgSrc] = useState<string | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    const token = getInMemoryToken();
    const BASE_URL = import.meta.env?.VITE_API_URL || '';
    fetch(`${BASE_URL}/api/v1/bookings/${booking.id}/qr`, {
      credentials: 'include',
      headers: {
        'X-Requested-With': 'XMLHttpRequest',
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
      },
    })
      .then(res => {
        if (!res.ok) throw new Error('Failed to load pass');
        return res.blob();
      })
      .then(blob => setImgSrc(URL.createObjectURL(blob)))
      .catch(() => setError(true));
    return () => {
      if (imgSrc) URL.revokeObjectURL(imgSrc);
    };
  }, [booking.id]);

  function handleDownload() {
    if (!imgSrc) return;
    const a = document.createElement('a');
    a.href = imgSrc;
    a.download = `parkhub-pass-${booking.id}.png`;
    a.click();
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-label={t('pass.title')}
    >
      <div
        className="print-pass bg-white dark:bg-surface-900 rounded-2xl shadow-xl max-w-sm w-full mx-4 p-6"
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-bold text-surface-900 dark:text-white">
            {t('pass.title')}
          </h2>
          <button
            onClick={onClose}
            className="p-1 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-800 print:hidden"
            aria-label={t('common.close')}
          >
            <X weight="bold" className="w-5 h-5 text-surface-500" />
          </button>
        </div>

        <div className="flex justify-center mb-4">
          {error ? (
            <p className="text-sm text-red-500">{t('pass.loadError')}</p>
          ) : imgSrc ? (
            <img
              src={imgSrc}
              alt={t('pass.qrAlt')}
              className="w-48 h-48 rounded-lg"
            />
          ) : (
            <div className="w-48 h-48 rounded-lg bg-surface-100 dark:bg-surface-800 animate-pulse" />
          )}
        </div>

        <div className="space-y-2 text-sm text-surface-700 dark:text-surface-300 mb-4">
          <div className="flex justify-between">
            <span className="text-surface-500">{t('pass.lot')}</span>
            <span className="font-medium">{booking.lot_name}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-surface-500">{t('pass.slot')}</span>
            <span className="font-medium">{booking.slot_number}</span>
          </div>
          <div className="flex justify-between">
            <span className="text-surface-500">{t('pass.time')}</span>
            <span className="font-medium">
              {format(new Date(booking.start_time), 'HH:mm')} —{' '}
              {format(new Date(booking.end_time), 'HH:mm')}
            </span>
          </div>
          {booking.vehicle_plate && (
            <div className="flex justify-between">
              <span className="text-surface-500">{t('pass.vehicle')}</span>
              <span className="font-medium">{booking.vehicle_plate}</span>
            </div>
          )}
        </div>

        <div className="flex gap-2 print:hidden">
          <button
            onClick={handleDownload}
            disabled={!imgSrc}
            className="btn btn-primary flex-1 flex items-center justify-center gap-2"
          >
            <DownloadSimple weight="bold" className="w-4 h-4" />{' '}
            {t('pass.download')}
          </button>
          <button
            onClick={() => window.print()}
            className="btn btn-secondary flex items-center justify-center gap-2 px-4"
            title="Print booking confirmation"
          >
            <Printer weight="bold" className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}
