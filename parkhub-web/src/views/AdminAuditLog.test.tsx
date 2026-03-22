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
});
