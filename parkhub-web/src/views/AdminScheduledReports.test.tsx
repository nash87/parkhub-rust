import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'scheduledReports.title': 'Scheduled Reports',
        'scheduledReports.subtitle': 'Configure automated email report delivery',
        'scheduledReports.help': 'Set up recurring reports delivered automatically to specified recipients.',
        'scheduledReports.helpLabel': 'Help',
        'scheduledReports.create': 'Create Schedule',
        'scheduledReports.newSchedule': 'New Schedule',
        'scheduledReports.editSchedule': 'Edit Schedule',
        'scheduledReports.name': 'Name',
        'scheduledReports.reportType': 'Report Type',
        'scheduledReports.frequency': 'Frequency',
        'scheduledReports.recipients': 'Recipients',
        'scheduledReports.recipientsPlaceholder': 'user@example.com, admin@example.com',
        'scheduledReports.recipientsLabel': 'Recipients',
        'scheduledReports.save': 'Save',
        'scheduledReports.created': 'Schedule created',
        'scheduledReports.updated': 'Schedule updated',
        'scheduledReports.deleted': 'Schedule deleted',
        'scheduledReports.sentNow': 'Report sent',
        'scheduledReports.sendNow': 'Send Now',
        'scheduledReports.edit': 'Edit',
        'scheduledReports.delete': 'Delete',
        'scheduledReports.lastSent': 'Last sent',
        'scheduledReports.empty': 'No schedules configured',
        'scheduledReports.nameRequired': 'Name is required',
        'scheduledReports.recipientsRequired': 'At least one recipient required',
        'common.error': 'Error',
        'common.cancel': 'Cancel',
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
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  Plus: (props: any) => <span data-testid="icon-plus" {...props} />,
  Trash: (props: any) => <span data-testid="icon-trash" {...props} />,
  PaperPlaneTilt: (props: any) => <span data-testid="icon-send" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  Pencil: (props: any) => <span data-testid="icon-pencil" {...props} />,
  ToggleLeft: (props: any) => <span data-testid="icon-toggle-off" {...props} />,
  ToggleRight: (props: any) => <span data-testid="icon-toggle-on" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: { success: vi.fn(), error: vi.fn() },
}));

import { AdminScheduledReportsPage } from './AdminScheduledReports';

const sampleSchedules = {
  schedules: [
    {
      id: 'sched-001',
      name: 'Daily Occupancy Digest',
      report_type: 'occupancy_summary',
      frequency: 'daily',
      recipients: ['admin@parkhub.test'],
      enabled: true,
      last_sent_at: '2026-03-23T08:00:00Z',
      next_run_at: '2026-03-24T08:00:00Z',
      created_at: '2026-03-20T10:00:00Z',
      updated_at: '2026-03-23T08:00:00Z',
    },
    {
      id: 'sched-002',
      name: 'Weekly Revenue Summary',
      report_type: 'revenue_report',
      frequency: 'weekly',
      recipients: ['admin@parkhub.test', 'finance@parkhub.test'],
      enabled: false,
      last_sent_at: null,
      next_run_at: '2026-03-30T08:00:00Z',
      created_at: '2026-03-20T10:00:00Z',
      updated_at: '2026-03-20T10:00:00Z',
    },
  ],
  total: 2,
};

describe('AdminScheduledReportsPage', () => {
  beforeEach(() => {
    global.fetch = vi.fn(() =>
      Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleSchedules }) } as Response)
    ) as any;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the title', async () => {
    render(<AdminScheduledReportsPage />);
    await waitFor(() => {
      expect(screen.getByText('Scheduled Reports')).toBeInTheDocument();
    });
  });

  it('renders schedule cards', async () => {
    render(<AdminScheduledReportsPage />);
    await waitFor(() => {
      const cards = screen.getAllByTestId('schedule-card');
      expect(cards).toHaveLength(2);
    });
  });

  it('renders schedule names', async () => {
    render(<AdminScheduledReportsPage />);
    await waitFor(() => {
      expect(screen.getByText('Daily Occupancy Digest')).toBeInTheDocument();
      expect(screen.getByText('Weekly Revenue Summary')).toBeInTheDocument();
    });
  });

  it('shows help text when help button clicked', async () => {
    render(<AdminScheduledReportsPage />);
    await waitFor(() => {
      expect(screen.getByTestId('reports-help-btn')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId('reports-help-btn'));
    expect(screen.getByTestId('reports-help')).toBeInTheDocument();
  });

  it('shows create form when create button clicked', async () => {
    render(<AdminScheduledReportsPage />);
    await waitFor(() => {
      expect(screen.getByTestId('create-schedule-btn')).toBeInTheDocument();
    });
    fireEvent.click(screen.getByTestId('create-schedule-btn'));
    expect(screen.getByTestId('schedule-form')).toBeInTheDocument();
    expect(screen.getByTestId('form-name')).toBeInTheDocument();
    expect(screen.getByTestId('form-type')).toBeInTheDocument();
    expect(screen.getByTestId('form-frequency')).toBeInTheDocument();
    expect(screen.getByTestId('form-recipients')).toBeInTheDocument();
  });

  it('shows cron expressions for frequencies', async () => {
    render(<AdminScheduledReportsPage />);
    await waitFor(() => {
      expect(screen.getByText('0 8 * * *')).toBeInTheDocument();
      expect(screen.getByText('0 8 * * MON')).toBeInTheDocument();
    });
  });

  it('renders action buttons for each schedule', async () => {
    render(<AdminScheduledReportsPage />);
    await waitFor(() => {
      const sendBtns = screen.getAllByTestId('send-now-btn');
      const editBtns = screen.getAllByTestId('edit-btn');
      const deleteBtns = screen.getAllByTestId('delete-btn');
      expect(sendBtns).toHaveLength(2);
      expect(editBtns).toHaveLength(2);
      expect(deleteBtns).toHaveLength(2);
    });
  });
});
