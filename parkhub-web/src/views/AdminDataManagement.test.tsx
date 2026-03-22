import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallbackOrOpts?: string | Record<string, any>) => {
      const map: Record<string, string> = {
        'dataManagement.title': 'Data Management',
        'dataManagement.subtitle': 'Import and export your ParkHub data',
        'dataManagement.import': 'Import',
        'dataManagement.export': 'Export',
        'dataManagement.importUsers': 'Users',
        'dataManagement.importLots': 'Lots',
        'dataManagement.dropHint': 'Drop a CSV or JSON file here',
        'dataManagement.usersFormat': 'CSV: username, email, name, role, password',
        'dataManagement.lotsFormat': 'CSV: name, address, total_slots',
        'dataManagement.exportUsers': 'Export Users',
        'dataManagement.exportLots': 'Export Lots',
        'dataManagement.exportBookings': 'Export Bookings',
        'dataManagement.exportUsersDesc': 'All users with stats',
        'dataManagement.exportLotsDesc': 'All lots with stats',
        'dataManagement.exportBookingsDesc': 'Bookings with date filter',
        'dataManagement.dateFrom': 'From',
        'dataManagement.dateTo': 'To',
        'dataManagement.preview': 'Preview',
        'dataManagement.rows': 'rows',
        'common.error': 'Error',
        'common.loading': 'Loading...',
      };
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
  UploadSimple: (props: any) => <span data-testid="icon-upload" {...props} />,
  DownloadSimple: (props: any) => <span data-testid="icon-download" {...props} />,
  FileArrowUp: (props: any) => <span data-testid="icon-file-up" {...props} />,
  FileArrowDown: (props: any) => <span data-testid="icon-file-down" {...props} />,
  Table: (props: any) => <span data-testid="icon-table" {...props} />,
  Warning: (props: any) => <span data-testid="icon-warning" {...props} />,
  CheckCircle: (props: any) => <span data-testid="icon-check" {...props} />,
}));

vi.mock('../api/client', () => ({
  api: {},
}));

import { AdminDataManagementPage } from './AdminDataManagement';

describe('AdminDataManagementPage', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the title', () => {
    render(<AdminDataManagementPage />);
    expect(screen.getByText('Data Management')).toBeInTheDocument();
  });

  it('renders import and export tabs', () => {
    render(<AdminDataManagementPage />);
    expect(screen.getByTestId('tab-import')).toBeInTheDocument();
    expect(screen.getByTestId('tab-export')).toBeInTheDocument();
  });

  it('shows import section by default with drop zone', () => {
    render(<AdminDataManagementPage />);
    expect(screen.getByTestId('import-section')).toBeInTheDocument();
    expect(screen.getByTestId('drop-zone')).toBeInTheDocument();
  });

  it('switches to export tab showing export cards', () => {
    render(<AdminDataManagementPage />);
    fireEvent.click(screen.getByTestId('tab-export'));
    expect(screen.getByTestId('export-section')).toBeInTheDocument();
    expect(screen.getByTestId('export-card-users')).toBeInTheDocument();
    expect(screen.getByTestId('export-card-lots')).toBeInTheDocument();
    expect(screen.getByTestId('export-card-bookings')).toBeInTheDocument();
  });

  it('shows CSV format hint for users import', () => {
    render(<AdminDataManagementPage />);
    expect(screen.getByText('CSV: username, email, name, role, password')).toBeInTheDocument();
  });

  it('export card clicks open download URL', () => {
    const openSpy = vi.spyOn(window, 'open').mockImplementation(() => null);
    render(<AdminDataManagementPage />);
    fireEvent.click(screen.getByTestId('tab-export'));
    // Click the first CSV button inside export-card-users
    const userCard = screen.getByTestId('export-card-users');
    const btn = userCard.querySelector('button');
    if (btn) fireEvent.click(btn);
    expect(openSpy).toHaveBeenCalledWith('/api/v1/admin/data/export/users', '_blank');
    openSpy.mockRestore();
  });
});
