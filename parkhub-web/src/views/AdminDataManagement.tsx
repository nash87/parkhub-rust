import { useState, useRef } from 'react';
import { motion } from 'framer-motion';
import { UploadSimple, DownloadSimple, FileArrowUp, FileArrowDown, Table, Warning, CheckCircle } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

type Tab = 'import' | 'export';
type ImportType = 'users' | 'lots';
type ExportType = 'users' | 'lots' | 'bookings';

interface ImportResult {
  imported: number;
  skipped: number;
  errors: { row: number; field: string; message: string }[];
}

export function AdminDataManagementPage() {
  const { t } = useTranslation();
  const [tab, setTab] = useState<Tab>('import');

  return (
    <motion.div initial={{ opacity: 0, y: 20 }} animate={{ opacity: 1, y: 0 }} className="space-y-6">
      {/* Header */}
      <div className="flex items-center gap-3">
        <Table weight="duotone" className="w-6 h-6 text-primary-500" />
        <div>
          <h2 className="text-xl font-bold text-surface-900 dark:text-white">{t('dataManagement.title', 'Data Management')}</h2>
          <p className="text-sm text-surface-500">{t('dataManagement.subtitle', 'Import and export your ParkHub data')}</p>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2" data-testid="data-tabs">
        <button
          onClick={() => setTab('import')}
          className={`flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-medium transition-colors ${
            tab === 'import'
              ? 'bg-primary-600 text-white'
              : 'bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-300 hover:bg-surface-200 dark:hover:bg-surface-700'
          }`}
          data-testid="tab-import"
        >
          <UploadSimple weight="bold" className="w-4 h-4" />
          {t('dataManagement.import', 'Import')}
        </button>
        <button
          onClick={() => setTab('export')}
          className={`flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-medium transition-colors ${
            tab === 'export'
              ? 'bg-primary-600 text-white'
              : 'bg-surface-100 dark:bg-surface-800 text-surface-600 dark:text-surface-300 hover:bg-surface-200 dark:hover:bg-surface-700'
          }`}
          data-testid="tab-export"
        >
          <DownloadSimple weight="bold" className="w-4 h-4" />
          {t('dataManagement.export', 'Export')}
        </button>
      </div>

      {tab === 'import' ? <ImportSection /> : <ExportSection />}
    </motion.div>
  );
}

