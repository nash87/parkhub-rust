import { useState } from 'react';
import { Link } from 'react-router-dom';
import { motion } from 'framer-motion';
import { Car, Envelope, ArrowRight, SpinnerGap, ArrowLeft } from '@phosphor-icons/react';
import toast from 'react-hot-toast';

export function ForgotPasswordPage() {
  const [email, setEmail] = useState('');
  const [loading, setLoading] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!email.trim()) {
      toast.error('Bitte geben Sie Ihre E-Mail-Adresse ein');
      return;
    }
    setLoading(true);
    try {
      const response = await fetch('/api/v1/auth/forgot-password', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: email.trim() }),
      });
      // Always show success to prevent user enumeration
      if (response.ok || response.status === 200) {
        setSubmitted(true);
      } else {
        // Unexpected error
        toast.error('Ein unerwarteter Fehler ist aufgetreten. Bitte versuchen Sie es später erneut.');
      }
    } catch {
      toast.error('Netzwerkfehler. Bitte überprüfen Sie Ihre Verbindung.');
    } finally {
      setLoading(false);
    }
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
            Passwort zurücksetzen
          </h1>
          <p className="text-lg text-white/80">
            Geben Sie Ihre E-Mail-Adresse ein und wir senden Ihnen einen Link zum Zurücksetzen Ihres Passworts.
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

          {submitted ? (
            <motion.div
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              className="text-center"
            >
              <div className="w-16 h-16 bg-emerald-100 dark:bg-emerald-900/30 rounded-2xl flex items-center justify-center mx-auto mb-6">
                <Envelope weight="fill" className="w-8 h-8 text-emerald-600 dark:text-emerald-400" />
              </div>
              <h2 className="text-2xl font-bold text-gray-900 dark:text-white mb-3">
                E-Mail gesendet
              </h2>
              <p className="text-gray-500 dark:text-gray-400 mb-8">
                Wenn die E-Mail-Adresse <span className="font-medium text-gray-700 dark:text-gray-300">{email}</span> bekannt ist, wurde ein Reset-Link gesendet. Bitte prüfen Sie Ihren Posteingang.
              </p>
              <Link
                to="/login"
                className="btn btn-primary w-full justify-center"
              >
                <ArrowLeft weight="bold" className="w-5 h-5" />
                Zurück zur Anmeldung
              </Link>
            </motion.div>
          ) : (
            <>
              <div className="mb-8">
                <h2 className="text-2xl font-bold text-gray-900 dark:text-white">
                  Passwort vergessen?
                </h2>
                <p className="text-gray-500 dark:text-gray-400 mt-2">
                  Kein Problem. Geben Sie Ihre E-Mail-Adresse ein und wir senden Ihnen einen Reset-Link.
                </p>
              </div>

              <form onSubmit={handleSubmit} className="space-y-5">
                <div>
                  <label htmlFor="email" className="label">
                    E-Mail-Adresse
                  </label>
                  <div className="relative">
                    <Envelope weight="regular" className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-gray-400" aria-hidden="true" />
                    <input
                      id="email"
                      type="email"
                      value={email}
                      onChange={(e) => setEmail(e.target.value)}
                      className="input pl-11"
                      placeholder="max@beispiel.de"
                      required
                      autoFocus
                      autoComplete="email"
                    />
                  </div>
                </div>

                <button
                  type="submit"
                  disabled={loading}
                  aria-busy={loading}
                  className="btn btn-primary w-full justify-center disabled:opacity-60"
                >
                  {loading ? (
                    <>
                      <SpinnerGap weight="bold" className="w-5 h-5 animate-spin" aria-hidden="true" />
                      <span>Wird gesendet…</span>
                    </>
                  ) : (
                    <>
                      Reset-Link senden
                      <ArrowRight weight="bold" className="w-5 h-5" aria-hidden="true" />
                    </>
                  )}
                </button>
              </form>

              <p className="mt-8 text-center text-sm text-gray-500 dark:text-gray-400">
                <Link
                  to="/login"
                  className="flex items-center justify-center gap-1 text-primary-600 hover:text-primary-700 dark:text-primary-400 font-medium"
                >
                  <ArrowLeft weight="bold" className="w-4 h-4" />
                  Zurück zur Anmeldung
                </Link>
              </p>
            </>
          )}
        </motion.div>
      </div>
    </div>
  );
}
