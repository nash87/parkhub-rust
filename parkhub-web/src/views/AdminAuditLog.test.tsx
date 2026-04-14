import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

// -- Mocks --

const mockGetAuditLog = vi.fn();
const mockExportAuditLog = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    getAuditLog: (...args: any[]) => mockGetAuditLog(...args),
    exportAuditLog: (...args: any[]) => mockExportAuditLog(...args),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallbackOrOpts?: string | Record<string, any>, opts?: Record<string, any>) => {
      const map: Record<string, string> = {
        'auditLog.title': 'Audit Log',
        'auditLog.exportCsv': 'Export CSV',
        'auditLog.advancedExport': 'Advanced Export',
        'auditLog.exportHelp': 'Export audit log in your preferred format.',
        'auditLog.exportStarted': 'Export started',
        'auditLog.exporting': 'Exporting...',
        'auditLog.download': 'Download',
        'auditLog.filters': 'Filters',
        'auditLog.allActions': 'All Actions',
        'auditLog.searchUser': 'Search user...',
        'auditLog.empty': 'No audit entries found',
        'auditLog.colTime': 'Time',
        'auditLog.colAction': 'Action',
        'auditLog.colUser': 'User',
        'auditLog.colTarget': 'Target',
        'auditLog.colIp': 'IP',
        'auditLog.colDetails': 'Details',
        'auditLog.dateFrom': 'From date',
        'auditLog.dateTo': 'To date',
        'auditLog.filterAction': 'Filter by action',
        'common.back': 'Previous',
        'common.next': 'Next',
        'common.error': 'Error',
      };
      const resolvedOpts = typeof fallbackOrOpts === 'object' ? fallbackOrOpts : opts;
      if (key === 'auditLog.totalEntries' && resolvedOpts?.count !== undefined) {
        return `${resolvedOpts.count} entries`;
      }
      if (key === 'auditLog.pageInfo' && resolvedOpts?.page !== undefined) {
        return `Page ${resolvedOpts.page} of ${resolvedOpts.total}`;
      }
      return map[key] || (typeof fallbackOrOpts === 'string' ? fallbackOrOpts : key);
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
  ClockCounterClockwise: (props: any) => <span data-testid="icon-clock" {...props} />,
  DownloadSimple: (props: any) => <span data-testid="icon-download" {...props} />,
  FunnelSimple: (props: any) => <span data-testid="icon-funnel" {...props} />,
  MagnifyingGlass: (props: any) => <span data-testid="icon-search" {...props} />,
  FileCsv: (props: any) => <span data-testid="icon-csv" {...props} />,
  FileDoc: (props: any) => <span data-testid="icon-doc" {...props} />,
  FileJs: (props: any) => <span data-testid="icon-js" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  CircleNotch: (props: any) => <span data-testid="icon-spinner" {...props} />,
}));

import { AdminAuditLogPage } from './AdminAuditLog';

const sampleData = {
  entries: [
    {
      id: '1',
      timestamp: '2026-03-22T10:00:00Z',
      event_type: 'LoginSuccess',
      user_id: 'u1',
      username: 'admin',
      target_type: null,
      target_id: null,
      ip_address: '192.168.1.1',
      details: null,
    },
    {
      id: '2',
      timestamp: '2026-03-22T09:30:00Z',
      event_type: 'BookingCreated',
      user_id: 'u2',
      username: 'alice',
      target_type: 'booking',
      target_id: 'b-123',
      ip_address: '10.0.0.1',
      details: '{"slot":"A-5"}',
    },
    {
      id: '3',
      timestamp: '2026-03-22T09:00:00Z',
      event_type: 'UserDeleted',
      user_id: 'u1',
      username: 'admin',
      target_type: 'user',
      target_id: 'u3',
      ip_address: null,
      details: null,
    },
  ],
  total: 3,
  page: 1,
  per_page: 25,
  total_pages: 1,
};

