import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { api, ParkingLot, Booking } from '../api/client';
import { useAuth } from '../context/AuthContext';

export function DashboardPage() {
  const { user } = useAuth();
  const [lots, setLots] = useState<ParkingLot[]>([]);
  const [activeBookings, setActiveBookings] = useState<Booking[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadData();
  }, []);

  async function loadData() {
    try {
      const [lotsRes, bookingsRes] = await Promise.all([
        api.getLots(),
        api.getBookings(),
      ]);

      if (lotsRes.success && lotsRes.data) {
        setLots(lotsRes.data);
      }
      if (bookingsRes.success && bookingsRes.data) {
        setActiveBookings(bookingsRes.data.filter(b => b.status === 'active'));
      }
    } finally {
      setLoading(false);
    }
  }

  const totalSlots = lots.reduce((sum, lot) => sum + lot.total_slots, 0);
  const availableSlots = lots.reduce((sum, lot) => sum + lot.available_slots, 0);
  const occupancyRate = totalSlots > 0 ? Math.round((1 - availableSlots / totalSlots) * 100) : 0;

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      {/* Welcome */}
      <div>
        <h1 className="text-2xl font-bold text-gray-900">
          Willkommen, {user?.name}! 👋
        </h1>
        <p className="text-gray-600 mt-1">
          Hier ist Ihre Parkplatz-Übersicht für heute.
        </p>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <div className="card">
          <div className="text-sm font-medium text-gray-500">Verfügbare Plätze</div>
          <div className="mt-2 flex items-baseline">
            <span className="text-3xl font-bold text-parking-available">{availableSlots}</span>
            <span className="ml-2 text-gray-500">von {totalSlots}</span>
          </div>
        </div>

        <div className="card">
          <div className="text-sm font-medium text-gray-500">Auslastung</div>
          <div className="mt-2">
            <span className="text-3xl font-bold text-gray-900">{occupancyRate}%</span>
          </div>
          <div className="mt-2 w-full bg-gray-200 rounded-full h-2">
            <div
              className="bg-primary-600 h-2 rounded-full transition-all"
              style={{ width: `${occupancyRate}%` }}
            ></div>
          </div>
        </div>

        <div className="card">
          <div className="text-sm font-medium text-gray-500">Ihre aktiven Buchungen</div>
          <div className="mt-2 flex items-baseline">
            <span className="text-3xl font-bold text-primary-600">{activeBookings.length}</span>
          </div>
        </div>
      </div>

      {/* Active Bookings */}
      {activeBookings.length > 0 && (
        <div className="card">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Aktive Buchungen</h2>
          <div className="space-y-3">
            {activeBookings.map((booking) => (
              <div
                key={booking.id}
                className="flex items-center justify-between p-4 bg-gray-50 rounded-lg"
              >
                <div className="flex items-center space-x-4">
                  <div className="w-12 h-12 bg-primary-100 rounded-lg flex items-center justify-center">
                    <span className="text-xl">🅿️</span>
                  </div>
                  <div>
                    <div className="font-medium text-gray-900">
                      Platz {booking.slot_number}
                    </div>
                    <div className="text-sm text-gray-500">
                      {booking.lot_name} • {booking.vehicle_plate || 'Kein Kennzeichen'}
                    </div>
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-sm font-medium text-gray-900">
                    Bis {new Date(booking.end_time).toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' })}
                  </div>
                  <div className="text-sm text-gray-500">
                    {new Date(booking.end_time).toLocaleDateString('de-DE')}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Parking Lots */}
      <div>
        <h2 className="text-lg font-semibold text-gray-900 mb-4">Parkplätze</h2>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {lots.map((lot) => (
            <Link
              key={lot.id}
              to={`/book?lot=${lot.id}`}
              className="card hover:shadow-lg transition-shadow"
            >
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="font-semibold text-gray-900">{lot.name}</h3>
                  <p className="text-sm text-gray-500">{lot.address}</p>
                </div>
                <div className="text-right">
                  <div className="text-2xl font-bold text-parking-available">
                    {lot.available_slots}
                  </div>
                  <div className="text-xs text-gray-500">frei</div>
                </div>
              </div>
              <div className="mt-4 flex items-center justify-between">
                <div className="w-full bg-gray-200 rounded-full h-2">
                  <div
                    className={`h-2 rounded-full ${
                      lot.available_slots === 0 ? 'bg-parking-occupied' : 'bg-parking-available'
                    }`}
                    style={{ width: `${(lot.available_slots / lot.total_slots) * 100}%` }}
                  ></div>
                </div>
                <span className="ml-3 text-sm text-gray-500">
                  {lot.total_slots - lot.available_slots}/{lot.total_slots}
                </span>
              </div>
            </Link>
          ))}
        </div>
      </div>

      {/* Quick Actions */}
      <div className="card bg-primary-50 border border-primary-100">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="font-semibold text-primary-900">Jetzt Parkplatz buchen</h3>
            <p className="text-sm text-primary-700">
              Wählen Sie einen freien Platz für heute oder die kommenden Tage
            </p>
          </div>
          <Link to="/book" className="btn btn-primary">
            Buchen →
          </Link>
        </div>
      </div>
    </div>
  );
}
