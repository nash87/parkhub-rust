import { useEffect, useState } from 'react';
import { api, Vehicle } from '../api/client';

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
  const [error, setError] = useState('');

  useEffect(() => {
    loadVehicles();
  }, []);

  async function loadVehicles() {
    try {
      const res = await api.getVehicles();
      if (res.success && res.data) {
        setVehicles(res.data);
      }
    } finally {
      setLoading(false);
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError('');

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
    } else {
      setError(res.error?.message || 'Fehler beim Speichern');
    }
    setSaving(false);
  }

  async function handleDelete(id: string) {
    if (!confirm('Fahrzeug wirklich löschen?')) return;

    const res = await api.deleteVehicle(id);
    if (res.success) {
      setVehicles(vehicles.filter(v => v.id !== id));
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-600"></div>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">Meine Fahrzeuge</h1>
          <p className="text-gray-600 mt-1">Verwalten Sie Ihre registrierten Fahrzeuge</p>
        </div>
        <button
          onClick={() => setShowForm(!showForm)}
          className="btn btn-primary"
        >
          {showForm ? 'Abbrechen' : '+ Fahrzeug hinzufügen'}
        </button>
      </div>

      {/* Add Vehicle Form */}
      {showForm && (
        <div className="card">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Neues Fahrzeug</h2>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label className="block text-sm font-medium text-gray-700">
                  Kennzeichen *
                </label>
                <input
                  type="text"
                  value={formData.license_plate}
                  onChange={(e) => setFormData({ ...formData, license_plate: e.target.value.toUpperCase() })}
                  placeholder="z.B. M-AB 1234"
                  className="input mt-1"
                  required
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700">
                  Marke
                </label>
                <input
                  type="text"
                  value={formData.make}
                  onChange={(e) => setFormData({ ...formData, make: e.target.value })}
                  placeholder="z.B. BMW"
                  className="input mt-1"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700">
                  Modell
                </label>
                <input
                  type="text"
                  value={formData.model}
                  onChange={(e) => setFormData({ ...formData, model: e.target.value })}
                  placeholder="z.B. 3er"
                  className="input mt-1"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700">
                  Farbe
                </label>
                <input
                  type="text"
                  value={formData.color}
                  onChange={(e) => setFormData({ ...formData, color: e.target.value })}
                  placeholder="z.B. Schwarz"
                  className="input mt-1"
                />
              </div>
            </div>

            {error && (
              <div className="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm">
                {error}
              </div>
            )}

            <div className="flex justify-end">
              <button type="submit" disabled={saving} className="btn btn-primary">
                {saving ? 'Wird gespeichert...' : 'Speichern'}
              </button>
            </div>
          </form>
        </div>
      )}

      {/* Vehicle List */}
      {vehicles.length === 0 ? (
        <div className="card text-center py-12 text-gray-500">
          <span className="text-4xl mb-4 block">🚗</span>
          <p>Keine Fahrzeuge registriert</p>
          <p className="text-sm mt-2">
            Fügen Sie Ihr erstes Fahrzeug hinzu, um schneller buchen zu können
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {vehicles.map((vehicle) => (
            <div key={vehicle.id} className="card">
              <div className="flex items-start justify-between">
                <div className="flex items-center space-x-4">
                  <div className="w-14 h-14 bg-gray-100 rounded-xl flex items-center justify-center">
                    <span className="text-2xl">🚗</span>
                  </div>
                  <div>
                    <div className="font-bold text-gray-900 text-lg">
                      {vehicle.license_plate}
                    </div>
                    {(vehicle.make || vehicle.model) && (
                      <div className="text-sm text-gray-600">
                        {vehicle.make} {vehicle.model}
                      </div>
                    )}
                    {vehicle.color && (
                      <div className="text-sm text-gray-500">{vehicle.color}</div>
                    )}
                  </div>
                </div>
                <button
                  onClick={() => handleDelete(vehicle.id)}
                  className="text-gray-400 hover:text-red-600 p-2"
                  title="Löschen"
                >
                  🗑️
                </button>
              </div>
              {vehicle.is_default && (
                <div className="mt-3 inline-block px-2 py-1 bg-primary-100 text-primary-700 text-xs font-medium rounded">
                  Standard-Fahrzeug
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
