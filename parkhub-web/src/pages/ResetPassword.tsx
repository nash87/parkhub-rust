import { useState, useEffect } from 'react';
import { useSearchParams, useNavigate, Link } from 'react-router-dom';
import { motion } from 'framer-motion';
import { Car, Eye, EyeSlash, Lock, ArrowRight, SpinnerGap, Warning } from '@phosphor-icons/react';
import toast from 'react-hot-toast';

export function ResetPasswordPage() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const token = searchParams.get('token');

  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [loading, setLoading] = useState(false);
  const [passwordError, setPasswordError] = useState<string | null>(null);

  useEffect(() => {
    if (!token) {
      toast.error('Ungültiger Reset-Link');
    }
  }, [token]);

  function validatePassword(pw: string): string | null {
    if (pw.length < 8) return 'Passwort muss mindestens 8 Zeichen haben';
    if (!/[a-z]/.test(pw)) return 'Passwort muss einen Kleinbuchstaben enthalten';
    if (!/[A-Z]/.test(pw)) return 'Passwort muss einen Großbuchstaben enthalten';
    if (!/[0-9]/.test(pw)) return 'Passwort muss eine Ziffer enthalten';
    return null;
  }

  function handlePasswordChange(pw: string) {
    setPassword(pw);
    if (pw) {
      setPasswordError(validatePassword(pw));
    } else {
      setPasswordError(null);
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();

    if (!token) {
      toast.error('Kein Reset-Token vorhanden. Bitte fordern Sie einen neuen Link an.');
      return;
    }

    const pwError = validatePassword(password);
    if (pwError) {
      toast.error(pwError);
      return;
    }

    if (password !== confirmPassword) {
      toast.error('Passwörter stimmen nicht überein');
      return;
    }

    setLoading(true);
    try {
      const response = await fetch('/api/v1/auth/reset-password', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ token, password }),
      });

      const data = await response.json();

      if (response.ok && data.success) {
        toast.success('Passwort erfolgreich geändert! Bitte melden Sie sich an.');
        navigate('/login', { replace: true });
      } else {
        const errorCode = data.error?.code;
        if (errorCode === 'TOKEN_EXPIRED') {
          toast.error('Der Reset-Link ist abgelaufen. Bitte fordern Sie einen neuen an.');
        } else if (errorCode === 'INVALID_TOKEN') {
          toast.error('Ungültiger Reset-Link. Bitte fordern Sie einen neuen an.');
        } else {
          toast.error(data.error?.message || 'Fehler beim Zurücksetzen des Passworts');
        }
      }
    } catch {
      toast.error('Netzwerkfehler. Bitte überprüfen Sie Ihre Verbindung.');
    } finally {
      setLoading(false);
    }
  }

  if (!token) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 dark:bg-gray-950 px-6">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          className="text-center max-w-md"
        >
          <div className="w-16 h-16 bg-red-100 dark:bg-red-900/30 rounded-2xl flex items-center justify-center mx-auto mb-6">
            <Warning weight="fill" className="w-8 h-8 text-red-600 dark:text-red-400" />
          </div>
          <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-3">
            Ungültiger Reset-Link
          </h2>
          <p className="text-gray-500 dark:text-gray-400 mb-8">
            Dieser Link ist ungültig oder abgelaufen. Bitte fordern Sie einen neuen Passwort-Reset an.
          </p>
          <Link to="/forgot-password" className="btn btn-primary w-full justify-center">
            Neuen Reset-Link anfordern
          </Link>
        </motion.div>
      </div>
    );
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
            Neues Passwort festlegen
          </h1>
          <p className="text-lg text-white/80">
            Wählen Sie ein sicheres Passwort für Ihr Konto.
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
          {/* Mobile Logo */}
          <div className="lg:hidden flex items-center gap-3 mb-8">
            <div className="w-12 h-12 bg-primary-600 rounded-2xl flex items-center justify-center">
              <Car weight="fill" className="w-7 h-7 text-white" />
            </div>
            <span className="text-2xl font-bold text-gray-900 dark:text-white">ParkHub</span>
          </div>

          <div className="mb-8">
            <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
              Neues Passwort festlegen
            </h2>
            <p className="text-gray-500 dark:text-gray-400 mt-2">
              Geben Sie Ihr neues Passwort ein. Es muss mindestens 8 Zeichen haben und Groß-, Kleinbuchstaben sowie eine Ziffer enthalten.
            </p>
          </div>

          <form onSubmit={handleSubmit} className="space-y-5">
            <div>
              <label htmlFor="new-password" className="label">
                Neues Passwort
              </label>
              <div className="relative">
                <Lock weight="regular" className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" aria-hidden="true" />
                <input
                  id="new-password"
                  type={showPassword ? 'text' : 'password'}
                  value={password}
                  onChange={(e) => handlePasswordChange(e.target.value)}
                  className={`input pl-11 pr-12 ${passwordError && password ? 'border-red-400 focus:ring-red-500' : ''}`}
                  placeholder="••••••••"
                  required
                  autoFocus
                  autoComplete="new-password"
                />
                <button
                  type="button"
                  onClick={() => setShowPassword(!showPassword)}
                  aria-label={showPassword ? 'Passwort verbergen' : 'Passwort anzeigen'}
                  aria-pressed={showPassword}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 focus:outline-none focus:ring-2 focus:ring-primary-500 rounded"
                >
                  {showPassword ? (
                    <EyeSlash weight="regular" className="w-5 h-5" aria-hidden="true" />
                  ) : (
                    <Eye weight="regular" className="w-5 h-5" aria-hidden="true" />
                  )}
                </button>
              </div>
              {passwordError && password && (
                <p className="mt-1.5 text-sm text-red-600 dark:text-red-400" role="alert">
                  {passwordError}
                </p>
              )}
            </div>

            <div>
              <label htmlFor="confirm-password" className="label">
                Passwort bestätigen
              </label>
              <div className="relative">
                <Lock weight="regular" className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" aria-hidden="true" />
                <input
                  id="confirm-password"
                  type={showPassword ? 'text' : 'password'}
                  value={confirmPassword}
                  onChange={(e) => setConfirmPassword(e.target.value)}
                  className={`input pl-11 ${confirmPassword && password !== confirmPassword ? 'border-red-400 focus:ring-red-500' : ''}`}
                  placeholder="••••••••"
                  required
                  autoComplete="new-password"
                />
              </div>
              {confirmPassword && password !== confirmPassword && (
                <p className="mt-1.5 text-sm text-red-600 dark:text-red-400" role="alert">
                  Passwörter stimmen nicht überein
                </p>
              )}
            </div>

            <button
              type="submit"
              disabled={loading || !!passwordError || (!!confirmPassword && password !== confirmPassword)}
              aria-busy={loading}
              className="btn btn-primary w-full justify-center disabled:opacity-60"
            >
              {loading ? (
                <>
                  <SpinnerGap weight="bold" className="w-5 h-5 animate-spin" aria-hidden="true" />
                  <span>Wird gespeichert…</span>
                </>
              ) : (
                <>
                  Passwort speichern
                  <ArrowRight weight="bold" className="w-5 h-5" aria-hidden="true" />
                </>
              )}
            </button>
          </form>

          <p className="mt-8 text-center text-sm text-gray-500 dark:text-gray-400">
            <Link
              to="/forgot-password"
              className="text-primary-600 hover:text-primary-700 dark:text-primary-400 font-medium"
            >
              Neuen Reset-Link anfordern
            </Link>
          </p>
        </motion.div>
      </div>
    </div>
  );
}
