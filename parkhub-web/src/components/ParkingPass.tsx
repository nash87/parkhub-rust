import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  X, DownloadSimple, Printer, NavigationArrow, ClockCounterClockwise,
  XCircle, Ticket, MapTrifold, ClockClockwise, UserCircle,
} from '@phosphor-icons/react';
import type { Booking } from '../api/client';
import { format } from 'date-fns';

interface ParkingPassProps {
  booking: Booking;
  onClose: () => void;
}

type BottomTab = 'passes' | 'map' | 'history' | 'profile';

export function ParkingPass({ booking, onClose }: ParkingPassProps) {
  const { t } = useTranslation();
  const [imgSrc, setImgSrc] = useState<string | null>(null);
  const [error, setError] = useState(false);
  const [activeTab, setActiveTab] = useState<BottomTab>('passes');

  useEffect(() => {
    const token = localStorage.getItem('parkhub_token');
    const BASE_URL = import.meta.env?.VITE_API_URL || '';
    fetch(`${BASE_URL}/api/v1/bookings/${booking.id}/qr`, {
      headers: { ...(token ? { Authorization: `Bearer ${token}` } : {}) },
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

  const isActive = booking.status === 'active';

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm"
      onClick={onClose}
      role="dialog"
      aria-modal="true"
      aria-label={t('pass.title')}
    >
      <div
        className="print-pass bg-surface-950 rounded-3xl shadow-2xl max-w-sm w-full mx-4 overflow-hidden flex flex-col"
        style={{ maxHeight: '90dvh' }}
        onClick={e => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 pt-5 pb-3">
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg hover:bg-surface-800 transition-colors print:hidden"
            aria-label={t('common.close')}
          >
            <X weight="bold" className="w-5 h-5 text-surface-400" />
          </button>
          <h2 className="text-sm font-bold text-white uppercase tracking-wider">
            {t('pass.title')}
          </h2>
          <button
            onClick={handleDownload}
            disabled={!imgSrc}
            className="p-1.5 rounded-lg hover:bg-surface-800 transition-colors print:hidden"
            aria-label={t('pass.download')}
          >
            <DownloadSimple weight="bold" className="w-5 h-5 text-surface-400" />
          </button>
        </div>

        {/* Status badge */}
        <div className="flex justify-center mb-3">
          <span className="flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold uppercase tracking-wider" style={{
            color: isActive ? '#34d399' : '#fbbf24',
            background: isActive ? 'rgba(52, 211, 153, 0.1)' : 'rgba(251, 191, 36, 0.1)',
          }}>
            <span className={`w-2 h-2 rounded-full ${isActive ? 'bg-emerald-400 pulse-dot' : 'bg-amber-400'}`} />
            {isActive
              ? t('pass.activeSession', 'Active Session')
              : t('pass.sessionStatus', booking.status)}
          </span>
        </div>

        {/* QR Code */}
        <div className="flex justify-center px-6 mb-4">
          <div className="w-44 h-44 rounded-2xl overflow-hidden border-2 border-primary-700/30 bg-surface-900 flex items-center justify-center"
            style={{ boxShadow: '0 0 30px rgba(13, 148, 136, 0.15)' }}
          >
            {error ? (
              <p className="text-sm text-red-400 text-center px-4">{t('pass.loadError')}</p>
            ) : imgSrc ? (
              <img
                src={imgSrc}
                alt={t('pass.qrAlt')}
                className="w-40 h-40 rounded-xl"
              />
            ) : (
              <div className="w-40 h-40 rounded-xl bg-surface-800 animate-pulse" />
            )}
          </div>
        </div>

        {/* Assigned Slot — large display */}
        <div className="text-center mb-5">
          <p className="text-xs font-medium uppercase tracking-wider text-surface-500 mb-1">
            {t('pass.assignedSlot', 'Assigned Slot')}
          </p>
          <p className="text-5xl font-black text-white tracking-tight" style={{ letterSpacing: '-0.04em', fontVariantNumeric: 'tabular-nums' }}>
            {booking.slot_number}
          </p>
        </div>

        {/* Details grid */}
        <div className="grid grid-cols-2 gap-x-6 gap-y-3 px-6 mb-6">
          <div>
            <p className="text-[10px] font-medium uppercase tracking-wider text-surface-500 mb-0.5">
              {t('pass.location', 'Location')}
            </p>
            <p className="text-sm font-semibold text-white">{booking.lot_name}</p>
          </div>
          <div>
            <p className="text-[10px] font-medium uppercase tracking-wider text-surface-500 mb-0.5">
              {t('pass.vehicle')}
            </p>
            <p className="text-sm font-semibold text-white">{booking.vehicle_plate || '—'}</p>
          </div>
          <div>
            <p className="text-[10px] font-medium uppercase tracking-wider text-surface-500 mb-0.5">
              {t('pass.validFrom', 'Valid From')}
            </p>
            <p className="text-sm font-semibold text-white tabular-nums">
              {format(new Date(booking.start_time), 'hh:mm a')}
            </p>
          </div>
          <div>
            <p className="text-[10px] font-medium uppercase tracking-wider text-surface-500 mb-0.5">
              {t('pass.expires', 'Expires')}
            </p>
            <p className="text-sm font-semibold text-white tabular-nums">
              {format(new Date(booking.end_time), 'hh:mm a')}
            </p>
          </div>
        </div>

        {/* Action buttons */}
        <div className="px-6 mb-4 space-y-2 print:hidden">
          <button className="w-full btn text-white font-semibold py-3 rounded-xl"
            style={{ background: 'var(--color-primary-600)' }}
          >
            <NavigationArrow weight="bold" className="w-4 h-4" />
            {t('pass.navigateToSlot', 'Navigate to Slot')}
          </button>
          <div className="flex gap-2">
            <button className="flex-1 btn btn-secondary py-2.5 rounded-xl border border-surface-700 bg-surface-800 text-surface-300 hover:bg-surface-700">
              <ClockCounterClockwise weight="bold" className="w-4 h-4" />
              {t('pass.extend', 'Extend')}
            </button>
            <button className="flex-1 btn btn-secondary py-2.5 rounded-xl border border-surface-700 bg-surface-800 text-surface-300 hover:bg-surface-700">
              <XCircle weight="bold" className="w-4 h-4" />
              {t('pass.cancel', 'Cancel')}
            </button>
          </div>
        </div>

        {/* Print button (hidden in normal view, shown in quick actions) */}
        <div className="px-6 mb-4 print:hidden hidden sm:block">
          <button
            onClick={() => window.print()}
            className="w-full btn btn-ghost text-surface-400 hover:text-white py-2 text-xs"
            title="Print booking confirmation"
          >
            <Printer weight="bold" className="w-3.5 h-3.5" />
            {t('pass.print', 'Print Pass')}
          </button>
        </div>

        {/* Bottom navigation tabs */}
        <div className="border-t border-surface-800 px-4 py-2 flex justify-around print:hidden mt-auto">
          {([
            { key: 'passes' as BottomTab, icon: Ticket, label: t('pass.tabPasses', 'Passes') },
            { key: 'map' as BottomTab, icon: MapTrifold, label: t('pass.tabMap', 'Map') },
            { key: 'history' as BottomTab, icon: ClockClockwise, label: t('pass.tabHistory', 'History') },
            { key: 'profile' as BottomTab, icon: UserCircle, label: t('pass.tabProfile', 'Profile') },
          ]).map(tab => (
            <button
              key={tab.key}
              onClick={() => setActiveTab(tab.key)}
              className={`flex flex-col items-center gap-0.5 px-3 py-1.5 rounded-lg transition-colors ${
                activeTab === tab.key
                  ? 'text-primary-400'
                  : 'text-surface-500 hover:text-surface-300'
              }`}
            >
              <tab.icon weight={activeTab === tab.key ? 'fill' : 'regular'} className="w-5 h-5" />
              <span className="text-[10px] font-medium">{tab.label}</span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
