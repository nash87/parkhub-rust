import { useState, useCallback, useRef, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { getInMemoryToken } from '../api/client';

export type ExportType = 'bookings' | 'users' | 'revenue';
export interface ExportButtonProps { baseUrl?: string; }

function todayStr(): string { return new Date().toISOString().slice(0, 10); }
function thirtyDaysAgoStr(): string { const d = new Date(); d.setDate(d.getDate() - 30); return d.toISOString().slice(0, 10); }
function buildExportUrl(base: string, type: ExportType, from: string, to: string): string {
  const p = new URLSearchParams(); if (from) p.set('from', from); if (to) p.set('to', to);
  const qs = p.toString(); return `${base}/api/v1/admin/export/${type}${qs ? `?${qs}` : ''}`;
}

export function ExportButton({ baseUrl = '' }: ExportButtonProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [from, setFrom] = useState(thirtyDaysAgoStr);
  const [to, setTo] = useState(todayStr);
  const [loading, setLoading] = useState<ExportType | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function h(e: MouseEvent) { if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false); }
    if (open) { document.addEventListener('mousedown', h); return () => document.removeEventListener('mousedown', h); }
  }, [open]);

  const handleExport = useCallback(async (type: ExportType) => {
    setLoading(type);
    try {
      const token = getInMemoryToken();
      const url = buildExportUrl(baseUrl, type, from, to);
      const res = await fetch(url, {
        credentials: 'include',
        headers: {
          'X-Requested-With': 'XMLHttpRequest',
          ...(token ? { Authorization: `Bearer ${token}` } : {}),
        },
      });
      if (!res.ok) { const text = await res.text(); throw new Error(text || `HTTP ${res.status}`); }
      const blob = await res.blob();
      const blobUrl = URL.createObjectURL(blob);
      const a = document.createElement('a'); a.href = blobUrl; a.download = `${type}.csv`;
      document.body.appendChild(a); a.click(); document.body.removeChild(a);
      URL.revokeObjectURL(blobUrl); setOpen(false);
    } catch (err) { console.error(`Export ${type} failed:`, err); }
    finally { setLoading(null); }
  }, [baseUrl, from, to]);

  const opts: { type: ExportType; label: string }[] = [
    { type: 'bookings', label: t('export.bookings', 'Bookings') },
    { type: 'users', label: t('export.users', 'Users') },
    { type: 'revenue', label: t('export.revenue', 'Revenue') },
  ];

  return (
    <div ref={ref} className="relative inline-block" data-testid="export-button-container">
      <button type="button" onClick={() => setOpen(p => !p)}
        className="inline-flex items-center gap-2 rounded-lg bg-neutral-800 px-4 py-2 text-sm font-medium text-white hover:bg-neutral-700 transition-colors"
        aria-haspopup="true" aria-expanded={open} data-testid="export-toggle">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /><line x1="12" y1="15" x2="12" y2="3" />
        </svg>
        {t('export.button', 'Export CSV')}
      </button>
      {open && (
        <div className="absolute right-0 z-50 mt-2 w-72 rounded-xl border border-neutral-700 bg-neutral-900 p-4 shadow-xl" role="menu" data-testid="export-dropdown">
          <div className="mb-3 space-y-2">
            <label className="block text-xs font-medium text-neutral-400">{t('export.from', 'From')}
              <input type="date" value={from} onChange={e => setFrom(e.target.value)} className="mt-1 block w-full rounded-md border border-neutral-600 bg-neutral-800 px-3 py-1.5 text-sm text-white" data-testid="export-from" />
            </label>
            <label className="block text-xs font-medium text-neutral-400">{t('export.to', 'To')}
              <input type="date" value={to} onChange={e => setTo(e.target.value)} className="mt-1 block w-full rounded-md border border-neutral-600 bg-neutral-800 px-3 py-1.5 text-sm text-white" data-testid="export-to" />
            </label>
          </div>
          <div className="space-y-1">
            {opts.map(({ type, label }) => (
              <button key={type} type="button" role="menuitem" disabled={loading !== null} onClick={() => handleExport(type)}
                className="flex w-full items-center gap-2 rounded-lg px-3 py-2 text-sm text-neutral-200 hover:bg-neutral-800 disabled:opacity-50 transition-colors" data-testid={`export-${type}`}>
                {loading === type
                  ? <span className="inline-block h-4 w-4 animate-spin rounded-full border-2 border-neutral-500 border-t-white" />
                  : <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" /><polyline points="14 2 14 8 20 8" /></svg>}
                {label}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
export default ExportButton;
