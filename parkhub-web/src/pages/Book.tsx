import { useEffect, useState } from 'react';
import { useSearchParams, useNavigate } from 'react-router-dom';
import { api, ParkingLot, ParkingSlot, Vehicle } from '../api/client';

export function BookPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const preselectedLot = searchParams.get('lot');

  const [lots, setLots] = useState<ParkingLot[]>([]);
  const [selectedLot, setSelectedLot] = useState<string>(preselectedLot || '');
  const [slots, setSlots] = useState<ParkingSlot[]>([]);
  const [selectedSlot, setSelectedSlot] = useState<ParkingSlot | null>(null);
  const [vehicles, setVehicles] = useState<Vehicle[]>([]);
  const [selectedVehicle, setSelectedVehicle] = useState<string>('');
  const [customPlate, setCustomPlate] = useState('');
  const [duration, setDuration] = useState(60);
  const [loading, setLoading] = useState(true);
  const [booking, setBooking] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    loadInitialData();
  }, []);

  useEffect(() => {
    if (selectedLot) {
      loadSlots(selectedLot);
    }
  }, [selectedLot]);

  async function loadInitialData() {
    try {
      const [lotsRes, vehiclesRes] = await Promise.all([
        api.getLots(),
        api.getVehicles(),
      ]);

      if (lotsRes.success && lotsRes.data) {
        setLots(lotsRes.data);
        if (preselectedLot) {
          setSelectedLot(preselectedLot);
        }
      }
      if (vehiclesRes.success && vehiclesRes.data) {
        setVehicles(vehiclesRes.data);
        const defaultVehicle = vehiclesRes.data.find(v => v.is_default);
        if (defaultVehicle) {
          setSelectedVehicle(defaultVehicle.id);
        }
      }
    } finally {
      setLoading(false);
    }
  }

  async function loadSlots(lotId: string) {
    const res = await api.getLotSlots(lotId);
    if (res.success && res.data) {
      setSlots(res.data);
    }
  }

  async function handleBook() {
    if (!selectedSlot) return;

    setBooking(true);
    setError('');

    const startTime = new Date();
    const res = await api.createBooking({
      slot_id: selectedSlot.id,
      start_time: startTime.toISOString(),
      duration_minutes: duration,
      vehicle_id: selectedVehicle || undefined,
      license_plate: !selectedVehicle ? customPlate : undefined,
    });

    if (res.success) {
      navigate('/bookings');
    } else {
      setError(res.error?.message || 'Buchung fehlgeschlagen');
    }
    setBooking(false);
  }

  const selectedLotData = lots.find(l => l.id === selectedLot);
  const availableSlots = slots.filter(s => s.status === 'available');

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto space-y-8">
      <div>
        <h1 className="text-2xl font-bold text-gray-900">Parkplatz buchen</h1>
        <p className="text-gray-600 mt-1">Wählen Sie einen Parkplatz und die gewünschte Dauer</p>
      </div>

      {/* Step 1: Select Lot */}
      <div className="card">
        <h2 className="text-lg font-semibold text-gray-900 mb-4">
          1. Parkplatz wählen
        </h2>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {lots.map((lot) => (
            <button
              key={lot.id}
              onClick={() => setSelectedLot(lot.id)}
              className={`p-4 rounded-lg border-2 text-left transition-colors ${
                selectedLot === lot.id
                  ? 'border-primary-500 bg-primary-50'
                  : 'border-gray-200 hover:border-gray-300'
              }`}
            >
              <div className="flex justify-between items-start">
                <div>
                  <div className="font-medium text-gray-900">{lot.name}</div>
                  <div className="text-sm text-gray-500">{lot.address}</div>
                </div>
                <div className={`text-lg font-bold ${
                  lot.available_slots > 0 ? 'text-parking-available' : 'text-parking-occupied'
                }`}>
                  {lot.available_slots} frei
                </div>
              </div>
            </button>
          ))}
        </div>
      </div>

      {/* Step 2: Select Slot */}
      {selectedLot && (
        <div className="card">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">
            2. Stellplatz wählen
          </h2>
          {availableSlots.length === 0 ? (
            <div className="text-center py-8 text-gray-500">
              Keine freien Plätze verfügbar
            </div>
          ) : (
            <div className="grid grid-cols-5 sm:grid-cols-8 md:grid-cols-10 gap-2">
              {slots.map((slot) => (
                <button
                  key={slot.id}
                  onClick={() => slot.status === 'available' && setSelectedSlot(slot)}
                  disabled={slot.status !== 'available'}
                  className={`slot ${
                    slot.status === 'available'
                      ? selectedSlot?.id === slot.id
                        ? 'bg-primary-600 ring-2 ring-primary-300'
                        : 'slot-available'
                      : slot.status === 'occupied'
                      ? 'slot-occupied'
                      : slot.status === 'reserved'
                      ? 'slot-reserved'
                      : 'slot-disabled'
                  }`}
                  title={`Platz ${slot.number} - ${slot.status}`}
                >
                  {slot.number}
                </button>
              ))}
            </div>
          )}
          <div className="mt-4 flex items-center gap-4 text-sm">
            <div className="flex items-center gap-2">
              <div className="w-4 h-4 rounded bg-parking-available"></div>
              <span>Frei</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-4 h-4 rounded bg-parking-occupied"></div>
              <span>Belegt</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-4 h-4 rounded bg-parking-reserved"></div>
              <span>Reserviert</span>
            </div>
          </div>
        </div>
      )}

      {/* Step 3: Duration & Vehicle */}
      {selectedSlot && (
        <div className="card">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">
            3. Dauer & Fahrzeug
          </h2>
          
          <div className="space-y-6">
            {/* Duration */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Dauer
              </label>
              <div className="grid grid-cols-4 gap-2">
                {[30, 60, 120, 240, 480, 720].map((mins) => (
                  <button
                    key={mins}
                    onClick={() => setDuration(mins)}
                    className={`py-2 px-3 rounded-lg text-sm font-medium ${
                      duration === mins
                        ? 'bg-primary-600 text-white'
                        : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
                    }`}
                  >
                    {mins < 60 ? `${mins} Min` : `${mins / 60} Std`}
                  </button>
                ))}
              </div>
            </div>

            {/* Vehicle */}
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                Fahrzeug
              </label>
              {vehicles.length > 0 ? (
                <select
                  value={selectedVehicle}
                  onChange={(e) => setSelectedVehicle(e.target.value)}
                  className="input"
                >
                  <option value="">Anderes Kennzeichen eingeben</option>
                  {vehicles.map((v) => (
                    <option key={v.id} value={v.id}>
                      {v.license_plate} {v.make && v.model ? `(${v.make} ${v.model})` : ''}
                    </option>
                  ))}
                </select>
              ) : null}
              
              {!selectedVehicle && (
                <input
                  type="text"
                  value={customPlate}
                  onChange={(e) => setCustomPlate(e.target.value.toUpperCase())}
                  placeholder="Kennzeichen eingeben (z.B. M-AB 1234)"
                  className="input mt-2"
                />
              )}
            </div>
          </div>
        </div>
      )}

      {/* Summary & Book */}
      {selectedSlot && (
        <div className="card bg-primary-50 border border-primary-100">
          <h2 className="text-lg font-semibold text-primary-900 mb-4">
            Zusammenfassung
          </h2>
          
          <div className="space-y-2 mb-6">
            <div className="flex justify-between">
              <span className="text-primary-700">Parkplatz:</span>
              <span className="font-medium text-primary-900">{selectedLotData?.name}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-primary-700">Stellplatz:</span>
              <span className="font-medium text-primary-900">{selectedSlot.number}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-primary-700">Dauer:</span>
              <span className="font-medium text-primary-900">
                {duration < 60 ? `${duration} Minuten` : `${duration / 60} Stunden`}
              </span>
            </div>
            <div className="flex justify-between">
              <span className="text-primary-700">Kennzeichen:</span>
              <span className="font-medium text-primary-900">
                {selectedVehicle
                  ? vehicles.find(v => v.id === selectedVehicle)?.license_plate
                  : customPlate || '—'}
              </span>
            </div>
          </div>

          {error && (
            <div className="mb-4 bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm">
              {error}
            </div>
          )}

          <button
            onClick={handleBook}
            disabled={booking || (!selectedVehicle && !customPlate)}
            className="btn btn-primary w-full"
          >
            {booking ? 'Wird gebucht...' : 'Jetzt buchen'}
          </button>
        </div>
      )}
    </div>
  );
}
