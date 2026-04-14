import { describe, it, expect, vi, afterEach } from 'vitest';
import React from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';

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

const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();
vi.mock('react-hot-toast', () => ({
  default: {
    success: (...args: any[]) => mockToastSuccess(...args),
    error: (...args: any[]) => mockToastError(...args),
  },
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

  it('switches import type to Lots', () => {
    render(<AdminDataManagementPage />);
    const lotsBtn = screen.getByText('Lots');
    fireEvent.click(lotsBtn);
    expect(screen.getByText('CSV: name, address, total_slots')).toBeInTheDocument();
  });

  it('switches import type back to Users after Lots', () => {
    render(<AdminDataManagementPage />);
    fireEvent.click(screen.getByText('Lots'));
    expect(screen.getByText('CSV: name, address, total_slots')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Users'));
    expect(screen.getByText('CSV: username, email, name, role, password')).toBeInTheDocument();
  });

  it('shows file preview after selecting a CSV file', async () => {
    render(<AdminDataManagementPage />);
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    const csvContent = 'name,email,role\nAlice,alice@test.com,user\nBob,bob@test.com,admin';
    const file = new File([csvContent], 'users.csv', { type: 'text/csv' });
    Object.defineProperty(file, 'text', { value: () => Promise.resolve(csvContent) });

    fireEvent.change(fileInput, { target: { files: [file] } });
    await waitFor(() => {
      expect(screen.getByTestId('import-preview')).toBeInTheDocument();
      expect(screen.getByText('name')).toBeInTheDocument();
    });
  });

  it('shows import button after file selection', async () => {
    render(<AdminDataManagementPage />);
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    const csvContent = 'name,email\nAlice,alice@test.com';
    const file = new File([csvContent], 'users.csv', { type: 'text/csv' });
    Object.defineProperty(file, 'text', { value: () => Promise.resolve(csvContent) });

    fireEvent.change(fileInput, { target: { files: [file] } });
    await waitFor(() => {
      expect(screen.getByTestId('import-btn')).toBeInTheDocument();
    });
  });

  it('performs import and shows result', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({
        success: true,
        data: { imported: 5, skipped: 1, errors: [] },
      }),
    } as Response);

    render(<AdminDataManagementPage />);
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    const csvContent = 'name,email\nAlice,alice@test.com';
    const file = new File([csvContent], 'users.csv', { type: 'text/csv' });
    Object.defineProperty(file, 'text', { value: () => Promise.resolve(csvContent) });

    fireEvent.change(fileInput, { target: { files: [file] } });
    await waitFor(() => screen.getByTestId('import-btn'));

    fireEvent.click(screen.getByTestId('import-btn'));
    await waitFor(() => {
      expect(screen.getByTestId('import-result')).toBeInTheDocument();
      expect(screen.getByText('5')).toBeInTheDocument(); // imported count
    });
  });

  it('shows import errors in result', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({
        success: true,
        data: {
          imported: 2,
          skipped: 0,
          errors: [
            { row: 3, field: 'email', message: 'Invalid email' },
            { row: 5, field: '', message: 'Missing required field' },
          ],
        },
      }),
    } as Response);

    render(<AdminDataManagementPage />);
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    const csvContent = 'name,email\nAlice,alice@test.com';
    const file = new File([csvContent], 'users.csv', { type: 'text/csv' });
    Object.defineProperty(file, 'text', { value: () => Promise.resolve(csvContent) });

    fireEvent.change(fileInput, { target: { files: [file] } });
    await waitFor(() => screen.getByTestId('import-btn'));

    fireEvent.click(screen.getByTestId('import-btn'));
    await waitFor(() => {
      expect(screen.getByText(/Row 3/)).toBeInTheDocument();
      expect(screen.getByText(/Invalid email/)).toBeInTheDocument();
    });
  });

  it('handles import failure with toast', async () => {
    // Use pre-configured mock toast
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({ success: false, error: 'Bad CSV format' }),
    } as Response);

    render(<AdminDataManagementPage />);
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    const csvContent = 'bad,data';
    const file = new File([csvContent], 'users.csv', { type: 'text/csv' });
    Object.defineProperty(file, 'text', { value: () => Promise.resolve(csvContent) });

    fireEvent.change(fileInput, { target: { files: [file] } });
    await waitFor(() => screen.getByTestId('import-btn'));
    fireEvent.click(screen.getByTestId('import-btn'));
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalled();
    });
  });

  it('handles import network error', async () => {
    // Use pre-configured mock toast
    global.fetch = vi.fn().mockRejectedValue(new Error('Network'));

    render(<AdminDataManagementPage />);
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    const csvContent = 'name,email\nAlice,a@b.com';
    const file = new File([csvContent], 'users.csv', { type: 'text/csv' });
    Object.defineProperty(file, 'text', { value: () => Promise.resolve(csvContent) });

    fireEvent.change(fileInput, { target: { files: [file] } });
    await waitFor(() => screen.getByTestId('import-btn'));
    fireEvent.click(screen.getByTestId('import-btn'));
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalled();
    });
  });

  it('handles JSON file import', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({
        success: true,
        data: { imported: 3, skipped: 0, errors: [] },
      }),
    } as Response);

    render(<AdminDataManagementPage />);
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    const jsonContent = '[{"name":"Alice"}]';
    const file = new File([jsonContent], 'users.json', { type: 'application/json' });
    Object.defineProperty(file, 'text', { value: () => Promise.resolve(jsonContent) });

    fireEvent.change(fileInput, { target: { files: [file] } });
    await waitFor(() => screen.getByTestId('import-btn'));

    fireEvent.click(screen.getByTestId('import-btn'));
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/v1/admin/import/users',
        expect.objectContaining({
          body: expect.stringContaining('"format":"json"'),
        }),
      );
    });
  });

  it('handles file drop on drop zone', async () => {
    render(<AdminDataManagementPage />);
    const dropZone = screen.getByTestId('drop-zone');
    const csvContent = 'name,email\nAlice,alice@test.com';
    const file = new File([csvContent], 'users.csv', { type: 'text/csv' });
    Object.defineProperty(file, 'text', { value: () => Promise.resolve(csvContent) });

    fireEvent.drop(dropZone, {
      dataTransfer: { files: [file] },
    });
    await waitFor(() => {
      expect(screen.getByTestId('import-preview')).toBeInTheDocument();
    });
  });

  it('handles dragover on drop zone', () => {
    render(<AdminDataManagementPage />);
    const dropZone = screen.getByTestId('drop-zone');
    const event = new Event('dragover', { bubbles: true });
    Object.defineProperty(event, 'preventDefault', { value: vi.fn() });
    dropZone.dispatchEvent(event);
    // Should not crash
  });

  it('exports with date range params', () => {
    const openSpy = vi.spyOn(window, 'open').mockImplementation(() => null);
    render(<AdminDataManagementPage />);
    fireEvent.click(screen.getByTestId('tab-export'));

    const fromInput = document.querySelectorAll('input[type="date"]')[0] as HTMLInputElement;
    const toInput = document.querySelectorAll('input[type="date"]')[1] as HTMLInputElement;
    fireEvent.change(fromInput, { target: { value: '2026-01-01' } });
    fireEvent.change(toInput, { target: { value: '2026-03-31' } });

    const bookingsCard = screen.getByTestId('export-card-bookings');
    const btn = bookingsCard.querySelector('button');
    if (btn) fireEvent.click(btn);
    expect(openSpy).toHaveBeenCalledWith(
      expect.stringContaining('from=2026-01-01'),
      '_blank',
    );
    openSpy.mockRestore();
  });

  it('exports lots', () => {
    const openSpy = vi.spyOn(window, 'open').mockImplementation(() => null);
    render(<AdminDataManagementPage />);
    fireEvent.click(screen.getByTestId('tab-export'));
    const lotsCard = screen.getByTestId('export-card-lots');
    const btn = lotsCard.querySelector('button');
    if (btn) fireEvent.click(btn);
    expect(openSpy).toHaveBeenCalledWith('/api/v1/admin/data/export/lots', '_blank');
    openSpy.mockRestore();
  });

  it('imports lots type using correct endpoint', async () => {
    global.fetch = vi.fn().mockResolvedValue({
      json: () => Promise.resolve({ success: true, data: { imported: 2, skipped: 0, errors: [] } }),
    } as Response);

    render(<AdminDataManagementPage />);
    fireEvent.click(screen.getByText('Lots'));

    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    const csvContent = 'name,address\nLot A,123 St';
    const file = new File([csvContent], 'lots.csv', { type: 'text/csv' });
    Object.defineProperty(file, 'text', { value: () => Promise.resolve(csvContent) });

    fireEvent.change(fileInput, { target: { files: [file] } });
    await waitFor(() => screen.getByTestId('import-btn'));
    fireEvent.click(screen.getByTestId('import-btn'));
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/v1/admin/import/lots',
        expect.anything(),
      );
    });
  });

  it('shows file name after selecting a file', async () => {
    render(<AdminDataManagementPage />);
    const fileInput = document.querySelector('input[type="file"]') as HTMLInputElement;
    const csvContent = 'name,email\nAlice,a@b.com';
    const file = new File([csvContent], 'my-users.csv', { type: 'text/csv' });
    Object.defineProperty(file, 'text', { value: () => Promise.resolve(csvContent) });

    fireEvent.change(fileInput, { target: { files: [file] } });
    await waitFor(() => {
      expect(screen.getByText('my-users.csv')).toBeInTheDocument();
    });
  });
});
