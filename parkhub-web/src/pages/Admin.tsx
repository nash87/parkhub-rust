import { useEffect, useState } from 'react';
import { Routes, Route, Link, useLocation } from 'react-router-dom';
import { api, ParkingLot } from '../api/client';

function AdminNav() {
  const location = useLocation();
  
  const tabs = [
    { name: 'Übersicht', path: '/admin', icon: '📊' },
    { name: 'Parkplätze', path: '/admin/lots', icon: '🅿️' },
    { name: 'Benutzer', path: '/admin/users', icon: '👥' },
    { name: 'Buchungen', path: '/admin/bookings', icon: '📋' },
  ];

  return (
    <div className="border-b border-gray-200 mb-6">
      <nav className="flex space-x-4 overflow-x-auto">
        {tabs.map((tab) => (
          <Link
            key={tab.path}
            to={tab.path}
            className={`px-4 py-3 text-sm font-medium whitespace-nowrap border-b-2 transition-colors ${
              location.pathname === tab.path
                ? 'border-primary-600 text-primary-600'
                : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
            }`}
          >
            <span className="mr-2">{tab.icon}</span>
            {tab.name}
          </Link>
        ))}
      </nav>
    </div>
  );
}

function AdminOverview() {
  const [stats, setStats] = useState({
    totalLots: 0,
    totalSlots: 0,
    availableSlots: 0,
    activeBookings: 0,
    totalUsers: 0,
  });

  useEffect(() => {
    loadStats();
  }, []);

  async function loadStats() {
    const lotsRes = await api.getLots();
    if (lotsRes.success && lotsRes.data) {
      const lots = lotsRes.data;
      setStats({
        totalLots: lots.length,
        totalSlots: lots.reduce((sum, l) => sum + l.total_slots, 0),
        availableSlots: lots.reduce((sum, l) => sum + l.available_slots, 0),
        activeBookings: 0, // Would need separate endpoint
        totalUsers: 0, // Would need separate endpoint
      });
    }
  }

  return (
    <div className="space-y-6">
      <h2 className="text-xl font-semibold text-gray-900">System-Übersicht</h2>
      
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <div className="card">
          <div className="text-sm font-medium text-gray-500">Parkplätze</div>
          <div className="mt-2 text-3xl font-bold text-gray-900">{stats.totalLots}</div>
        </div>
        <div className="card">
          <div className="text-sm font-medium text-gray-500">Stellplätze gesamt</div>
          <div className="mt-2 text-3xl font-bold text-gray-900">{stats.totalSlots}</div>
        </div>
        <div className="card">
          <div className="text-sm font-medium text-gray-500">Verfügbar</div>
          <div className="mt-2 text-3xl font-bold text-parking-available">{stats.availableSlots}</div>
        </div>
        <div className="card">
          <div className="text-sm font-medium text-gray-500">Auslastung</div>
          <div className="mt-2 text-3xl font-bold text-primary-600">
            {stats.totalSlots > 0
              ? Math.round((1 - stats.availableSlots / stats.totalSlots) * 100)
              : 0}%
          </div>
        </div>
      </div>

      <div className="card">
        <h3 className="text-lg font-semibold text-gray-900 mb-4">Schnellaktionen</h3>
        <div className="flex flex-wrap gap-3">
          <Link to="/admin/lots" className="btn btn-primary">
            🅿️ Parkplatz anlegen
          </Link>
          <Link to="/admin/users" className="btn btn-secondary">
            👤 Benutzer verwalten
          </Link>
        </div>
      </div>
    </div>
  );
}

function AdminLots() {
  const [lots, setLots] = useState<ParkingLot[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadLots();
  }, []);

  async function loadLots() {
    try {
      const res = await api.getLots();
      if (res.success && res.data) {
        setLots(res.data);
      }
    } finally {
      setLoading(false);
    }
  }

  if (loading) {
    return <div className="animate-pulse">Lädt...</div>;
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-gray-900">Parkplätze verwalten</h2>
        <button className="btn btn-primary">
          + Neuer Parkplatz
        </button>
      </div>

      <div className="space-y-4">
        {lots.map((lot) => (
          <div key={lot.id} className="card">
            <div className="flex items-center justify-between">
              <div>
                <h3 className="font-semibold text-gray-900">{lot.name}</h3>
                <p className="text-sm text-gray-500">{lot.address}</p>
              </div>
              <div className="flex items-center space-x-4">
                <div className="text-right">
                  <div className="font-bold text-gray-900">
                    {lot.available_slots}/{lot.total_slots}
                  </div>
                  <div className="text-xs text-gray-500">verfügbar</div>
                </div>
                <button className="btn btn-secondary text-sm">
                  Bearbeiten
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function AdminUsers() {
  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-gray-900">Benutzer verwalten</h2>
        <button className="btn btn-primary">
          + Neuer Benutzer
        </button>
      </div>

      <div className="card text-center py-12 text-gray-500">
        <span className="text-4xl mb-4 block">👥</span>
        <p>Benutzerverwaltung kommt bald</p>
        <p className="text-sm mt-2">
          Hier können Sie Benutzer anlegen, bearbeiten und Rollen zuweisen.
        </p>
      </div>
    </div>
  );
}

function AdminBookings() {
  return (
    <div className="space-y-6">
      <h2 className="text-xl font-semibold text-gray-900">Alle Buchungen</h2>

      <div className="card text-center py-12 text-gray-500">
        <span className="text-4xl mb-4 block">📋</span>
        <p>Buchungsübersicht kommt bald</p>
        <p className="text-sm mt-2">
          Hier sehen Sie alle Buchungen aller Benutzer.
        </p>
      </div>
    </div>
  );
}

export function AdminPage() {
  return (
    <div>
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-gray-900">Administration</h1>
        <p className="text-gray-600 mt-1">System- und Benutzerverwaltung</p>
      </div>

      <AdminNav />

      <Routes>
        <Route path="/" element={<AdminOverview />} />
        <Route path="/lots" element={<AdminLots />} />
        <Route path="/users" element={<AdminUsers />} />
        <Route path="/bookings" element={<AdminBookings />} />
      </Routes>
    </div>
  );
}
