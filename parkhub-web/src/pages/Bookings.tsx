import { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  CalendarBlank,
  Clock,
  Car,
  X,
  SpinnerGap,
  CheckCircle,
  XCircle,
  ArrowClockwise,
  Warning,
} from '@phosphor-icons/react';
import { api, Booking } from '../api/client';
import toast from 'react-hot-toast';
import { format, formatDistanceToNow } from 'date-fns';
import { de } from 'date-fns/locale';

export function BookingsPage() {
  const [bookings, setBookings] = useState<Booking[]>([]);
  const [loading, setLoading] = useState(true);
  const [cancelling, setCancelling] = useState<string | null>(null);

  useEffect(() => {
    loadBookings();
  }, []);

  async function loadBookings() {
    try {
      const res = await api.getBookings();
      if (res.success && res.data) {
        setBookings(res.data);
      }
    } finally {
      setLoading(false);
    }
  }

  async function handleCancel(id: string) {
    setCancelling(id);
    const res = await api.cancelBooking(id);
    if (res.success) {
      setBookings(bookings.map(b => 
        b.id === id ? { ...b, status: 'cancelled' as const } : b
      ));
      toast.success('Buchung storniert');
    } else {
      toast.error('Stornierung fehlgeschlagen');
    }
    setCancelling(null);
  }

  const activeBookings = bookings.filter(b => b.status === 'active');
  const pastBookings = bookings.filter(b => b.status !== 'active');

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" />
      </div>
    );
  }

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
            Meine Buchungen
          </h1>
          <p className="text-gray-500 dark:text-gray-400 mt-1">
            Übersicht Ihrer Parkplatz-Buchungen
          </p>
        </div>
        <button onClick={loadBookings} className="btn btn-secondary">
          <ArrowClockwise weight="bold" className="w-4 h-4" />
          Aktualisieren
        </button>
      </div>

      {/* Active Bookings */}
      <div>
        <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
          <Clock weight="fill" className="w-5 h-5 text-primary-600" />
          Aktive Buchungen
          <span className="badge badge-info">{activeBookings.length}</span>
        </h2>
        
        {activeBookings.length === 0 ? (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="card p-12 text-center"
          >
            <CalendarBlank weight="light" className="w-16 h-16 text-gray-300 dark:text-gray-700 mx-auto mb-4" />
            <p className="text-gray-500 dark:text-gray-400">
              Keine aktiven Buchungen
            </p>
          </motion.div>
        ) : (
          <div className="space-y-4">
            <AnimatePresence>
              {activeBookings.map((booking, index) => {
                const isExpiringSoon = new Date(booking.end_time).getTime() - Date.now() < 30 * 60 * 1000;
                
                return (
                  <motion.div
                    key={booking.id}
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0, x: -100 }}
                    transition={{ delay: index * 0.05 }}
                    className="card p-6"
                  >
                    <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
                      <div className="flex items-center gap-4">
                        <div className={`w-16 h-16 rounded-2xl flex items-center justify-center ${
                          isExpiringSoon
                            ? 'bg-amber-100 dark:bg-amber-900/30'
                            : 'bg-primary-100 dark:bg-primary-900/30'
                        }`}>
                          <span className={`text-xl font-bold ${
                            isExpiringSoon
                              ? 'text-amber-600 dark:text-amber-400'
                              : 'text-primary-600 dark:text-primary-400'
                          }`}>
                            {booking.slot_number}
                          </span>
                        </div>
                        <div>
                          <p className="font-semibold text-gray-900 dark:text-white">
                            {booking.lot_name}
                          </p>
                          <div className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400">
                            <Car weight="regular" className="w-4 h-4" />
                            {booking.vehicle_plate || 'Kein Kennzeichen'}
                          </div>
                        </div>
                      </div>
                      
                      <div className="flex items-center gap-4">
                        <div className="text-right">
                          <div className="flex items-center gap-2">
                            {isExpiringSoon && (
                              <Warning weight="fill" className="w-4 h-4 text-amber-500" />
                            )}
                            <p className="font-medium text-gray-900 dark:text-white">
                              Bis {format(new Date(booking.end_time), 'HH:mm')} Uhr
                            </p>
                          </div>
                          <p className={`text-sm ${
                            isExpiringSoon
                              ? 'text-amber-600 dark:text-amber-400 font-medium'
                              : 'text-gray-500 dark:text-gray-400'
                          }`}>
                            {formatDistanceToNow(new Date(booking.end_time), { addSuffix: true, locale: de })}
                          </p>
                        </div>
                        
                        <button
                          onClick={() => handleCancel(booking.id)}
                          disabled={cancelling === booking.id}
                          className="btn btn-ghost text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
                        >
                          {cancelling === booking.id ? (
                            <SpinnerGap weight="bold" className="w-5 h-5 animate-spin" />
                          ) : (
                            <X weight="bold" className="w-5 h-5" />
                          )}
                        </button>
                      </div>
                    </div>
                  </motion.div>
                );
              })}
            </AnimatePresence>
          </div>
        )}
      </div>

      {/* Past Bookings */}
      {pastBookings.length > 0 && (
        <div>
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
            <CalendarBlank weight="regular" className="w-5 h-5 text-gray-400" />
            Vergangene Buchungen
          </h2>
          
          <div className="space-y-3">
            {pastBookings.slice(0, 10).map((booking, index) => (
              <motion.div
                key={booking.id}
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                transition={{ delay: index * 0.05 }}
                className="card p-4 bg-gray-50 dark:bg-gray-900/50"
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-4">
                    <div className="w-12 h-12 bg-gray-200 dark:bg-gray-800 rounded-xl flex items-center justify-center">
                      <span className="text-sm font-bold text-gray-500 dark:text-gray-400">
                        {booking.slot_number}
                      </span>
                    </div>
                    <div>
                      <p className="font-medium text-gray-700 dark:text-gray-300">
                        {booking.lot_name}
                      </p>
                      <p className="text-sm text-gray-500 dark:text-gray-400">
                        {format(new Date(booking.start_time), 'd. MMM yyyy', { locale: de })}
                      </p>
                    </div>
                  </div>
                  
                  <div className={`badge ${
                    booking.status === 'completed' ? 'badge-success' : 'badge-gray'
                  }`}>
                    {booking.status === 'completed' ? (
                      <>
                        <CheckCircle weight="fill" className="w-3 h-3" />
                        Abgeschlossen
                      </>
                    ) : (
                      <>
                        <XCircle weight="fill" className="w-3 h-3" />
                        Storniert
                      </>
                    )}
                  </div>
                </div>
              </motion.div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
