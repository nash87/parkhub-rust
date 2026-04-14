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

const mockToastError = vi.fn();
vi.mock('react-hot-toast', () => ({
  default: { success: vi.fn(), error: (...a: any[]) => mockToastError(...a) },
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

  it('clicks download PDF', async () => {
    const openSpy = vi.spyOn(window, 'open').mockImplementation(() => null);
    render(<AdminCompliancePage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('download-pdf')));
    expect(openSpy).toHaveBeenCalledWith(expect.stringContaining('/pdf'), '_blank');
    openSpy.mockRestore();
  });

  it('clicks download audit JSON', async () => {
    const openSpy = vi.spyOn(window, 'open').mockImplementation(() => null);
    render(<AdminCompliancePage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('download-audit-json')));
    expect(openSpy).toHaveBeenCalledWith(expect.stringContaining('format=json'), '_blank');
    openSpy.mockRestore();
  });

  it('clicks download audit CSV', async () => {
    const openSpy = vi.spyOn(window, 'open').mockImplementation(() => null);
    render(<AdminCompliancePage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('download-audit-csv')));
    expect(openSpy).toHaveBeenCalledWith(expect.stringContaining('format=csv'), '_blank');
    openSpy.mockRestore();
  });

  it('downloads data map', async () => {
    const createObjectURL = vi.fn(() => 'blob:test');
    const revokeObjectURL = vi.fn();
    Object.defineProperty(URL, 'createObjectURL', { value: createObjectURL, writable: true });
    Object.defineProperty(URL, 'revokeObjectURL', { value: revokeObjectURL, writable: true });
    render(<AdminCompliancePage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('download-datamap')));
    await waitFor(() => expect(createObjectURL).toHaveBeenCalled());
  });

  it('download data map error', async () => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/data-map')) return Promise.reject(new Error('net'));
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleReport }) } as Response);
    }) as any;
    render(<AdminCompliancePage />);
    await waitFor(() => fireEvent.click(screen.getByTestId('download-datamap')));
    await waitFor(() => expect(mockToastError).toHaveBeenCalled());
  });

  it('load error shows toast', async () => {
    global.fetch = vi.fn(() => Promise.reject(new Error('net'))) as any;
    render(<AdminCompliancePage />);
    await waitFor(() => expect(mockToastError).toHaveBeenCalled());
  });

  it('returns null when no report', async () => {
    global.fetch = vi.fn(() =>
      Promise.resolve({ json: () => Promise.resolve({ success: false, data: null }) } as Response)
    ) as any;
    const { container } = render(<AdminCompliancePage />);
    await waitFor(() => {
      // Should not render main content since report is null
      expect(container.querySelector('[data-testid="compliance-stats"]')).toBeNull();
    });
  });

  it('shows non_compliant check with icon', async () => {
    const reportWithNonCompliant = {
      ...sampleReport,
      overall_status: 'non_compliant',
      checks: [
        ...sampleReport.checks,
        { id: 'nc1', category: 'Security', name: 'Audit Logging', description: 'Audit', status: 'non_compliant', details: 'Missing', recommendation: 'Enable audit logging' },
      ],
    };
    global.fetch = vi.fn(() =>
      Promise.resolve({ json: () => Promise.resolve({ success: true, data: reportWithNonCompliant }) } as Response)
    ) as any;
    render(<AdminCompliancePage />);
    await waitFor(() => expect(screen.getByText(/Enable audit logging/)).toBeInTheDocument());
  });
});
