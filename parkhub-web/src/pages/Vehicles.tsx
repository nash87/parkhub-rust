import { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Car, Plus, Trash, SpinnerGap, Star, X } from '@phosphor-icons/react';
import { api, Vehicle } from '../api/client';
import toast from 'react-hot-toast';

export function VehiclesPage() {
  const [vehicles, setVehicles] = useState<Vehicle[]>([]);
  const [loading, setLoading] = useState(true);
  const [showForm, setShowForm] = useState(false);
  const [formData, setFormData] = useState({
    license_plate: '',
    make: '',
    model: '',
    color: '',
  });
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    loadVehicles();
  }, []);

  async function loadVehicles() {
    try {
      const res = await api.getVehicles();
      if (res.success && res.data) setVehicles(res.data);
    } finally {
      setLoading(false);
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);

    const res = await api.createVehicle({
      license_plate: formData.license_plate.toUpperCase(),
      make: formData.make || undefined,
      model: formData.model || undefined,
      color: formData.color || undefined,
    });

    if (res.success && res.data) {
      setVehicles([...vehicles, res.data]);
      setFormData({ license_plate: '', make: '', model: '', color: '' });
      setShowForm(false);
      toast.success('Fahrzeug hinzugefügt');
    } else {
      toast.error('Fehler beim Speichern');
    }
    setSaving(false);
  }

  async function handleDelete(id: string) {
    const res = await api.deleteVehicle(id);
    if (res.success) {
      setVehicles(vehicles.filter(v => v.id !== id));
      toast.success('Fahrzeug entfernt');
    } else {
      toast.error('Löschen fehlgeschlagen');
    }
  }

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
            Meine Fahrzeuge
          </h1>
          <p className="text-gray-500 dark:text-gray-400 mt-1">
            Verwalten Sie Ihre registrierten Fahrzeuge
          </p>
        </div>
        <button
          onClick={() => setShowForm(!showForm)}
          className="btn btn-primary"
        >
          {showForm ? (
            <>
              <X weight="bold" className="w-4 h-4" />
              Abbrechen
            </>
          ) : (
            <>
              <Plus weight="bold" className="w-4 h-4" />
              Fahrzeug hinzufügen
            </>
          )}
        </button>
      </div>

      {/* Add Form */}
      <AnimatePresence>
        {showForm && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="overflow-hidden"
          >
            <div className="card p-6">
              <h2 className="text-lg font-semibold text-gray-900 dark:text-white mb-6">
                Neues Fahrzeug
              </h2>
              <form onSubmit={handleSubmit} className="space-y-4">
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <label className="label">Kennzeichen *</label>
                    <input
                      type="text"
                      value={formData.license_plate}
                      onChange={(e) => setFormData({ ...formData, license_plate: e.target.value.toUpperCase() })}
                      placeholder="M-AB 1234"
                      className="input"
                      required
                    />
                  </div>
                  <div>
                    <label className="label">Marke</label>
                    <input
                      type="text"
                      value={formData.make}
                      onChange={(e) => setFormData({ ...formData, make: e.target.value })}
                      placeholder="BMW"
                      className="input"
                    />
                  </div>
                  <div>
                    <label className="label">Modell</label>
                    <input
                      type="text"
                      value={formData.model}
                      onChange={(e) => setFormData({ ...formData, model: e.target.value })}
                      placeholder="3er"
                      className="input"
                    />
                  </div>
                  <div>
                    <label className="label">Farbe</label>
                    <input
                      type="text"
                      value={formData.color}
                      onChange={(e) => setFormData({ ...formData, color: e.target.value })}
                      placeholder="Schwarz"
                      className="input"
                    />
                  </div>
                </div>

                <div className="flex justify-end pt-4">
                  <button type="submit" disabled={saving} className="btn btn-primary">
                    {saving ? (
                      <SpinnerGap weight="bold" className="w-5 h-5 animate-spin" />
                    ) : (
                      <>
                        <Plus weight="bold" className="w-4 h-4" />
                        Speichern
                      </>
                    )}
                  </button>
                </div>
              </form>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Vehicle List */}
      {vehicles.length === 0 ? (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="card p-12 text-center"
        >
          <Car weight="light" className="w-16 h-16 text-gray-300 dark:text-gray-700 mx-auto mb-4" />
          <p className="text-gray-500 dark:text-gray-400 mb-2">
            Keine Fahrzeuge registriert
          </p>
          <p className="text-sm text-gray-400 dark:text-gray-500">
            Fügen Sie Ihr erstes Fahrzeug hinzu, um schneller buchen zu können
          </p>
        </motion.div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {vehicles.map((vehicle, index) => (
            <motion.div
              key={vehicle.id}
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: index * 0.05 }}
              className="card-hover p-6"
            >
              <div className="flex items-start justify-between">
                <div className="flex items-center gap-4">
                  <div className="w-14 h-14 bg-gray-100 dark:bg-gray-800 rounded-xl flex items-center justify-center">
                    <Car weight="fill" className="w-7 h-7 text-gray-500 dark:text-gray-400" />
                  </div>
                  <div>
                    <p className="text-lg font-bold text-gray-900 dark:text-white">
                      {vehicle.license_plate}
                    </p>
                    {(vehicle.make || vehicle.model) && (
                      <p className="text-sm text-gray-600 dark:text-gray-400">
                        {vehicle.make} {vehicle.model}
                      </p>
                    )}
                    {vehicle.color && (
                      <p className="text-sm text-gray-500 dark:text-gray-500">
                        {vehicle.color}
                      </p>
                    )}
                  </div>
                </div>
                <button
                  onClick={() => handleDelete(vehicle.id)}
                  className="btn btn-ghost btn-icon text-gray-400 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
                >
                  <Trash weight="regular" className="w-5 h-5" />
                </button>
              </div>
              
              {vehicle.is_default && (
                <div className="mt-4 pt-4 border-t border-gray-100 dark:border-gray-800">
                  <span className="badge badge-info">
                    <Star weight="fill" className="w-3 h-3" />
                    Standard-Fahrzeug
                  </span>
                </div>
              )}
            </motion.div>
          ))}
        </div>
      )}
    </div>
  );
}
