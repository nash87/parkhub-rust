import { useState } from 'react';
import { Navigate, Link } from 'react-router-dom';
import { motion } from 'framer-motion';
import { Car, Eye, EyeSlash, ArrowRight, SpinnerGap } from '@phosphor-icons/react';
import { useAuth } from '../context/AuthContext';
import toast from 'react-hot-toast';

export function LoginPage() {
  const { login, isAuthenticated, isLoading } = useAuth();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [loading, setLoading] = useState(false);

  if (isLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-950">
        <SpinnerGap weight="bold" className="w-8 h-8 text-primary-600 animate-spin" />
      </div>
    );
  }

  if (isAuthenticated) {
    return <Navigate to="/" replace />;
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);

    const success = await login(username, password);
    if (success) {
      toast.success('Willkommen zurück!');
    } else {
      toast.error('Ungültige Anmeldedaten');
    }
    setLoading(false);
  }

  return (
    <div className="min-h-screen flex bg-gray-50 dark:bg-gray-950">
      {/* Left Side - Branding */}
      <div className="hidden lg:flex lg:w-1/2 bg-primary-600 dark:bg-primary-700 relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-br from-primary-500 to-primary-700" />
        <div className="absolute inset-0 opacity-10">
          <div className="absolute top-20 left-20 w-64 h-64 bg-white rounded-full blur-3xl" />
          <div className="absolute bottom-20 right-20 w-96 h-96 bg-white rounded-full blur-3xl" />
        </div>
        
        <div className="relative z-10 flex flex-col justify-center px-16 text-white">
          <div className="flex items-center gap-4 mb-8">
            <div className="w-14 h-14 bg-white/20 rounded-2xl flex items-center justify-center">
              <Car weight="fill" className="w-8 h-8" />
            </div>
            <span className="text-3xl font-bold">ParkHub</span>
          </div>
          
          <h1 className="text-4xl font-bold mb-4">
            Intelligentes Parkplatz-Management
          </h1>
          <p className="text-lg text-white/80 mb-8">
            Verwalten Sie Parkplätze effizient. Buchen Sie mit wenigen Klicks.
            Behalten Sie die Übersicht.
          </p>
          
          <div className="grid grid-cols-2 gap-4">
            <div className="bg-white/10 backdrop-blur-sm rounded-2xl p-4">
              <div className="text-3xl font-bold">24/7</div>
              <div className="text-sm text-white/70">Verfügbar</div>
            </div>
            <div className="bg-white/10 backdrop-blur-sm rounded-2xl p-4">
              <div className="text-3xl font-bold">100%</div>
              <div className="text-sm text-white/70">Open Source</div>
            </div>
          </div>
        </div>
      </div>

      {/* Right Side - Login Form */}
      <div className="flex-1 flex items-center justify-center px-6 py-12">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="w-full max-w-md"
        >
          {/* Mobile Logo */}
          <div className="lg:hidden flex items-center gap-3 mb-8">
            <div className="w-12 h-12 bg-primary-600 rounded-2xl flex items-center justify-center">
              <Car weight="fill" className="w-7 h-7 text-white" />
            </div>
            <span className="text-2xl font-bold text-gray-900 dark:text-white">ParkHub</span>
          </div>

          <div className="mb-8">
            <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
              Willkommen zurück
            </h2>
            <p className="text-gray-500 dark:text-gray-400 mt-2">
              Melden Sie sich an, um fortzufahren
            </p>
          </div>

          <form onSubmit={handleSubmit} className="space-y-5">
            <div>
              <label htmlFor="username" className="label">
                Benutzername
              </label>
              <input
                id="username"
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                className="input"
                placeholder="Ihr Benutzername"
                required
                autoFocus
                autoComplete="username"
              />
            </div>

            <div>
              <label htmlFor="password" className="label">
                Passwort
              </label>
              <div className="relative">
                <input
                  id="password"
                  type={showPassword ? 'text' : 'password'}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  className="input pr-12"
                  placeholder="••••••••"
                  required
                  autoComplete="current-password"
                />
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300"
                >
                  {showPassword ? (
                    <EyeSlash weight="regular" className="w-5 h-5" />
                  ) : (
                    <Eye weight="regular" className="w-5 h-5" />
                  )}
                </button>
              </div>
            </div>

            <div className="flex items-center justify-between">
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  className="w-4 h-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500"
                />
                <span className="text-sm text-gray-600 dark:text-gray-400">
                  Angemeldet bleiben
                </span>
              </label>
              <Link
                to="/forgot-password"
                className="text-sm text-primary-600 hover:text-primary-700 dark:text-primary-400"
              >
                Passwort vergessen?
              </Link>
            </div>

            <button
              type="submit"
              disabled={loading}
              className="btn btn-primary w-full justify-center"
            >
              {loading ? (
                <SpinnerGap weight="bold" className="w-5 h-5 animate-spin" />
              ) : (
                <>
                  Anmelden
                  <ArrowRight weight="bold" className="w-5 h-5" />
                </>
              )}
            </button>
          </form>

          <p className="mt-8 text-center text-sm text-gray-500 dark:text-gray-400">
            Noch kein Konto?{' '}
            <Link
              to="/register"
              className="text-primary-600 hover:text-primary-700 dark:text-primary-400 font-medium"
            >
              Jetzt registrieren
            </Link>
          </p>
        </motion.div>
      </div>
    </div>
  );
}