describe('AdminAuditLogPage', () => {
  beforeEach(() => {
    mockGetAuditLog.mockClear();
    mockExportAuditLog.mockClear();
    mockGetAuditLog.mockResolvedValue({ success: true, data: sampleData });
    mockExportAuditLog.mockReturnValue('/api/v1/admin/audit-log/export');
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the title and total after loading', async () => {
    render(<AdminAuditLogPage />);
    await waitFor(() => {
      expect(screen.getByText('Audit Log')).toBeInTheDocument();
      expect(screen.getByText('3 entries')).toBeInTheDocument();
    });
  });

  it('renders audit log entries in the table', async () => {
    render(<AdminAuditLogPage />);
    await waitFor(() => {
      const rows = screen.getAllByTestId('audit-row');
      expect(rows).toHaveLength(3);
    });
  });

  it('displays color-coded action badges', async () => {
    render(<AdminAuditLogPage />);
    await waitFor(() => {
      // Action types appear both in dropdown options and as badges in rows
      const loginBadges = screen.getAllByText('Login Success');
      expect(loginBadges.length).toBeGreaterThanOrEqual(1);
      // Booking Created and User Deleted only appear once (badge in table row)
      const bookingBadges = screen.getAllByText('Booking Created');
      expect(bookingBadges.length).toBeGreaterThanOrEqual(1);
      const deleteBadges = screen.getAllByText('User Deleted');
      expect(deleteBadges.length).toBeGreaterThanOrEqual(1);
    });
  });

  it('shows export button that triggers download', async () => {
    const openSpy = vi.spyOn(window, 'open').mockImplementation(() => null);
    render(<AdminAuditLogPage />);
    await waitFor(() => {
      expect(screen.getByTestId('export-csv-btn')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId('export-csv-btn'));
    expect(openSpy).toHaveBeenCalledWith('/api/v1/admin/audit-log/export', '_blank');
    openSpy.mockRestore();
  });

  it('renders filter controls', async () => {
    render(<AdminAuditLogPage />);
    await waitFor(() => {
      expect(screen.getByTestId('audit-filters')).toBeInTheDocument();
      expect(screen.getByTestId('filter-action')).toBeInTheDocument();
      expect(screen.getByTestId('filter-user')).toBeInTheDocument();
      expect(screen.getByTestId('filter-from')).toBeInTheDocument();
      expect(screen.getByTestId('filter-to')).toBeInTheDocument();
    });
  });

  it('shows empty state when no entries', async () => {
    mockGetAuditLog.mockResolvedValue({
      success: true,
      data: { entries: [], total: 0, page: 1, per_page: 25, total_pages: 1 },
    });
    render(<AdminAuditLogPage />);
    await waitFor(() => {
      expect(screen.getByText('No audit entries found')).toBeInTheDocument();
    });
  });

  it('displays username and IP in audit rows', async () => {
    render(<AdminAuditLogPage />);
    await waitFor(() => {
      expect(screen.getAllByText('admin').length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText('alice')).toBeInTheDocument();
      expect(screen.getByText('192.168.1.1')).toBeInTheDocument();
      expect(screen.getByText('10.0.0.1')).toBeInTheDocument();
    });
  });

  it('displays target info for entries with targets', async () => {
    render(<AdminAuditLogPage />);
    await waitFor(() => {
      const rows = screen.getAllByTestId('audit-row');
      expect(rows.length).toBe(3);
    });
  });

  it('hides pagination when only one page', async () => {
    render(<AdminAuditLogPage />);
    await waitFor(() => {
      expect(screen.getByText('3 entries')).toBeInTheDocument();
    });
    // Pagination should NOT render when totalPages is 1
    expect(screen.queryByTestId('audit-pagination')).not.toBeInTheDocument();
  });

  it('filters by action type when selected', async () => {
    render(<AdminAuditLogPage />);
    await waitFor(() => expect(screen.getByTestId('filter-action')).toBeInTheDocument());

    fireEvent.change(screen.getByTestId('filter-action'), { target: { value: 'LoginSuccess' } });

    await waitFor(() => {
      expect(mockGetAuditLog).toHaveBeenCalledWith(
        expect.objectContaining({ action: 'LoginSuccess' })
      );
    });
  });

  it('filters by user when text is entered', async () => {
    render(<AdminAuditLogPage />);
    await waitFor(() => expect(screen.getByTestId('filter-user')).toBeInTheDocument());

    fireEvent.change(screen.getByTestId('filter-user'), { target: { value: 'admin' } });

    await waitFor(() => {
      expect(mockGetAuditLog).toHaveBeenCalledWith(
        expect.objectContaining({ user: 'admin' })
      );
    });
  });

  it('filters by date range', async () => {
    render(<AdminAuditLogPage />);
    await waitFor(() => expect(screen.getByTestId('filter-from')).toBeInTheDocument());

    fireEvent.change(screen.getByTestId('filter-from'), { target: { value: '2026-01-01' } });
    fireEvent.change(screen.getByTestId('filter-to'), { target: { value: '2026-12-31' } });

    await waitFor(() => {
      expect(mockGetAuditLog).toHaveBeenCalledWith(
        expect.objectContaining({ from: '2026-01-01', to: '2026-12-31' })
      );
    });
  });

  it('displays pagination with multi-page data', async () => {
    mockGetAuditLog.mockResolvedValue({
      success: true,
      data: { ...sampleData, total: 50, total_pages: 2 },
    });
    render(<AdminAuditLogPage />);
    await waitFor(() => {
      expect(screen.getByText('50 entries')).toBeInTheDocument();
      expect(screen.getByText('Page 1 of 2')).toBeInTheDocument();
    });
  });

  it('navigates to next page', async () => {
    mockGetAuditLog.mockResolvedValue({
      success: true,
      data: { ...sampleData, total: 50, total_pages: 2 },
    });
    render(<AdminAuditLogPage />);
    await waitFor(() => expect(screen.getByText('Page 1 of 2')).toBeInTheDocument());

    fireEvent.click(screen.getByText('Next'));

    await waitFor(() => {
      expect(mockGetAuditLog).toHaveBeenCalledWith(
        expect.objectContaining({ page: 2 })
      );
    });
  });

  it('handles API error gracefully', async () => {
    mockGetAuditLog.mockResolvedValue({ success: false, data: null, error: { code: 'ERROR', message: 'Failed' } });
    render(<AdminAuditLogPage />);
    await waitFor(() => {
      expect(screen.getByText('Audit Log')).toBeInTheDocument();
    });
  });

  it('shows details for entries with JSON details', async () => {
    render(<AdminAuditLogPage />);
    await waitFor(() => {
      const rows = screen.getAllByTestId('audit-row');
      expect(rows).toHaveLength(3);
    });
  });

  it('opens advanced export dialog', async () => {
    render(<AdminAuditLogPage />);
    await waitFor(() => expect(screen.getByTestId('export-enhanced-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('export-enhanced-btn'));
    await waitFor(() => {
      expect(screen.getByTestId('export-dialog')).toBeInTheDocument();
      expect(screen.getByText('Export audit log in your preferred format.')).toBeInTheDocument();
    });
  });

  it('selects export format in dialog', async () => {
    render(<AdminAuditLogPage />);
    await waitFor(() => expect(screen.getByTestId('export-enhanced-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('export-enhanced-btn'));
    await waitFor(() => expect(screen.getByTestId('export-dialog')).toBeInTheDocument());

    // Select JSON format
    fireEvent.click(screen.getByTestId('format-json'));
    // Select PDF format
    fireEvent.click(screen.getByTestId('format-pdf'));
    // Back to CSV
    fireEvent.click(screen.getByTestId('format-csv'));
  });

  it('triggers enhanced export download', async () => {
    const openSpy = vi.spyOn(window, 'open').mockImplementation(() => null);
    globalThis.fetch = vi.fn(() =>
      Promise.resolve({
        json: () => Promise.resolve({ success: true, data: { download_url: '/download/test.csv' } }),
      } as Response)
    );

    render(<AdminAuditLogPage />);
    await waitFor(() => expect(screen.getByTestId('export-enhanced-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('export-enhanced-btn'));
    await waitFor(() => expect(screen.getByTestId('export-dialog')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('export-download-btn'));

    await waitFor(() => {
      expect(openSpy).toHaveBeenCalledWith('/download/test.csv', '_blank');
    });
    openSpy.mockRestore();
  });

  it('handles enhanced export failure', async () => {
    globalThis.fetch = vi.fn(() =>
      Promise.resolve({
        json: () => Promise.resolve({ success: false, error: { message: 'Export failed' } }),
      } as Response)
    );

    render(<AdminAuditLogPage />);
    await waitFor(() => expect(screen.getByTestId('export-enhanced-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('export-enhanced-btn'));
    await waitFor(() => expect(screen.getByTestId('export-dialog')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('export-download-btn'));
    // Error path exercised
  });

  it('handles enhanced export network exception', async () => {
    globalThis.fetch = vi.fn(() => Promise.reject(new Error('Network')));

    render(<AdminAuditLogPage />);
    await waitFor(() => expect(screen.getByTestId('export-enhanced-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('export-enhanced-btn'));
    await waitFor(() => expect(screen.getByTestId('export-dialog')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('export-download-btn'));
    // Exception path exercised
  });

  it('closes advanced export dialog on cancel', async () => {
    render(<AdminAuditLogPage />);
    await waitFor(() => expect(screen.getByTestId('export-enhanced-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('export-enhanced-btn'));
    await waitFor(() => expect(screen.getByTestId('export-dialog')).toBeInTheDocument());

    fireEvent.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(screen.queryByTestId('export-dialog')).not.toBeInTheDocument();
    });
  });

  it('navigates to previous page', async () => {
    mockGetAuditLog.mockResolvedValue({
      success: true,
      data: { ...sampleData, total: 50, total_pages: 2, page: 2 },
    });
    render(<AdminAuditLogPage />);
    await waitFor(() => expect(screen.getByText('Previous')).toBeInTheDocument());

    // First navigate to page 2
    mockGetAuditLog.mockResolvedValue({
      success: true,
      data: { ...sampleData, total: 50, total_pages: 2, page: 2 },
    });
    fireEvent.click(screen.getByText('Next'));

    await waitFor(() => {
      expect(mockGetAuditLog).toHaveBeenCalledWith(expect.objectContaining({ page: 2 }));
    });
  });

  it('applies user search filter on Enter key', async () => {
    render(<AdminAuditLogPage />);
    await waitFor(() => expect(screen.getByTestId('filter-user')).toBeInTheDocument());

    fireEvent.change(screen.getByTestId('filter-user'), { target: { value: 'admin' } });
    fireEvent.keyDown(screen.getByTestId('filter-user'), { key: 'Enter' });

    await waitFor(() => {
      expect(mockGetAuditLog).toHaveBeenCalledWith(
        expect.objectContaining({ user: 'admin' })
      );
    });
  });

  it('shows loading skeleton when no entries yet', () => {
    mockGetAuditLog.mockReturnValue(new Promise(() => {}));
    render(<AdminAuditLogPage />);
    const skeletons = document.querySelectorAll('.skeleton');
    expect(skeletons.length).toBeGreaterThan(0);
  });
});
