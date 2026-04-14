import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, fallbackOrOpts?: string | Record<string, any>) => {
      const map: Record<string, string> = {
        'billing.title': 'Cost Center Billing',
        'billing.subtitle': 'Billing breakdown by cost center and department',
        'billing.export': 'CSV Export',
        'billing.totalSpending': 'Total Spending',
        'billing.totalBookings': 'Total Bookings',
        'billing.totalUsers': 'Total Users',
        'billing.byCostCenter': 'By Cost Center',
        'billing.byDepartment': 'By Department',
        'billing.costCenter': 'Cost Center',
        'billing.department': 'Department',
        'billing.users': 'Users',
        'billing.bookings': 'Bookings',
        'billing.credits': 'Credits',
        'billing.amount': 'Amount',
        'billing.empty': 'No billing data',
        'billing.help': 'This module provides billing analytics.',
        'common.error': 'Error',
      };
      return map[key] || (typeof fallbackOrOpts === 'string' ? fallbackOrOpts : key);
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  CurrencyDollar: (props: any) => <span data-testid="icon-dollar" {...props} />,
  ChartBar: (props: any) => <span data-testid="icon-chart" {...props} />,
  DownloadSimple: (props: any) => <span data-testid="icon-download" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  Buildings: (props: any) => <span data-testid="icon-buildings" {...props} />,
}));

import { AdminBillingPage } from './AdminBilling';

const sampleCcData = [
  { cost_center: 'CC-100', department: 'Engineering', user_count: 5, total_bookings: 20, total_credits_used: 100, total_amount: 250.50, currency: 'EUR' },
  { cost_center: 'CC-200', department: 'Marketing', user_count: 3, total_bookings: 10, total_credits_used: 30, total_amount: 75.00, currency: 'EUR' },
];

const sampleDeptData = [
  { department: 'Engineering', user_count: 5, total_bookings: 20, total_credits_used: 100, total_amount: 250.50, currency: 'EUR' },
  { department: 'Marketing', user_count: 3, total_bookings: 10, total_credits_used: 30, total_amount: 75.00, currency: 'EUR' },
];

