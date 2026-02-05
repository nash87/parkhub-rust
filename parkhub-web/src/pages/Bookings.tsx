import { useEffect, useState } from 'react';
import { api, Booking } from '../api/client';

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
    if (!confirm('Buchung wirklich stornieren?')) return;
    
    setCancelling(id);
    const res = await api.cancelBooking(id);
    if (res.success) {
      setBookings(bookings.map(b => 
        b.id === id ? { ...b, status: 'cancelled' as const } : b
      ));
    }
    setCancelling(null);
  }

  const activeBookings = bookings.filter(b => b.status === 'active');
  const pastBookings = bookings.filter(b => b.status !== 'active');

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-2xl font-bold text-gray-900">Meine Buchungen</h1>
        <p className="text-gray-600 mt-1">Übersicht Ihrer Parkplatz-Buchungen</p>
      </div>

      {/* Active Bookings */}
      <div>
        <h2 className="text-lg font-semibold text-gray-900 mb-4">
          Aktive Buchungen ({activeBookings.length})
        </h2>
        
        {activeBookings.length === 0 ? (
          <div className="card text-center py-12 text-gray-500">
            <span className="text-4xl mb-4 block">🅿️</span>
            <p>Keine aktiven Buchungen</p>
          </div>
        ) : (
          <div className="space-y-4">
            {activeBookings.map((booking) => (
              <div key={booking.id} className="card">
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-4">
                    <div className="w-16 h-16 bg-primary-100 rounded-xl flex items-center justify-center">
                      <span className="text-2xl font-bold text-primary-600">
                        {booking.slot_number}
                      </span>
                    </div>
                    <div>
                      <div className="font-semibold text-gray-900">{booking.lot_name}</div>
                      <div className="text-sm text-gray-500">
                        {booking.vehicle_plate || 'Kein Kennzeichen'}
                      </div>
                    </div>
                  </div>
                  
                  <div className="text-right">
                    <div className="text-sm text-gray-500">
                      {new Date(booking.start_time).toLocaleDateString('de-DE')}
                    </div>
                    <div className="font-medium text-gray-900">
                      {new Date(booking.start_time).toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' })}
                      {' — '}
                      {new Date(booking.end_time).toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' })}
                    </div>
                    <button
                      onClick={() => handleCancel(booking.id)}
                      disabled={cancelling === booking.id}
                      className="mt-2 text-sm text-red-600 hover:text-red-700"
                    >
                      {cancelling === booking.id ? 'Wird storniert...' : 'Stornieren'}
                    </button>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Past Bookings */}
      {pastBookings.length > 0 && (
        <div>
          <h2 className="text-lg font-semibold text-gray-900 mb-4">
            Vergangene Buchungen ({pastBookings.length})
          </h2>
          
          <div className="space-y-3">
            {pastBookings.slice(0, 10).map((booking) => (
              <div key={booking.id} className="card bg-gray-50">
                <div className="flex items-center justify-between">
                  <div className="flex items-center space-x-4">
                    <div className="w-12 h-12 bg-gray-200 rounded-lg flex items-center justify-center">
                      <span className="text-lg font-bold text-gray-500">
                        {booking.slot_number}
                      </span>
                    </div>
                    <div>
                      <div className="font-medium text-gray-700">{booking.lot_name}</div>
                      <div className="text-sm text-gray-500">
                        {new Date(booking.start_time).toLocaleDateString('de-DE')}
                      </div>
                    </div>
                  </div>
                  
                  <div className={`px-3 py-1 rounded-full text-xs font-medium ${
                    booking.status === 'completed'
                      ? 'bg-green-100 text-green-700'
                      : 'bg-gray-200 text-gray-600'
                  }`}>
                    {booking.status === 'completed' ? 'Abgeschlossen' : 'Storniert'}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
