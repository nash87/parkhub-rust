import { useEffect, useState } from 'react';
import { useSearchParams, useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Car,
  Clock,
  CheckCircle,
  Warning,
  SpinnerGap,
  MapPin,
  Star,
} from '@phosphor-icons/react';
import { api, ParkingLot, ParkingSlot, Vehicle } from '../api/client';
import toast from 'react-hot-toast';
import { format, addMinutes } from 'date-fns';
import { de } from 'date-fns/locale';

const DURATION_OPTIONS = [
  { value: 30, label: '30 Min' },
  { value: 60, label: '1 Std' },
  { value: 120, label: '2 Std' },
  { value: 240, label: '4 Std' },
  { value: 480, label: '8 Std' },
  { value: 720, label: '12 Std' },
];

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
  const [favorites, setFavorites] = useState<string[]>([]);

  useEffect(() => {
    loadInitialData();
    // Load favorites from localStorage
    const saved = localStorage.getItem('parkhub_favorites');
    if (saved) setFavorites(JSON.parse(saved));
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
        if (preselectedLot) setSelectedLot(preselectedLot);
      }
      if (vehiclesRes.success && vehiclesRes.data) {
        setVehicles(vehiclesRes.data);
        const defaultVehicle = vehiclesRes.data.find(v => v.is_default);
        if (defaultVehicle) setSelectedVehicle(defaultVehicle.id);
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

  function toggleFavorite(slotId: string) {
    const newFavorites = favorites.includes(slotId)
      ? favorites.filter(id => id !== slotId)
      : [...favorites, slotId];
    setFavorites(newFavorites);
    localStorage.setItem('parkhub_favorites', JSON.stringify(newFavorites));
  }

  async function handleBook() {
    if (!selectedSlot) return;

    setBooking(true);
    const startTime = new Date();

    const res = await api.createBooking({
      slot_id: selectedSlot.id,
      start_time: startTime.toISOString(),
      duration_minutes: duration,
      vehicle_id: selectedVehicle || undefined,
      license_plate: !selectedVehicle ? customPlate : undefined,
    });

    if (res.success) {
      toast.success('Parkplatz erfolgreich gebucht!');
      navigate('/bookings');
    } else {
      toast.error(res.error?.message || 'Buchung fehlgeschlagen');
    }
    setBooking(false);
  }

  const selectedLotData = lots.find(l => l.id === selectedLot);
  const availableSlots = slots.filter(s => s.status === 'available');
  const endTime = addMinutes(new Date(), duration);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" />
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto space-y-8">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Parkplatz buchen
        </h1>
        <p className="text-gray-500 dark:text-gray-400 mt-1">
          Wählen Sie Ihren Stellplatz und die gewünschte Dauer
        </p>
      </div>

      {/* Step 1: Select Lot */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        className="card p-6"
      >
        <div className="flex items-center gap-3 mb-6">
          <div className="w-8 h-8 bg-primary-100 dark:bg-primary-900/30 rounded-lg flex items-center justify-center">
            <span className="text-sm font-bold text-primary-600 dark:text-primary-400">1</span>
          </div>
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            Parkplatz wählen
          </h2>
        </div>
        
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {lots.map((lot) => (
            <button
              key={lot.id}
              onClick={() => {
                setSelectedLot(lot.id);
                setSelectedSlot(null);
              }}
              className={`p-4 rounded-xl border-2 text-left transition-all ${
                selectedLot === lot.id
                  ? 'border-primary-500 bg-primary-50 dark:bg-primary-900/20'
                  : 'border-gray-200 dark:border-gray-700 hover:border-gray-300 dark:hover:border-gray-600'
              }`}
            >
              <div className="flex items-start justify-between">
                <div className="flex items-center gap-3">
                  <MapPin weight="fill" className="w-5 h-5 text-gray-400" />
                  <div>
                    <p className="font-medium text-gray-900 dark:text-white">
                      {lot.name}
                    </p>
                    <p className="text-sm text-gray-500 dark:text-gray-400">
                      {lot.address}
                    </p>
                  </div>
                </div>
                <div className={`badge ${
                  lot.available_slots === 0 ? 'badge-error' : 'badge-success'
                }`}>
                  {lot.available_slots} frei
                </div>
              </div>
            </button>
          ))}
        </div>
      </motion.div>

      {/* Step 2: Select Slot */}
      <AnimatePresence>
        {selectedLot && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -20 }}
            className="card p-6"
          >
            <div className="flex items-center gap-3 mb-6">
              <div className="w-8 h-8 bg-primary-100 dark:bg-primary-900/30 rounded-lg flex items-center justify-center">
                <span className="text-sm font-bold text-primary-600 dark:text-primary-400">2</span>
              </div>
              <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                Stellplatz wählen
              </h2>
              {selectedLotData && (
                <span className="ml-auto text-sm text-gray-500 dark:text-gray-400">
                  {availableSlots.length} von {slots.length} verfügbar
                </span>
              )}
            </div>
            
            {availableSlots.length === 0 ? (
              <div className="text-center py-12">
                <Warning weight="fill" className="w-12 h-12 text-amber-500 mx-auto mb-4" />
                <p className="text-gray-600 dark:text-gray-400">
                  Keine freien Plätze verfügbar
                </p>
              </div>
            ) : (
              <>
                <div className="grid grid-cols-6 sm:grid-cols-8 md:grid-cols-10 gap-2">
                  {slots.map((slot) => {
                    const isFavorite = favorites.includes(slot.id);
                    const isSelected = selectedSlot?.id === slot.id;
                    
                    return (
                      <div key={slot.id} className="relative group">
                        <button
                          onClick={() => slot.status === 'available' && setSelectedSlot(slot)}
                          disabled={slot.status !== 'available'}
                          className={`slot w-full ${
                            slot.status === 'available'
                              ? isSelected
                                ? 'slot-selected bg-primary-600'
                                : 'slot-available'
                              : slot.status === 'occupied'
                              ? 'slot-occupied'
                              : slot.status === 'reserved'
                              ? 'slot-reserved'
                              : 'slot-disabled'
                          }`}
                        >
                          <span>{slot.number}</span>
                          {isFavorite && (
                            <Star weight="fill" className="w-3 h-3 absolute top-1 right-1" />
                          )}
                        </button>
                        
                        {slot.status === 'available' && (
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              toggleFavorite(slot.id);
                            }}
                            className="absolute -top-1 -right-1 w-5 h-5 bg-white dark:bg-gray-800 rounded-full shadow opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center"
                          >
                            {isFavorite ? (
                              <Star weight="fill" className="w-3 h-3 text-amber-500" />
                            ) : (
                              <Star weight="regular" className="w-3 h-3 text-gray-400" />
                            )}
                          </button>
                        )}
                      </div>
                    );
                  })}
                </div>
                
                <div className="flex flex-wrap items-center gap-4 mt-6 pt-6 border-t border-gray-100 dark:border-gray-800">
                  <div className="flex items-center gap-2">
                    <div className="w-4 h-4 rounded bg-emerald-500" />
                    <span className="text-sm text-gray-600 dark:text-gray-400">Frei</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <div className="w-4 h-4 rounded bg-gray-300 dark:bg-gray-700" />
                    <span className="text-sm text-gray-600 dark:text-gray-400">Belegt</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <div className="w-4 h-4 rounded bg-amber-500" />
                    <span className="text-sm text-gray-600 dark:text-gray-400">Reserviert</span>
                  </div>
                  <div className="flex items-center gap-2">
                    <Star weight="fill" className="w-4 h-4 text-amber-500" />
                    <span className="text-sm text-gray-600 dark:text-gray-400">Favorit</span>
                  </div>
                </div>
              </>
            )}
          </motion.div>
        )}
      </AnimatePresence>

      {/* Step 3: Duration & Vehicle */}
      <AnimatePresence>
        {selectedSlot && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -20 }}
            className="card p-6"
          >
            <div className="flex items-center gap-3 mb-6">
              <div className="w-8 h-8 bg-primary-100 dark:bg-primary-900/30 rounded-lg flex items-center justify-center">
                <span className="text-sm font-bold text-primary-600 dark:text-primary-400">3</span>
              </div>
              <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
                Dauer & Fahrzeug
              </h2>
            </div>
            
            <div className="space-y-6">
              {/* Duration */}
              <div>
                <label className="label flex items-center gap-2">
                  <Clock weight="regular" className="w-4 h-4" />
                  Parkdauer
                </label>
                <div className="grid grid-cols-3 sm:grid-cols-6 gap-2">
                  {DURATION_OPTIONS.map((opt) => (
                    <button
                      key={opt.value}
                      onClick={() => setDuration(opt.value)}
                      className={`py-2.5 px-4 rounded-xl text-sm font-medium transition-all ${
                        duration === opt.value
                          ? 'bg-primary-600 text-white'
                          : 'bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300 hover:bg-gray-200 dark:hover:bg-gray-700'
                      }`}
                    >
                      {opt.label}
                    </button>
                  ))}
                </div>
                <p className="text-sm text-gray-500 dark:text-gray-400 mt-2">
                  Bis {format(endTime, 'HH:mm')} Uhr ({format(endTime, 'EEEE', { locale: de })})
                </p>
              </div>

              {/* Vehicle */}
              <div>
                <label className="label flex items-center gap-2">
                  <Car weight="regular" className="w-4 h-4" />
                  Fahrzeug
                </label>
                {vehicles.length > 0 ? (
                  <div className="space-y-2">
                    <select
                      value={selectedVehicle}
                      onChange={(e) => {
                        setSelectedVehicle(e.target.value);
                        setCustomPlate('');
                      }}
                      className="input"
                    >
                      <option value="">Anderes Kennzeichen eingeben</option>
                      {vehicles.map((v) => (
                        <option key={v.id} value={v.id}>
                          {v.license_plate} {v.make && v.model ? `(${v.make} ${v.model})` : ''}
                        </option>
                      ))}
                    </select>
                  </div>
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
          </motion.div>
        )}
      </AnimatePresence>

      {/* Summary & Book */}
      <AnimatePresence>
        {selectedSlot && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -20 }}
            className="card bg-gradient-to-br from-primary-600 to-primary-700 p-6 text-white"
          >
            <h2 className="text-lg font-semibold mb-4">Zusammenfassung</h2>
            
            <div className="grid grid-cols-2 gap-4 mb-6">
              <div>
                <p className="text-white/70 text-sm">Parkplatz</p>
                <p className="font-medium">{selectedLotData?.name}</p>
              </div>
              <div>
                <p className="text-white/70 text-sm">Stellplatz</p>
                <p className="font-medium">{selectedSlot.number}</p>
              </div>
              <div>
                <p className="text-white/70 text-sm">Dauer</p>
                <p className="font-medium">
                  {DURATION_OPTIONS.find(o => o.value === duration)?.label}
                </p>
              </div>
              <div>
                <p className="text-white/70 text-sm">Ende</p>
                <p className="font-medium">{format(endTime, 'HH:mm')} Uhr</p>
              </div>
              <div className="col-span-2">
                <p className="text-white/70 text-sm">Kennzeichen</p>
                <p className="font-medium">
                  {selectedVehicle
                    ? vehicles.find(v => v.id === selectedVehicle)?.license_plate
                    : customPlate || '—'}
                </p>
              </div>
            </div>

            <button
              onClick={handleBook}
              disabled={booking || (!selectedVehicle && !customPlate)}
              className="btn bg-white text-primary-700 hover:bg-white/90 w-full justify-center"
            >
              {booking ? (
                <SpinnerGap weight="bold" className="w-5 h-5 animate-spin" />
              ) : (
                <>
                  <CheckCircle weight="bold" className="w-5 h-5" />
                  Jetzt buchen
                </>
              )}
            </button>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
