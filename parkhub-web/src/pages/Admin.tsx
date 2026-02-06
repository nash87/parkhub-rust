import { useEffect, useState } from 'react';
import { Routes, Route, Link, useLocation } from 'react-router-dom';
import { motion } from 'framer-motion';
import {
  ChartBar,
  Buildings,
  Users,
  ListChecks,
  Plus,
  CheckCircle,
  TrendUp,
  CaretRight,
  SpinnerGap,
} from '@phosphor-icons/react';
import { api, ParkingLot } from '../api/client';

const tabs = [
  { name: 'Übersicht', path: '/admin', icon: ChartBar },
  { name: 'Parkplätze', path: '/admin/lots', icon: Buildings },
  { name: 'Benutzer', path: '/admin/users', icon: Users },
  { name: 'Buchungen', path: '/admin/bookings', icon: ListChecks },
];

function AdminNav() {
  const location = useLocation();

  return (
    <div className="border-b border-gray-200 dark:border-gray-800 mb-8">
      <nav className="flex gap-1 overflow-x-auto">
        {tabs.map((tab) => {
          const Icon = tab.icon;
          const isActive = location.pathname === tab.path;
          
          return (
            <Link
              key={tab.path}
              to={tab.path}
              className={`flex items-center gap-2 px-4 py-3 text-sm font-medium whitespace-nowrap border-b-2 transition-colors ${
                isActive
                  ? 'border-primary-600 text-primary-600 dark:text-primary-400'
                  : 'border-transparent text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-300'
              }`}
            >
              <Icon weight={isActive ? 'fill' : 'regular'} className="w-5 h-5" />
              {tab.name}
            </Link>
          );
        })}
      </nav>
    </div>
  );
}

function AdminOverview() {
  const [stats, setStats] = useState({
    totalLots: 0,
    totalSlots: 0,
    availableSlots: 0,
  });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    loadStats();
  }, []);

  async function loadStats() {
    try {
      const lotsRes = await api.getLots();
      if (lotsRes.success && lotsRes.data) {
        const lots = lotsRes.data;
        setStats({
          totalLots: lots.length,
          totalSlots: lots.reduce((sum, l) => sum + l.total_slots, 0),
          availableSlots: lots.reduce((sum, l) => sum + l.available_slots, 0),
        });
      }
    } finally {
      setLoading(false);
    }
  }

  const occupancyRate = stats.totalSlots > 0
    ? Math.round((1 - stats.availableSlots / stats.totalSlots) * 100)
    : 0;

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" />
      </div>
    );
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="space-y-8"
    >
      <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
        System-Übersicht
      </h2>
      
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <div className="stat-card">
          <div className="flex items-center justify-between">
            <div>
              <p className="stat-label">Parkplätze</p>
              <p className="stat-value text-gray-900 dark:text-white">{stats.totalLots}</p>
            </div>
            <Buildings weight="fill" className="w-8 h-8 text-gray-300 dark:text-gray-700" />
          </div>
        </div>
        
        <div className="stat-card">
          <div className="flex items-center justify-between">
            <div>
              <p className="stat-label">Stellplätze gesamt</p>
              <p className="stat-value text-gray-900 dark:text-white">{stats.totalSlots}</p>
            </div>
            <ListChecks weight="fill" className="w-8 h-8 text-gray-300 dark:text-gray-700" />
          </div>
        </div>
        
        <div className="stat-card">
          <div className="flex items-center justify-between">
            <div>
              <p className="stat-label">Verfügbar</p>
              <p className="stat-value text-emerald-600">{stats.availableSlots}</p>
            </div>
            <CheckCircle weight="fill" className="w-8 h-8 text-emerald-200 dark:text-emerald-900" />
          </div>
        </div>
        
        <div className="stat-card">
          <div className="flex items-center justify-between">
            <div>
              <p className="stat-label">Auslastung</p>
              <p className="stat-value text-primary-600">{occupancyRate}%</p>
            </div>
            <TrendUp weight="fill" className="w-8 h-8 text-primary-200 dark:text-primary-900" />
          </div>
        </div>
      </div>

      <div className="card p-6">
        <h3 className="text-lg font-semibold text-gray-900 dark:text-white mb-4">
          Schnellaktionen
        </h3>
        <div className="flex flex-wrap gap-3">
          <Link to="/admin/lots" className="btn btn-primary">
            <Plus weight="bold" className="w-4 h-4" />
            Parkplatz anlegen
          </Link>
          <Link to="/admin/users" className="btn btn-secondary">
            <Users weight="regular" className="w-4 h-4" />
            Benutzer verwalten
          </Link>
        </div>
      </div>
    </motion.div>
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
      if (res.success && res.data) setLots(res.data);
    } finally {
      setLoading(false);
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
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="space-y-6"
    >
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
          Parkplätze verwalten
        </h2>
        <button className="btn btn-primary">
          <Plus weight="bold" className="w-4 h-4" />
          Neuer Parkplatz
        </button>
      </div>

      <div className="space-y-4">
        {lots.map((lot) => (
          <div key={lot.id} className="card-hover p-6">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-4">
                <div className="w-12 h-12 bg-gray-100 dark:bg-gray-800 rounded-xl flex items-center justify-center">
                  <Buildings weight="fill" className="w-6 h-6 text-gray-500" />
                </div>
                <div>
                  <p className="font-semibold text-gray-900 dark:text-white">{lot.name}</p>
                  <p className="text-sm text-gray-500 dark:text-gray-400">{lot.address}</p>
                </div>
              </div>
              <div className="flex items-center gap-4">
                <div className="text-right">
                  <p className="font-bold text-gray-900 dark:text-white">
                    {lot.available_slots}/{lot.total_slots}
                  </p>
                  <p className="text-xs text-gray-500">verfügbar</p>
                </div>
                <button className="btn btn-secondary btn-sm">
                  Bearbeiten
                  <CaretRight weight="bold" className="w-4 h-4" />
                </button>
              </div>
            </div>
          </div>
        ))}
      </div>
    </motion.div>
  );
}

function AdminUsers() {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="space-y-6"
    >
      <div className="flex items-center justify-between">
        <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
          Benutzer verwalten
        </h2>
        <button className="btn btn-primary">
          <Plus weight="bold" className="w-4 h-4" />
          Neuer Benutzer
        </button>
      </div>

      <div className="card p-12 text-center">
        <Users weight="light" className="w-16 h-16 text-gray-300 dark:text-gray-700 mx-auto mb-4" />
        <p className="text-gray-500 dark:text-gray-400">
          Benutzerverwaltung kommt bald
        </p>
      </div>
    </motion.div>
  );
}

function AdminBookings() {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="space-y-6"
    >
      <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
        Alle Buchungen
      </h2>

      <div className="card p-12 text-center">
        <ListChecks weight="light" className="w-16 h-16 text-gray-300 dark:text-gray-700 mx-auto mb-4" />
        <p className="text-gray-500 dark:text-gray-400">
          Buchungsübersicht kommt bald
        </p>
      </div>
    </motion.div>
  );
}

export function AdminPage() {
  return (
    <div>
      <div className="mb-2">
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Administration
        </h1>
        <p className="text-gray-500 dark:text-gray-400 mt-1">
          System- und Benutzerverwaltung
        </p>
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