function ImportSection() {
  const { t } = useTranslation();
  const [importType, setImportType] = useState<ImportType>('users');
  const [file, setFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<string[][]>([]);
  const [importing, setImporting] = useState(false);
  const [result, setResult] = useState<ImportResult | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);

  async function handleFileSelect(f: File) {
    setFile(f);
    setResult(null);
    const text = await f.text();
    const lines = text.split('\n').filter(l => l.trim()).slice(0, 6);
    setPreview(lines.map(l => l.split(',')));
  }

  function handleDrop(e: React.DragEvent) {
    e.preventDefault();
    const f = e.dataTransfer.files[0];
    if (f) handleFileSelect(f);
  }

  async function handleImport() {
    if (!file) return;
    setImporting(true);
    try {
      const text = await file.text();
      const isJson = file.name.endsWith('.json');
      const data = isJson ? text : btoa(text);
      const format = isJson ? 'json' : 'csv';

      const endpoint = importType === 'users' ? '/api/v1/admin/import/users' : '/api/v1/admin/import/lots';
      const res = await fetch(endpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ format, data }),
      });
      const json = await res.json();
      if (json.success && json.data) {
        setResult(json.data);
        if (json.data.imported > 0) {
          toast.success(t('dataManagement.importSuccess', '{{count}} records imported', { count: json.data.imported }));
        }
      } else {
        toast.error(json.error || t('common.error'));
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    } finally {
      setImporting(false);
    }
  }

  return (
    <div className="space-y-4" data-testid="import-section">
      {/* Type selector */}
      <div className="flex gap-2">
        <button
          onClick={() => { setImportType('users'); setFile(null); setPreview([]); setResult(null); }}
          className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${importType === 'users' ? 'bg-primary-100 text-primary-700 dark:bg-primary-900/30 dark:text-primary-400' : 'bg-surface-100 dark:bg-surface-800 text-surface-600'}`}
        >
          {t('dataManagement.importUsers', 'Users')}
        </button>
        <button
          onClick={() => { setImportType('lots'); setFile(null); setPreview([]); setResult(null); }}
          className={`px-3 py-1.5 rounded-lg text-sm font-medium transition-colors ${importType === 'lots' ? 'bg-primary-100 text-primary-700 dark:bg-primary-900/30 dark:text-primary-400' : 'bg-surface-100 dark:bg-surface-800 text-surface-600'}`}
        >
          {t('dataManagement.importLots', 'Lots')}
        </button>
      </div>

      {/* Drop zone */}
      <div
        onDrop={handleDrop}
        onDragOver={e => e.preventDefault()}
        onClick={() => fileRef.current?.click()}
        className="border-2 border-dashed border-surface-300 dark:border-surface-600 rounded-2xl p-8 text-center cursor-pointer hover:border-primary-400 transition-colors"
        data-testid="drop-zone"
      >
        <FileArrowUp weight="duotone" className="w-12 h-12 mx-auto text-surface-400 mb-3" />
        <p className="text-sm text-surface-600 dark:text-surface-300 font-medium">
          {file ? file.name : t('dataManagement.dropHint', 'Drop a CSV or JSON file here, or click to browse')}
        </p>
        <p className="text-xs text-surface-400 mt-1">
          {importType === 'users'
            ? t('dataManagement.usersFormat', 'CSV: username, email, name, role, password')
            : t('dataManagement.lotsFormat', 'CSV: name, address, total_slots, hourly_rate, daily_max, currency')}
        </p>
        <input
          ref={fileRef}
          type="file"
          accept=".csv,.json"
          className="hidden"
          onChange={e => e.target.files?.[0] && handleFileSelect(e.target.files[0])}
        />
      </div>

      {/* Preview */}
      {preview.length > 0 && (
        <div className="glass-card rounded-2xl overflow-hidden" data-testid="import-preview">
          <div className="px-4 py-2 bg-surface-100 dark:bg-surface-800 text-sm font-medium text-surface-600">
            {t('dataManagement.preview', 'Preview')} ({preview.length} {t('dataManagement.rows', 'rows')})
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <tbody>
                {preview.map((row, i) => (
                  <tr key={i} className={i === 0 ? 'font-medium bg-surface-50 dark:bg-surface-800/50' : 'border-t border-surface-100 dark:border-surface-800'}>
                    {row.map((cell, j) => (
                      <td key={j} className="px-3 py-1.5 text-surface-600 dark:text-surface-300">{cell.trim()}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Import button */}
      {file && (
        <button
          onClick={handleImport}
          disabled={importing}
          className="flex items-center gap-2 px-6 py-2.5 bg-primary-600 text-white rounded-xl font-medium hover:bg-primary-700 disabled:opacity-50 transition-colors"
          data-testid="import-btn"
        >
          {importing ? t('common.loading') : t('dataManagement.importNow', 'Import Now')}
        </button>
      )}

      {/* Result */}
      {result && (
        <div className="glass-card rounded-2xl p-4 space-y-2" data-testid="import-result">
          <div className="flex items-center gap-2">
            <CheckCircle weight="fill" className="w-5 h-5 text-emerald-500" />
            <span className="font-medium text-surface-900 dark:text-white">
              {t('dataManagement.importComplete', 'Import Complete')}
            </span>
          </div>
          <div className="grid grid-cols-3 gap-4 text-sm">
            <div>
              <span className="text-surface-500">{t('dataManagement.imported', 'Imported')}</span>
              <p className="text-lg font-bold text-emerald-600">{result.imported}</p>
            </div>
            <div>
              <span className="text-surface-500">{t('dataManagement.skipped', 'Skipped')}</span>
              <p className="text-lg font-bold text-amber-600">{result.skipped}</p>
            </div>
            <div>
              <span className="text-surface-500">{t('dataManagement.errorsCount', 'Errors')}</span>
              <p className="text-lg font-bold text-red-600">{result.errors.length}</p>
            </div>
          </div>
          {result.errors.length > 0 && (
            <div className="space-y-1 mt-2">
              {result.errors.slice(0, 10).map((err, i) => (
                <div key={i} className="flex items-start gap-2 text-xs text-red-600">
                  <Warning weight="fill" className="w-3 h-3 mt-0.5 flex-shrink-0" />
                  <span>Row {err.row}{err.field ? ` (${err.field})` : ''}: {err.message}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function ExportSection() {
  const { t } = useTranslation();
  const [dateFrom, setDateFrom] = useState('');
  const [dateTo, setDateTo] = useState('');

  function handleExport(type: ExportType) {
    const params = new URLSearchParams();
    if (dateFrom) params.set('from', dateFrom);
    if (dateTo) params.set('to', dateTo);
    const qs = params.toString();
    const url = `/api/v1/admin/data/export/${type}${qs ? `?${qs}` : ''}`;
    window.open(url, '_blank');
  }

  const exportCards: { type: ExportType; title: string; desc: string }[] = [
    { type: 'users', title: t('dataManagement.exportUsers', 'Export Users'), desc: t('dataManagement.exportUsersDesc', 'All users with booking stats') },
    { type: 'lots', title: t('dataManagement.exportLots', 'Export Lots'), desc: t('dataManagement.exportLotsDesc', 'All parking lots with statistics') },
    { type: 'bookings', title: t('dataManagement.exportBookings', 'Export Bookings'), desc: t('dataManagement.exportBookingsDesc', 'Bookings with date range filter') },
  ];

  return (
    <div className="space-y-4" data-testid="export-section">
      {/* Date range for bookings */}
      <div className="flex gap-3 items-end">
        <div>
          <label className="text-xs text-surface-500 mb-1 block">{t('dataManagement.dateFrom', 'From')}</label>
          <input type="date" value={dateFrom} onChange={e => setDateFrom(e.target.value)} className="input-field text-sm" />
        </div>
        <div>
          <label className="text-xs text-surface-500 mb-1 block">{t('dataManagement.dateTo', 'To')}</label>
          <input type="date" value={dateTo} onChange={e => setDateTo(e.target.value)} className="input-field text-sm" />
        </div>
      </div>

      {/* Export cards */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        {exportCards.map(card => (
          <div key={card.type} className="glass-card rounded-2xl p-5 space-y-3" data-testid={`export-card-${card.type}`}>
            <FileArrowDown weight="duotone" className="w-8 h-8 text-primary-500" />
            <div>
              <h3 className="font-medium text-surface-900 dark:text-white">{card.title}</h3>
              <p className="text-xs text-surface-500 mt-0.5">{card.desc}</p>
            </div>
            <button
              onClick={() => handleExport(card.type)}
              className="flex items-center gap-2 px-4 py-2 bg-primary-600 text-white rounded-xl text-sm font-medium hover:bg-primary-700 transition-colors w-full justify-center"
            >
              <DownloadSimple weight="bold" className="w-4 h-4" />
              CSV
            </button>
          </div>
        ))}
      </div>
    </div>
  );
}
