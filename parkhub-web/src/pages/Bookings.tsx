import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
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
  Receipt,
  CalendarPlus,
  MagnifyingGlass,
  Funnel,
} from '@phosphor-icons/react';
import { api, Booking } from '../api/client';
import toast from 'react-hot-toast';
import { format, formatDistanceToNow } from 'date-fns';
import { de } from 'date-fns/locale';

/**
 * Open a booking invoice in a new tab.
 * The invoice endpoint requires Bearer auth, so we fetch it with the token
 * and open it as a Blob URL instead of a direct link.
 */
async function openInvoice(bookingId: string) {
  const token = api.getToken();
  if (!token) {
    toast.error('Nicht angemeldet');
    return;
  }
  try {
    const response = await fetch(`/api/v1/bookings/${bookingId}/invoice`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    if (!response.ok) {
      toast.error('Rechnung konnte nicht geladen werden');
      return;
    }
    const html = await response.text();
    const blob = new Blob([html], { type: 'text/html;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const win = window.open(url, '_blank', 'noopener,noreferrer');
    // Revoke the URL after a short delay to free memory
    setTimeout(() => URL.revokeObjectURL(url), 60_000);
    if (!win) {
      toast.error('Pop-up blockiert — bitte erlauben Sie Pop-ups für diese Seite');
    }
  } catch {
    toast.error('Fehler beim Laden der Rechnung');
  }
}

export function BookingsPage() {
  const [bookings, setBookings] = useState<Booking[]>([]);
  const [loading, setLoading] = useState(true);
  const [cancelling, setCancelling] = useState<string | null>(null);

  // Filter state
  const [filterStatus, setFilterStatus] = useState<string>('');
  const [filterDateFrom, setFilterDateFrom] = useState<string>('');
  const [filterDateTo, setFilterDateTo] = useState<string>('');
  const [filterSearch, setFilterSearch] = useState<string>('');

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

  // Client-side filtering
  const filteredBookings = bookings.filter((b) => {
    if (filterStatus && b.status !== filterStatus) return false;
    if (filterDateFrom && b.start_time < filterDateFrom) return false;
    if (filterDateTo && b.start_time > filterDateTo + 'T23:59:59') return false;
    if (filterSearch) {
      const q = filterSearch.toLowerCase();
      if (
        !b.slot_number.toLowerCase().includes(q) &&
        !b.lot_name.toLowerCase().includes(q)
      ) return false;
    }
    return true;
  });

  const hasFilters = filterStatus || filterDateFrom || filterDateTo || filterSearch;

  function resetFilters() {
    setFilterStatus('');
    setFilterDateFrom('');
    setFilterDateTo('');
    setFilterSearch('');
  }

  const activeBookings = filteredBookings.filter(b => b.status === 'active');
  const pastBookings = filteredBookings.filter(b => b.status !== 'active');

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64" role="status" aria-label="Buchungen werden geladen">
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" aria-hidden="true" />
        <span className="sr-only">Buchungen werden geladen…</span>
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

      {/* Filter Bar */}
      <div className="card p-4">
        <div className="flex items-center gap-2 mb-3">
          <Funnel weight="fill" className="w-4 h-4 text-gray-400" aria-hidden="true" />
          <span className="text-sm font-medium text-gray-700 dark:text-gray-300">Filter</span>
          {hasFilters && (
            <button
              onClick={resetFilters}
              className="ml-auto text-xs text-primary-600 dark:text-primary-400 hover:underline flex items-center gap-1"
              aria-label="Filter zurücksetzen"
            >
              <X weight="bold" className="w-3 h-3" aria-hidden="true" />
              Filter zurücksetzen
            </button>
          )}
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3">
          {/* Status filter */}
          <div>
            <label htmlFor="booking-filter-status" className="label text-xs">Status</label>
            <select
              id="booking-filter-status"
              value={filterStatus}
              onChange={(e) => setFilterStatus(e.target.value)}
              className="input text-sm py-2"
            >
              <option value="">Alle</option>
              <option value="active">Aktiv</option>
              <option value="completed">Abgeschlossen</option>
              <option value="cancelled">Storniert</option>
            </select>
          </div>

          {/* Date From */}
          <div>
            <label htmlFor="booking-filter-from" className="label text-xs">Von Datum</label>
            <input
              id="booking-filter-from"
              type="date"
              value={filterDateFrom}
              onChange={(e) => setFilterDateFrom(e.target.value)}
              className="input text-sm py-2"
            />
          </div>

          {/* Date To */}
          <div>
            <label htmlFor="booking-filter-to" className="label text-xs">Bis Datum</label>
            <input
              id="booking-filter-to"
              type="date"
              value={filterDateTo}
              onChange={(e) => setFilterDateTo(e.target.value)}
              className="input text-sm py-2"
            />
          </div>

          {/* Text search */}
          <div>
            <label htmlFor="booking-filter-search" className="label text-xs">Suche</label>
            <div className="relative">
              <MagnifyingGlass
                weight="bold"
                className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400"
                aria-hidden="true"
              />
              <input
                id="booking-filter-search"
                type="search"
                value={filterSearch}
                onChange={(e) => setFilterSearch(e.target.value)}
                placeholder="Stellplatz oder Parkplatz…"
                className="input text-sm py-2 pl-9"
              />
            </div>
          </div>
        </div>

        {/* Result count */}
        {hasFilters && (
          <p className="mt-3 text-xs text-gray-500 dark:text-gray-400">
            {filteredBookings.length} von {bookings.length} Buchungen angezeigt
          </p>
        )}
      </div>

      {/* Active Bookings */}
      <section aria-labelledby="active-bookings-heading">
        <h2 id="active-bookings-heading" className="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
          <Clock weight="fill" className="w-5 h-5 text-primary-600" aria-hidden="true" />
          Aktive Buchungen
          <span className="badge badge-info" aria-label={`${activeBookings.length} aktive Buchungen`}>{activeBookings.length}</span>
        </h2>

        {activeBookings.length === 0 ? (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="card p-12 text-center"
          >
            <CalendarBlank weight="light" className="w-16 h-16 text-gray-300 dark:text-gray-700 mx-auto mb-4" aria-hidden="true" />
            <p className="text-gray-700 dark:text-gray-300 font-medium mb-1">
              Noch keine Buchungen
            </p>
            <p className="text-gray-500 dark:text-gray-400 text-sm mb-6">
              Buchen Sie jetzt einen Parkplatz, um loszulegen.
            </p>
            <Link to="/book" className="btn btn-primary inline-flex">
              <CalendarPlus weight="bold" className="w-5 h-5" aria-hidden="true" />
              Jetzt Parkplatz buchen
            </Link>
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
                          aria-label={`Buchung für ${booking.lot_name} Platz ${booking.slot_number} stornieren`}
                          aria-busy={cancelling === booking.id}
                          className="btn btn-ghost text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
                        >
                          {cancelling === booking.id ? (
                            <SpinnerGap weight="bold" className="w-5 h-5 animate-spin" aria-hidden="true" />
                          ) : (
                            <X weight="bold" className="w-5 h-5" aria-hidden="true" />
                          )}
                          <span className="sr-only">
                            {cancelling === booking.id ? 'Stornierung läuft…' : 'Stornieren'}
                          </span>
                        </button>
                      </div>
                    </div>
                  </motion.div>
                );
              })}
            </AnimatePresence>
          </div>
        )}
      </section>

      {/* Past Bookings */}
      {pastBookings.length > 0 && (
        <section aria-labelledby="past-bookings-heading">
          <h2 id="past-bookings-heading" className="text-lg font-semibold text-gray-900 dark:text-white mb-4 flex items-center gap-2">
            <CalendarBlank weight="regular" className="w-5 h-5 text-gray-400" aria-hidden="true" />
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
                    <div className="w-12 h-12 bg-gray-200 dark:bg-gray-800 rounded-xl flex items-center justify-center" aria-hidden="true">
                      <span className="text-sm font-bold text-gray-500 dark:text-gray-400">
                        {booking.slot_number}
                      </span>
                    </div>
                    <div>
                      <p className="font-medium text-gray-700 dark:text-gray-300">
                        {booking.lot_name} — Platz {booking.slot_number}
                      </p>
                      <p className="text-sm text-gray-500 dark:text-gray-400">
                        {format(new Date(booking.start_time), 'd. MMM yyyy', { locale: de })}
                      </p>
                    </div>
                  </div>

                  <div className="flex items-center gap-2">
                    {booking.status === 'completed' && (
                      <button
                        onClick={() => openInvoice(booking.id)}
                        aria-label={`Rechnung für Buchung ${booking.slot_number} anzeigen`}
                        className="btn btn-sm btn-ghost text-gray-500 dark:text-gray-400 hover:text-primary-600 dark:hover:text-primary-400"
                        title="Rechnung anzeigen"
                        type="button"
                      >
                        <Receipt weight="regular" className="w-4 h-4" aria-hidden="true" />
                        <span className="hidden sm:inline">Rechnung</span>
                      </button>
                    )}
                    <div className={`badge ${
                      booking.status === 'completed' ? 'badge-success' : 'badge-gray'
                    }`}>
                      {booking.status === 'completed' ? (
                        <>
                          <CheckCircle weight="fill" className="w-3 h-3" aria-hidden="true" />
                          Abgeschlossen
                        </>
                      ) : (
                        <>
                          <XCircle weight="fill" className="w-3 h-3" aria-hidden="true" />
                          Storniert
                        </>
                      )}
                    </div>
                  </div>
                </div>
              </motion.div>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
