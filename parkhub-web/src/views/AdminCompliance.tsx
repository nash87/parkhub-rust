import { useEffect, useState, useCallback } from 'react';
import { motion } from 'framer-motion';
import { Download, Question, FileText, Table, Warning, CheckCircle, XCircle } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import toast from 'react-hot-toast';

interface ComplianceCheck {
  id: string;
  category: string;
  name: string;
  description: string;
  status: 'compliant' | 'warning' | 'non_compliant';
  details: string;
  recommendation: string | null;
}

interface TomSummary {
  encryption_at_rest: boolean;
  encryption_in_transit: boolean;
  access_control: boolean;
  audit_logging: boolean;
  data_minimization: boolean;
  backup_encryption: boolean;
  incident_response_plan: boolean;
  dpo_appointed: boolean;
  privacy_by_design: boolean;
  regular_audits: boolean;
}

interface ComplianceReport {
  generated_at: string;
  overall_status: 'compliant' | 'warning' | 'non_compliant';
  checks: ComplianceCheck[];
  data_categories: any[];
  legal_basis: any[];
  retention_periods: any[];
  sub_processors: any[];
  tom_summary: TomSummary;
}

const statusConfig: Record<string, { color: string; icon: typeof CheckCircle; label: string }> = {
  compliant: { color: 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400', icon: CheckCircle, label: 'Compliant' },
  warning: { color: 'bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400', icon: Warning, label: 'Warning' },
  non_compliant: { color: 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400', icon: XCircle, label: 'Non-Compliant' },
};

export function AdminCompliancePage() {
  const { t } = useTranslation();
  const [report, setReport] = useState<ComplianceReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [showHelp, setShowHelp] = useState(false);

  const loadReport = useCallback(async () => {
    setLoading(true);
    try {
      const res = await fetch('/api/v1/admin/compliance/report').then(r => r.json());
      if (res.success) {
        setReport(res.data);
      }
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => { loadReport(); }, [loadReport]);

  const downloadPdf = () => {
    window.open('/api/v1/admin/compliance/report/pdf', '_blank');
  };

  const downloadDataMap = async () => {
    try {
      const res = await fetch('/api/v1/admin/compliance/data-map').then(r => r.json());
      const blob = new Blob([JSON.stringify(res.data, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'parkhub-data-map.json';
      a.click();
      URL.revokeObjectURL(url);
    /* istanbul ignore next -- network failure path */
    } catch {
      toast.error(t('common.error'));
    }
  };

  const downloadAudit = (format: 'json' | 'csv') => {
    window.open(`/api/v1/admin/compliance/audit-export?format=${format}`, '_blank');
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500" />
      </div>
    );
  }

  if (!report) return null;

  const compliantCount = report.checks.filter(c => c.status === 'compliant').length;
  const warningCount = report.checks.filter(c => c.status === 'warning').length;
  const nonCompliantCount = report.checks.filter(c => c.status === 'non_compliant').length;
  const overallConfig = statusConfig[report.overall_status] || statusConfig.warning;
  const OverallIcon = overallConfig.icon;

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-surface-900 dark:text-white">
            {t('compliance.title')}
          </h1>
          <p className="text-surface-500 dark:text-surface-400 mt-1">
            {t('compliance.subtitle')}
          </p>
        </div>
        <button
          onClick={() => setShowHelp(!showHelp)}
          className="p-2 rounded-lg hover:bg-surface-100 dark:hover:bg-surface-700"
          aria-label={t('compliance.helpLabel')}
          data-testid="compliance-help-btn"
        >
          <Question size={20} />
        </button>
      </div>

      {/* Help tooltip */}
      {showHelp && (
        <motion.div
          initial={{ opacity: 0, y: -10 }}
          animate={{ opacity: 1, y: 0 }}
          className="bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4"
          data-testid="compliance-help"
        >
          <p className="text-sm text-blue-700 dark:text-blue-300">
            {t('compliance.help')}
          </p>
        </motion.div>
      )}

      {/* Overall Status + Stats */}
      <div className="grid grid-cols-1 sm:grid-cols-4 gap-4" data-testid="compliance-stats">
        <div className={`rounded-xl p-4 shadow-sm border ${overallConfig.color}`}>
          <div className="flex items-center gap-3">
            <OverallIcon size={24} weight="fill" />
            <div>
              <p className="text-sm font-medium">{t('compliance.overallStatus')}</p>
              <p className="text-lg font-bold">{overallConfig.label}</p>
            </div>
          </div>
        </div>
        <div className="bg-white dark:bg-surface-800 rounded-xl p-4 shadow-sm border border-surface-200 dark:border-surface-700">
          <div className="flex items-center gap-3">
            <CheckCircle size={20} className="text-green-500" />
            <div>
              <p className="text-sm text-surface-500">{t('compliance.passed')}</p>
              <p className="text-xl font-bold text-surface-900 dark:text-white">{compliantCount}</p>
            </div>
          </div>
        </div>
        <div className="bg-white dark:bg-surface-800 rounded-xl p-4 shadow-sm border border-surface-200 dark:border-surface-700">
          <div className="flex items-center gap-3">
            <Warning size={20} className="text-amber-500" />
            <div>
              <p className="text-sm text-surface-500">{t('compliance.warnings')}</p>
              <p className="text-xl font-bold text-surface-900 dark:text-white">{warningCount}</p>
            </div>
          </div>
        </div>
        <div className="bg-white dark:bg-surface-800 rounded-xl p-4 shadow-sm border border-surface-200 dark:border-surface-700">
          <div className="flex items-center gap-3">
            <XCircle size={20} className="text-red-500" />
            <div>
              <p className="text-sm text-surface-500">{t('compliance.failures')}</p>
              <p className="text-xl font-bold text-surface-900 dark:text-white">{nonCompliantCount}</p>
            </div>
          </div>
        </div>
      </div>

      {/* Download Actions */}
      <div className="flex flex-wrap gap-3" data-testid="compliance-downloads">
        <button
          onClick={downloadPdf}
          className="flex items-center gap-2 px-4 py-2 rounded-lg bg-primary-500 text-white hover:bg-primary-600"
          data-testid="download-pdf"
        >
          <FileText size={16} />
          {t('compliance.downloadPdf')}
        </button>
        <button
          onClick={downloadDataMap}
          className="flex items-center gap-2 px-4 py-2 rounded-lg bg-surface-100 dark:bg-surface-700 text-surface-700 dark:text-surface-300 hover:bg-surface-200"
          data-testid="download-datamap"
        >
          <Table size={16} />
          {t('compliance.downloadDataMap')}
        </button>
        <button
          onClick={() => downloadAudit('json')}
          className="flex items-center gap-2 px-4 py-2 rounded-lg bg-surface-100 dark:bg-surface-700 text-surface-700 dark:text-surface-300 hover:bg-surface-200"
          data-testid="download-audit-json"
        >
          <Download size={16} />
          {t('compliance.auditJson')}
        </button>
        <button
          onClick={() => downloadAudit('csv')}
          className="flex items-center gap-2 px-4 py-2 rounded-lg bg-surface-100 dark:bg-surface-700 text-surface-700 dark:text-surface-300 hover:bg-surface-200"
          data-testid="download-audit-csv"
        >
          <Download size={16} />
          {t('compliance.auditCsv')}
        </button>
      </div>

      {/* Compliance Checks */}
      <div className="space-y-3" data-testid="compliance-checks">
        <h2 className="text-lg font-semibold text-surface-900 dark:text-white">
          {t('compliance.checksTitle')}
        </h2>
        {report.checks.map(check => {
          const config = statusConfig[check.status] || statusConfig.warning;
          const StatusIcon = config.icon;
          return (
            <motion.div
              key={check.id}
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              className="bg-white dark:bg-surface-800 rounded-xl p-4 shadow-sm border border-surface-200 dark:border-surface-700"
              data-testid="compliance-check"
            >
              <div className="flex items-start gap-3">
                <StatusIcon
                  size={20}
                  weight="fill"
                  className={check.status === 'compliant' ? 'text-green-500' : check.status === 'warning' ? 'text-amber-500' : 'text-red-500'}
                />
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <h3 className="font-medium text-surface-900 dark:text-white">{check.name}</h3>
                    <span className={`text-xs px-2 py-0.5 rounded-full ${config.color}`}>
                      {check.category}
                    </span>
                  </div>
                  <p className="text-sm text-surface-500 dark:text-surface-400 mt-1">{check.details}</p>
                  {check.recommendation && (
                    <p className="text-sm text-amber-600 dark:text-amber-400 mt-1">
                      {t('compliance.recommendation')}: {check.recommendation}
                    </p>
                  )}
                </div>
              </div>
            </motion.div>
          );
        })}
      </div>
    </div>
  );
}
