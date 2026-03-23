import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'compliance.title': 'Compliance',
        'compliance.subtitle': 'GDPR/DSGVO compliance dashboard',
        'compliance.help': 'Monitor GDPR/DSGVO compliance status and generate required documentation',
        'compliance.helpLabel': 'Help',
        'compliance.overallStatus': 'Overall Status',
        'compliance.passed': 'Passed',
        'compliance.warnings': 'Warnings',
        'compliance.failures': 'Failures',
        'compliance.downloadPdf': 'Download PDF',
        'compliance.downloadDataMap': 'Data Map',
        'compliance.auditJson': 'Audit (JSON)',
        'compliance.auditCsv': 'Audit (CSV)',
        'compliance.checksTitle': 'Compliance Checks',
        'compliance.recommendation': 'Recommendation',
        'common.error': 'Error',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  ShieldCheck: (props: any) => <span data-testid="icon-shield" {...props} />,
  Download: (props: any) => <span data-testid="icon-download" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  FileText: (props: any) => <span data-testid="icon-filetext" {...props} />,
  Table: (props: any) => <span data-testid="icon-table" {...props} />,
  Warning: (props: any) => <span data-testid="icon-warning" {...props} />,
  CheckCircle: (props: any) => <span data-testid="icon-check" {...props} />,
  XCircle: (props: any) => <span data-testid="icon-x" {...props} />,
}));

import { AdminCompliancePage } from './AdminCompliance';

const sampleReport = {
  generated_at: '2026-03-23T10:00:00Z',
  overall_status: 'warning',
  checks: [
    { id: 'encryption-at-rest', category: 'Security', name: 'Encryption at Rest', description: 'AES-256-GCM', status: 'compliant', details: 'Database encrypted', recommendation: null },
    { id: 'dpo-appointed', category: 'Organization', name: 'Data Protection Officer', description: 'DPO', status: 'warning', details: 'No DPO configured', recommendation: 'Consider appointing a DPO' },
    { id: 'data-portability', category: 'Data Subject Rights', name: 'Data Portability', description: 'Export', status: 'compliant', details: 'JSON export available', recommendation: null },
  ],
  data_categories: [],
  legal_basis: [],
  retention_periods: [],
  sub_processors: [],
  tom_summary: { encryption_at_rest: true, encryption_in_transit: true, access_control: true, audit_logging: true, data_minimization: true, backup_encryption: true, incident_response_plan: false, dpo_appointed: false, privacy_by_design: true, regular_audits: false },
};

describe('AdminCompliancePage', () => {
  beforeEach(() => {
    global.fetch = vi.fn(() =>
      Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleReport }) } as Response)
    ) as any;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the title after loading', async () => {
    render(<AdminCompliancePage />);
    await waitFor(() => {
      expect(screen.getByText('Compliance')).toBeInTheDocument();
    });
  });

  it('renders stats cards with correct counts', async () => {
    render(<AdminCompliancePage />);
    await waitFor(() => {
      expect(screen.getByTestId('compliance-stats')).toBeInTheDocument();
      expect(screen.getByText('Passed')).toBeInTheDocument();
      expect(screen.getByText('Warnings')).toBeInTheDocument();
    });
  });

  it('renders compliance checks', async () => {
    render(<AdminCompliancePage />);
    await waitFor(() => {
      const checks = screen.getAllByTestId('compliance-check');
      expect(checks).toHaveLength(3);
    });
  });

  it('shows recommendation for warning checks', async () => {
    render(<AdminCompliancePage />);
    await waitFor(() => {
      expect(screen.getByText(/Consider appointing a DPO/)).toBeInTheDocument();
    });
  });

  it('renders download buttons', async () => {
    render(<AdminCompliancePage />);
    await waitFor(() => {
      expect(screen.getByTestId('compliance-downloads')).toBeInTheDocument();
      expect(screen.getByTestId('download-pdf')).toBeInTheDocument();
      expect(screen.getByTestId('download-datamap')).toBeInTheDocument();
      expect(screen.getByTestId('download-audit-json')).toBeInTheDocument();
      expect(screen.getByTestId('download-audit-csv')).toBeInTheDocument();
    });
  });

  it('shows help text when help button clicked', async () => {
    render(<AdminCompliancePage />);
    await waitFor(() => {
      expect(screen.getByTestId('compliance-help-btn')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId('compliance-help-btn'));
    await waitFor(() => {
      expect(screen.getByTestId('compliance-help')).toBeInTheDocument();
    });
  });

  it('shows check names and categories', async () => {
    render(<AdminCompliancePage />);
    await waitFor(() => {
      expect(screen.getByText('Encryption at Rest')).toBeInTheDocument();
      expect(screen.getByText('Data Protection Officer')).toBeInTheDocument();
      expect(screen.getByText('Data Portability')).toBeInTheDocument();
    });
  });
});