describe('AdminBillingPage', () => {
  beforeEach(() => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/by-cost-center')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleCcData }) } as Response);
      }
      if (url.includes('/by-department')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleDeptData }) } as Response);
      }
      if (url.includes('/export')) {
        return Promise.resolve({ blob: () => Promise.resolve(new Blob(['csv'])) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: null }) } as Response);
    }) as any;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the title', async () => {
    render(<AdminBillingPage />);
    await waitFor(() => {
      expect(screen.getByText('Cost Center Billing')).toBeInTheDocument();
    });
  });

  it('renders summary cards', async () => {
    render(<AdminBillingPage />);
    await waitFor(() => {
      expect(screen.getByTestId('billing-summary')).toBeInTheDocument();
      expect(screen.getByText('Total Spending')).toBeInTheDocument();
      expect(screen.getByText('Total Bookings')).toBeInTheDocument();
    });
  });

  it('renders billing rows in cost center tab', async () => {
    render(<AdminBillingPage />);
    await waitFor(() => {
      const rows = screen.getAllByTestId('billing-row');
      expect(rows).toHaveLength(2);
      expect(screen.getByText('CC-100')).toBeInTheDocument();
      expect(screen.getByText('CC-200')).toBeInTheDocument();
    });
  });

  it('switches to department tab', async () => {
    render(<AdminBillingPage />);
    await waitFor(() => expect(screen.getByTestId('billing-tabs')).toBeInTheDocument());

    fireEvent.click(screen.getByText('By Department'));
    await waitFor(() => {
      const rows = screen.getAllByTestId('billing-row');
      expect(rows).toHaveLength(2);
    });
  });

  it('shows export button', async () => {
    render(<AdminBillingPage />);
    await waitFor(() => {
      expect(screen.getByTestId('export-btn')).toBeInTheDocument();
    });
  });

  it('shows empty state when no data', async () => {
    global.fetch = vi.fn(() =>
      Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response)
    ) as any;

    render(<AdminBillingPage />);
    await waitFor(() => {
      expect(screen.getByText('No billing data')).toBeInTheDocument();
    });
  });

  it('handles CSV export click', async () => {
    const mockBlob = new Blob(['csv data'], { type: 'text/csv' });
    const createObjectURL = vi.fn(() => 'blob:test');
    const revokeObjectURL = vi.fn();
    Object.defineProperty(URL, 'createObjectURL', { value: createObjectURL, writable: true });
    Object.defineProperty(URL, 'revokeObjectURL', { value: revokeObjectURL, writable: true });

    render(<AdminBillingPage />);
    await waitFor(() => expect(screen.getByTestId('export-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('export-btn'));

    await waitFor(() => {
      expect(createObjectURL).toHaveBeenCalled();
      expect(revokeObjectURL).toHaveBeenCalled();
    });
  });

  it('shows help text on help button click', async () => {
    render(<AdminBillingPage />);
    await waitFor(() => expect(screen.getByTestId('icon-question')).toBeInTheDocument());

    const helpBtn = screen.getByTestId('icon-question').closest('button')!;
    fireEvent.click(helpBtn);

    await waitFor(() => {
      expect(screen.getByText('This module provides billing analytics.')).toBeInTheDocument();
    });
  });

  it('toggles help text off on second click', async () => {
    render(<AdminBillingPage />);
    await waitFor(() => expect(screen.getByTestId('icon-question')).toBeInTheDocument());

    const helpBtn = screen.getByTestId('icon-question').closest('button')!;
    fireEvent.click(helpBtn);
    expect(screen.getByText('This module provides billing analytics.')).toBeInTheDocument();

    fireEvent.click(helpBtn);
    expect(screen.queryByText('This module provides billing analytics.')).not.toBeInTheDocument();
  });

  it('shows empty state in department tab', async () => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/by-cost-center')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleCcData }) } as Response);
      }
      if (url.includes('/by-department')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
    }) as any;

    render(<AdminBillingPage />);
    await waitFor(() => expect(screen.getByTestId('billing-tabs')).toBeInTheDocument());

    fireEvent.click(screen.getByText('By Department'));
    await waitFor(() => {
      expect(screen.getByText('No billing data')).toBeInTheDocument();
    });
  });

  it('switches back to cost-center tab from department', async () => {
    global.fetch = vi.fn((url: string) => {
      if (url.includes('/by-cost-center')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleCcData }) } as Response);
      }
      if (url.includes('/by-department')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
    }) as any;

    render(<AdminBillingPage />);
    await waitFor(() => expect(screen.getByTestId('billing-tabs')).toBeInTheDocument());

    fireEvent.click(screen.getByText('By Department'));
    fireEvent.click(screen.getByText('By Cost Center'));
    await waitFor(() => {
      expect(screen.getByText('Cost Center Billing')).toBeInTheDocument();
    });
  });

  it('CSV export error shows error toast', async () => {
    const mockToast = vi.fn();
    const toast = await import('react-hot-toast');
    (toast.default as any).error = mockToast;

    global.fetch = vi.fn((url: string) => {
      if (url.includes('/by-cost-center') || url.includes('/by-department')) {
        return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      }
      if (url.includes('/export')) {
        return Promise.reject(new Error('Export failed'));
      }
      return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
    }) as any;

    render(<AdminBillingPage />);
    await waitFor(() => expect(screen.getByTestId('export-btn')).toBeInTheDocument());

    fireEvent.click(screen.getByTestId('export-btn'));
    await waitFor(() => {
      expect(mockToast).toHaveBeenCalled();
    });
  });

  it('handles network error in data loading', async () => {
    global.fetch = vi.fn(() => Promise.reject(new Error('Network'))) as any;

    render(<AdminBillingPage />);
    // Should still render after error
    await waitFor(() => {
      expect(screen.getByText('Cost Center Billing')).toBeInTheDocument();
    });
  });
});
