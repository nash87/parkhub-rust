import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion, AnimatePresence } from 'framer-motion';
import {
  User,
  Envelope,
  IdentificationBadge,
  ShieldCheck,
  DownloadSimple,
  Trash,
  Warning,
  SpinnerGap,
  X,
} from '@phosphor-icons/react';
import { api } from '../api/client';
import { useAuth } from '../context/AuthContext';
import toast from 'react-hot-toast';

export function ProfilePage() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const [exporting, setExporting] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [deleteConfirmText, setDeleteConfirmText] = useState('');
  const [deleting, setDeleting] = useState(false);

  async function handleExportData() {
    setExporting(true);
    try {
      const res = await api.exportUserData();
      if (res.success && res.data) {
        // Trigger file download
        const blob = new Blob([JSON.stringify(res.data, null, 2)], {
          type: 'application/json',
        });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `parkhub-daten-${user?.username ?? 'export'}-${new Date().toISOString().slice(0, 10)}.json`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        toast.success('Daten erfolgreich exportiert');
      } else {
        toast.error(res.error?.message ?? 'Export fehlgeschlagen');
      }
    } finally {
      setExporting(false);
    }
  }

  async function handleDeleteAccount() {
    if (deleteConfirmText !== user?.username) return;
    setDeleting(true);
    try {
      const res = await api.deleteAccount();
      if (res.success) {
        toast.success('Konto wurde gelöscht');
        logout();
        navigate('/login', { replace: true });
      } else {
        toast.error(res.error?.message ?? 'Löschung fehlgeschlagen');
        setDeleting(false);
      }
    } catch {
      toast.error('Löschung fehlgeschlagen');
      setDeleting(false);
    }
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="max-w-2xl mx-auto space-y-8"
    >
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
          Mein Profil
        </h1>
        <p className="text-gray-500 dark:text-gray-400 mt-1">
          Kontoinformationen und Datenschutz-Einstellungen
        </p>
      </div>

      {/* Profile Info */}
      <div className="card p-6">
        <div className="flex items-center gap-3 mb-6">
          <User weight="fill" className="w-5 h-5 text-primary-600" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            Profildaten
          </h2>
        </div>

        <div className="flex items-center gap-4 mb-6">
          <div className="w-16 h-16 rounded-2xl bg-primary-100 dark:bg-primary-900/30 flex items-center justify-center">
            <span className="text-2xl font-bold text-primary-600 dark:text-primary-400">
              {user?.name?.charAt(0).toUpperCase()}
            </span>
          </div>
          <div>
            <p className="font-semibold text-gray-900 dark:text-white text-lg">{user?.name}</p>
            <span className={`badge ${
              user?.role === 'superadmin'
                ? 'badge-error'
                : user?.role === 'admin'
                  ? 'badge-warning'
                  : 'badge-info'
            }`}>
              {user?.role === 'superadmin' ? 'Super-Admin' : user?.role === 'admin' ? 'Admin' : 'Benutzer'}
            </span>
          </div>
        </div>

        <div className="space-y-3">
          <div className="flex items-center gap-3 p-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl">
            <IdentificationBadge weight="fill" className="w-5 h-5 text-gray-400 shrink-0" aria-hidden="true" />
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">Benutzername</p>
              <p className="text-sm font-medium text-gray-900 dark:text-white">{user?.username}</p>
            </div>
          </div>
          <div className="flex items-center gap-3 p-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl">
            <Envelope weight="fill" className="w-5 h-5 text-gray-400 shrink-0" aria-hidden="true" />
            <div>
              <p className="text-xs text-gray-500 dark:text-gray-400">E-Mail</p>
              <p className="text-sm font-medium text-gray-900 dark:text-white">{user?.email}</p>
            </div>
          </div>
          {user?.created_at && (
            <div className="flex items-center gap-3 p-3 bg-gray-50 dark:bg-gray-800/50 rounded-xl">
              <User weight="fill" className="w-5 h-5 text-gray-400 shrink-0" aria-hidden="true" />
              <div>
                <p className="text-xs text-gray-500 dark:text-gray-400">Mitglied seit</p>
                <p className="text-sm font-medium text-gray-900 dark:text-white">
                  {new Date(user.created_at).toLocaleDateString('de-DE', {
                    day: 'numeric',
                    month: 'long',
                    year: 'numeric',
                  })}
                </p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* GDPR / Data Privacy Section */}
      <div className="card p-6">
        <div className="flex items-center gap-3 mb-2">
          <ShieldCheck weight="fill" className="w-5 h-5 text-primary-600" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            Datenschutz
          </h2>
        </div>
        <p className="text-sm text-gray-500 dark:text-gray-400 mb-6">
          Gemäß DSGVO Art. 15–20 haben Sie das Recht auf Auskunft, Datenübertragbarkeit und Löschung
          Ihrer personenbezogenen Daten.
        </p>

        <div className="space-y-4">
          {/* Export */}
          <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 p-4 bg-gray-50 dark:bg-gray-800/50 rounded-xl">
            <div>
              <p className="font-medium text-gray-900 dark:text-white">
                Meine Daten exportieren
              </p>
              <p className="text-sm text-gray-500 dark:text-gray-400">
                Laden Sie alle über Sie gespeicherten Daten als JSON-Datei herunter (Art. 15 &amp; 20 DSGVO)
              </p>
            </div>
            <button
              onClick={handleExportData}
              disabled={exporting}
              aria-busy={exporting}
              className="btn btn-secondary shrink-0"
            >
              {exporting ? (
                <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" aria-hidden="true" />
              ) : (
                <DownloadSimple weight="bold" className="w-4 h-4" aria-hidden="true" />
              )}
              {exporting ? 'Exportiere…' : 'Meine Daten exportieren'}
            </button>
          </div>

          {/* Delete account */}
          <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 p-4 bg-red-50 dark:bg-red-900/10 rounded-xl border border-red-100 dark:border-red-900/30">
            <div>
              <p className="font-medium text-red-700 dark:text-red-400">
                Konto löschen
              </p>
              <p className="text-sm text-red-600/70 dark:text-red-400/70">
                Löscht Ihr Konto und anonymisiert alle Buchungsdaten (Art. 17 DSGVO)
              </p>
            </div>
            <button
              onClick={() => setShowDeleteConfirm(true)}
              className="btn btn-danger shrink-0"
            >
              <Trash weight="bold" className="w-4 h-4" aria-hidden="true" />
              Konto löschen
            </button>
          </div>
        </div>
      </div>

      {/* Legal links */}
      <div className="card p-4 bg-gray-50 dark:bg-gray-800/50">
        <p className="text-sm text-gray-500 dark:text-gray-400">
          Weitere Informationen finden Sie in unserer{' '}
          <a href="/datenschutz" className="text-primary-600 hover:underline dark:text-primary-400">
            Datenschutzerklärung
          </a>{' '}
          und den{' '}
          <a href="/agb" className="text-primary-600 hover:underline dark:text-primary-400">
            AGB
          </a>.
        </p>
      </div>

      {/* Delete Confirmation Modal */}
      <AnimatePresence>
        {showDeleteConfirm && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 z-50 flex items-center justify-center p-4"
          >
            {/* Backdrop */}
            <div
              className="absolute inset-0 bg-black/50 backdrop-blur-sm"
              onClick={() => !deleting && setShowDeleteConfirm(false)}
              aria-hidden="true"
            />

            {/* Dialog */}
            <motion.div
              initial={{ scale: 0.95, opacity: 0 }}
              animate={{ scale: 1, opacity: 1 }}
              exit={{ scale: 0.95, opacity: 0 }}
              role="dialog"
              aria-modal="true"
              aria-labelledby="delete-dialog-title"
              className="relative card p-6 w-full max-w-md shadow-2xl"
            >
              <button
                onClick={() => setShowDeleteConfirm(false)}
                disabled={deleting}
                className="absolute top-4 right-4 btn btn-ghost btn-icon"
                aria-label="Abbrechen"
              >
                <X weight="bold" className="w-4 h-4" aria-hidden="true" />
              </button>

              <div className="flex items-center gap-3 mb-4">
                <div className="w-10 h-10 bg-red-100 dark:bg-red-900/30 rounded-xl flex items-center justify-center">
                  <Warning weight="fill" className="w-5 h-5 text-red-600 dark:text-red-400" aria-hidden="true" />
                </div>
                <h3 id="delete-dialog-title" className="text-lg font-semibold text-gray-900 dark:text-white">
                  Konto unwiderruflich löschen
                </h3>
              </div>

              <div className="space-y-4 mb-6">
                <p className="text-sm text-gray-600 dark:text-gray-300">
                  Diese Aktion kann <span className="font-semibold">nicht rückgängig</span> gemacht werden.
                  Ihr Konto, Ihre Fahrzeuge und Ihre persönlichen Daten werden dauerhaft gelöscht.
                </p>
                <div className="bg-amber-50 dark:bg-amber-900/20 rounded-xl p-4 text-sm text-amber-700 dark:text-amber-400">
                  <p className="font-medium mb-1">Was passiert mit Ihren Buchungen?</p>
                  <p>
                    Vergangene Buchungen werden aus steuerrechtlichen Gründen anonymisiert (Kennzeichen
                    und Personendaten werden entfernt), aber nicht vollständig gelöscht.
                  </p>
                </div>
                <div>
                  <label htmlFor="delete-confirm-input" className="label">
                    Zur Bestätigung Ihren Benutzernamen eingeben:{' '}
                    <span className="font-mono text-red-600 dark:text-red-400">{user?.username}</span>
                  </label>
                  <input
                    id="delete-confirm-input"
                    type="text"
                    value={deleteConfirmText}
                    onChange={(e) => setDeleteConfirmText(e.target.value)}
                    placeholder={user?.username}
                    disabled={deleting}
                    className="input"
                    autoComplete="off"
                  />
                </div>
              </div>

              <div className="flex gap-3">
                <button
                  onClick={() => { setShowDeleteConfirm(false); setDeleteConfirmText(''); }}
                  disabled={deleting}
                  className="btn btn-secondary flex-1"
                >
                  Abbrechen
                </button>
                <button
                  onClick={handleDeleteAccount}
                  disabled={deleteConfirmText !== user?.username || deleting}
                  aria-busy={deleting}
                  className="btn btn-danger flex-1"
                >
                  {deleting ? (
                    <SpinnerGap weight="bold" className="w-4 h-4 animate-spin" aria-hidden="true" />
                  ) : (
                    <Trash weight="bold" className="w-4 h-4" aria-hidden="true" />
                  )}
                  {deleting ? 'Wird gelöscht…' : 'Konto löschen'}
                </button>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </motion.div>
  );
}
