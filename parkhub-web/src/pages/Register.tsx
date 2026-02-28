import { useState } from 'react';
import { Navigate, Link, useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { Car, Eye, EyeSlash, ArrowRight, SpinnerGap, User, Envelope, Lock } from '@phosphor-icons/react';
import { useAuth } from '../context/AuthContext';
import toast from 'react-hot-toast';

export function RegisterPage() {
  const { register, isAuthenticated, isLoading } = useAuth();
  const navigate = useNavigate();
  const [formData, setFormData] = useState({
    email: '',
    password: '',
    confirmPassword: '',
    name: '',
  });
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

  /**
   * Validate password strength client-side.
   * Must match the server-side `validate_password_strength` rules in
   * parkhub-server/src/validation.rs:
   *   - Minimum 8 characters
   *   - At least one lowercase letter
   *   - At least one uppercase letter
   *   - At least one digit
   */
  function validatePassword(password: string): string | null {
    if (password.length < 8) return 'Passwort muss mindestens 8 Zeichen haben';
    if (!/[a-z]/.test(password)) return 'Passwort muss einen Kleinbuchstaben enthalten';
    if (!/[A-Z]/.test(password)) return 'Passwort muss einen Großbuchstaben enthalten';
    if (!/[0-9]/.test(password)) return 'Passwort muss eine Ziffer enthalten';
    return null;
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();

    if (formData.password !== formData.confirmPassword) {
      toast.error('Passwörter stimmen nicht überein');
      return;
    }

    const passwordError = validatePassword(formData.password);
    if (passwordError) {
      toast.error(passwordError);
      return;
    }

    setLoading(true);
    const success = await register({
      email: formData.email,
      password: formData.password,
      name: formData.name,
    });

    if (success) {
      toast.success('Willkommen bei ParkHub!');
      navigate('/', { replace: true });
    } else {
      toast.error('Registrierung fehlgeschlagen. Bitte prüfen Sie Ihre Eingaben.');
    }
    setLoading(false);
  }

  return (
    <div className="min-h-screen flex bg-gray-50 dark:bg-gray-950">
      {/* Left Side */}
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
            Werden Sie Teil von ParkHub
          </h1>
          <p className="text-lg text-white/80">
            Erstellen Sie Ihr Konto und beginnen Sie noch heute mit der intelligenten Parkplatzverwaltung.
          </p>
        </div>
      </div>

      {/* Right Side - Form */}
      <div className="flex-1 flex items-center justify-center px-6 py-12">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="w-full max-w-md"
        >
          <div className="lg:hidden flex items-center gap-3 mb-8">
            <div className="w-12 h-12 bg-primary-600 rounded-2xl flex items-center justify-center">
              <Car weight="fill" className="w-7 h-7 text-white" />
            </div>
            <span className="text-2xl font-bold text-gray-900 dark:text-white">ParkHub</span>
          </div>

          <div className="mb-8">
            <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
              Konto erstellen
            </h2>
            <p className="text-gray-500 dark:text-gray-400 mt-2">
              Füllen Sie das Formular aus, um loszulegen
            </p>
          </div>

          <form onSubmit={handleSubmit} className="space-y-4">
            <div>
              <label htmlFor="reg-name" className="label">Vollständiger Name</label>
              <div className="relative">
                <User weight="regular" className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" aria-hidden="true" />
                <input
                  id="reg-name"
                  type="text"
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                  className="input pl-11"
                  placeholder="Max Mustermann"
                  required
                  autoComplete="name"
                  autoFocus
                />
              </div>
            </div>

            <div>
              <label htmlFor="reg-email" className="label">E-Mail</label>
              <div className="relative">
                <Envelope weight="regular" className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" aria-hidden="true" />
                <input
                  id="reg-email"
                  type="email"
                  value={formData.email}
                  onChange={(e) => setFormData({ ...formData, email: e.target.value })}
                  className="input pl-11"
                  placeholder="max@beispiel.de"
                  required
                  autoComplete="email"
                />
              </div>
            </div>

            <div>
              <label htmlFor="reg-password" className="label">Passwort</label>
              <div className="relative">
                <Lock weight="regular" className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" aria-hidden="true" />
                <input
                  id="reg-password"
                  type={showPassword ? 'text' : 'password'}
                  value={formData.password}
                  onChange={(e) => setFormData({ ...formData, password: e.target.value })}
                  className="input pl-11 pr-12"
                  placeholder="••••••••"
                  required
                  autoComplete="new-password"
                />
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  aria-label={showPassword ? 'Passwort verbergen' : 'Passwort anzeigen'}
                  aria-pressed={showPassword}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 focus:outline-none focus:ring-2 focus:ring-primary-500 rounded"
                >
                  {showPassword ? <EyeSlash weight="regular" className="w-5 h-5" aria-hidden="true" /> : <Eye weight="regular" className="w-5 h-5" aria-hidden="true" />}
                </button>
              </div>
              <p className="mt-1.5 text-xs text-gray-500 dark:text-gray-400">
                Mindestens 8 Zeichen, Groß- und Kleinbuchstaben sowie eine Ziffer
              </p>
            </div>

            <div>
              <label htmlFor="reg-confirm-password" className="label">Passwort bestätigen</label>
              <div className="relative">
                <Lock weight="regular" className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" aria-hidden="true" />
                <input
                  id="reg-confirm-password"
                  type={showPassword ? 'text' : 'password'}
                  value={formData.confirmPassword}
                  onChange={(e) => setFormData({ ...formData, confirmPassword: e.target.value })}
                  className="input pl-11"
                  placeholder="••••••••"
                  required
                  autoComplete="new-password"
                />
              </div>
              {formData.confirmPassword && formData.password !== formData.confirmPassword && (
                <p className="mt-1.5 text-sm text-red-600 dark:text-red-400" role="alert">
                  Passwörter stimmen nicht überein
                </p>
              )}
            </div>

            <button
              type="submit"
              disabled={loading}
              aria-busy={loading}
              className="btn btn-primary w-full justify-center mt-6 disabled:opacity-60"
            >
              {loading ? (
                <>
                  <SpinnerGap weight="bold" className="w-5 h-5 animate-spin" aria-hidden="true" />
                  <span>Registrierung läuft…</span>
                </>
              ) : (
                <>
                  Registrieren
                  <ArrowRight weight="bold" className="w-5 h-5" aria-hidden="true" />
                </>
              )}
            </button>
          </form>

          <p className="mt-8 text-center text-sm text-gray-500 dark:text-gray-400">
            Bereits registriert?{' '}
            <Link to="/login" className="text-primary-600 hover:text-primary-700 font-medium">
              Jetzt anmelden
            </Link>
          </p>
        </motion.div>
      </div>
    </div>
  );
}
