import { useState, useEffect } from 'react';
import { ClockCounterClockwise, SpinnerGap, Desktop, Globe, ShieldWarning, Check } from '@phosphor-icons/react';
import { api, type LoginHistoryEntry, type SessionInfo } from '../api/client';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';
import { format } from 'date-fns';

function parseUserAgent(ua: string): string {
  if (ua.includes('Chrome')) return 'Chrome';
  if (ua.includes('Firefox')) return 'Firefox';
  if (ua.includes('Safari')) return 'Safari';
  if (ua.includes('Edge')) return 'Edge';
  if (ua.includes('curl')) return 'curl';
  return 'Unknown';
}

export function LoginHistoryComponent() {
  const { t } = useTranslation();
  const [history, setHistory] = useState<LoginHistoryEntry[]>([]);
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [revoking, setRevoking] = useState<string | null>(null);
  const [tab, setTab] = useState<'history' | 'sessions'>('history');

  useEffect(() => {
    Promise.all([
      api.getLoginHistory(),
      api.getSessions(),
    ]).then(([hRes, sRes]) => {
      if (hRes.success && hRes.data) setHistory(hRes.data);
      if (sRes.success && sRes.data) setSessions(sRes.data);
      setLoading(false);
    }).catch(() => setLoading(false));
  }, []);

  async function handleRevoke(sessionId: string) {
    setRevoking(sessionId);
    const res = await api.revokeSession(sessionId);
    if (res.success) {
      setSessions(prev => prev.filter(s => s.id !== sessionId));
      toast.success('Session revoked');
    } else {
      toast.error(res.error?.message || 'Failed to revoke session');
    }
    setRevoking(null);
  }

  if (loading) {
    return (
      <div className="flex items-center gap-2 p-4">
        <SpinnerGap className="animate-spin" size={20} />
        <span>Loading...</span>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2 mb-3">
        <ClockCounterClockwise size={24} weight="duotone" className="text-primary-500" />
        <h3 className="font-semibold">Security</h3>
      </div>

      {/* Tab selector */}
      <div className="flex gap-1 bg-gray-100 dark:bg-gray-800 rounded-lg p-1">
        <button
          onClick={() => setTab('history')}
          className={`flex-1 py-1.5 px-3 rounded-md text-sm font-medium transition ${tab === 'history' ? 'bg-white dark:bg-gray-700 shadow-sm' : 'text-gray-500'}`}
        >
          Login History
        </button>
        <button
          onClick={() => setTab('sessions')}
          className={`flex-1 py-1.5 px-3 rounded-md text-sm font-medium transition ${tab === 'sessions' ? 'bg-white dark:bg-gray-700 shadow-sm' : 'text-gray-500'}`}
        >
          Active Sessions ({sessions.length})
        </button>
      </div>

      {tab === 'history' && (
        <div className="space-y-2">
          {history.length === 0 ? (
            <p className="text-sm text-gray-500 text-center py-4">No login history</p>
          ) : (
            history.map((entry, i) => (
              <div key={i} className="flex items-center justify-between p-3 rounded-lg bg-gray-50 dark:bg-gray-800/50 border border-gray-100 dark:border-gray-700">
                <div className="flex items-center gap-3">
                  {entry.success ? (
                    <Check size={16} className="text-green-500" />
                  ) : (
                    <ShieldWarning size={16} className="text-red-500" />
                  )}
                  <div>
                    <p className="text-sm font-medium">
                      {entry.success ? 'Successful login' : 'Failed login attempt'}
                    </p>
                    <p className="text-xs text-gray-500">
                      {format(new Date(entry.timestamp), 'MMM dd, yyyy HH:mm')}
                    </p>
                  </div>
                </div>
                <div className="text-right text-xs text-gray-500">
                  <div className="flex items-center gap-1">
                    <Globe size={12} />
                    {entry.ip_address}
                  </div>
                  <div className="flex items-center gap-1">
                    <Desktop size={12} />
                    {parseUserAgent(entry.user_agent)}
                  </div>
                </div>
              </div>
            ))
          )}
        </div>
      )}

      {tab === 'sessions' && (
        <div className="space-y-2">
          {sessions.length === 0 ? (
            <p className="text-sm text-gray-500 text-center py-4">No active sessions</p>
          ) : (
            sessions.map(session => (
              <div key={session.id} className="flex items-center justify-between p-3 rounded-lg bg-gray-50 dark:bg-gray-800/50 border border-gray-100 dark:border-gray-700">
                <div className="flex items-center gap-3">
                  <Desktop size={18} className={session.is_current ? 'text-primary-500' : 'text-gray-400'} />
                  <div>
                    <p className="text-sm font-medium flex items-center gap-2">
                      Session {session.id}
                      {session.is_current && (
                        <span className="text-xs bg-primary-100 text-primary-700 px-1.5 py-0.5 rounded-full">Current</span>
                      )}
                    </p>
                    <p className="text-xs text-gray-500">
                      Created {format(new Date(session.created_at), 'MMM dd, HH:mm')} — Expires {format(new Date(session.expires_at), 'MMM dd, HH:mm')}
                    </p>
                  </div>
                </div>
                {!session.is_current && (
                  <button
                    onClick={() => handleRevoke(session.id)}
                    disabled={revoking === session.id}
                    className="text-xs px-3 py-1.5 bg-red-50 text-red-600 rounded-lg hover:bg-red-100 disabled:opacity-50"
                  >
                    {revoking === session.id ? <SpinnerGap className="animate-spin" size={14} /> : 'Revoke'}
                  </button>
                )}
              </div>
            ))
          )}
        </div>
      )}
    </div>
  );
}
