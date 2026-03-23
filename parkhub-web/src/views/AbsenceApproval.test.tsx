import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

// ── Mocks ──

const mockMyAbsenceRequests = vi.fn();
const mockPendingAbsenceRequests = vi.fn();
const mockSubmitAbsenceRequest = vi.fn();
const mockApproveAbsenceRequest = vi.fn();
const mockRejectAbsenceRequest = vi.fn();
const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    myAbsenceRequests: (...args: any[]) => mockMyAbsenceRequests(...args),
    pendingAbsenceRequests: (...args: any[]) => mockPendingAbsenceRequests(...args),
    submitAbsenceRequest: (...args: any[]) => mockSubmitAbsenceRequest(...args),
    approveAbsenceRequest: (...args: any[]) => mockApproveAbsenceRequest(...args),
    rejectAbsenceRequest: (...args: any[]) => mockRejectAbsenceRequest(...args),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'absenceApproval.title': 'Absence Approval',
        'absenceApproval.subtitle': 'Request and manage absences',
        'absenceApproval.help': 'Request time away from your parking spot. Admins will review and approve or reject.',
        'absenceApproval.helpLabel': 'Help',
        'absenceApproval.submitTitle': 'Submit Request',
        'absenceApproval.type': 'Type',
        'absenceApproval.startDate': 'Start Date',
        'absenceApproval.endDate': 'End Date',
        'absenceApproval.reason': 'Reason',
        'absenceApproval.reasonPlaceholder': 'Why do you need this absence?',
        'absenceApproval.submitBtn': 'Submit Request',
        'absenceApproval.submitting': 'Submitting...',
        'absenceApproval.submitted': 'Request submitted',
        'absenceApproval.requiredFields': 'Please fill all fields',
        'absenceApproval.myRequests': 'My Requests',
        'absenceApproval.pendingQueue': 'Pending Queue',
        'absenceApproval.noRequests': 'No absence requests yet',
        'absenceApproval.noPending': 'No pending requests',
        'absenceApproval.commentPlaceholder': 'Comment (optional)',
        'absenceApproval.approveBtn': 'Approve',
        'absenceApproval.rejectBtn': 'Reject',
        'absenceApproval.approved': 'Request approved',
        'absenceApproval.rejected': 'Request rejected',
        'absenceApproval.rejectReasonRequired': 'Rejection reason is required',
        'absenceApproval.status.pending': 'Pending',
        'absenceApproval.status.approved': 'Approved',
        'absenceApproval.status.rejected': 'Rejected',
        'absenceApproval.types.vacation': 'Vacation',
        'absenceApproval.types.sick': 'Sick',
        'absenceApproval.types.homeoffice': 'Home Office',
        'absenceApproval.types.businessTrip': 'Business Trip',
        'absenceApproval.types.personal': 'Personal',
        'absenceApproval.types.other': 'Other',
        'common.error': 'Error',
        'common.loading': 'Loading...',
        'common.close': 'Close',
      };
      return map[key] || key;
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
  Calendar: (props: any) => <span data-testid="icon-calendar" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  PaperPlaneTilt: (props: any) => <span data-testid="icon-send" {...props} />,
  ChatText: (props: any) => <span data-testid="icon-chat" {...props} />,
  Warning: (props: any) => <span data-testid="icon-warning" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: {
    success: (...args: any[]) => mockToastSuccess(...args),
    error: (...args: any[]) => mockToastError(...args),
  },
}));

import { AbsenceApprovalPage } from './AbsenceApproval';

const sampleMyRequests = [
  {
    id: 'r1',
    user_id: 'u1',
    user_name: 'Alice',
    absence_type: 'vacation',
    start_date: '2026-04-01',
    end_date: '2026-04-05',
    reason: 'Family trip',
    status: 'pending' as const,
    created_at: '2026-03-20T10:00:00Z',
  },
  {
    id: 'r2',
    user_id: 'u1',
    user_name: 'Alice',
    absence_type: 'sick',
    start_date: '2026-03-15',
    end_date: '2026-03-16',
    reason: 'Flu',
    status: 'approved' as const,
    reviewer_comment: 'Get well soon',
    created_at: '2026-03-14T08:00:00Z',
    reviewed_at: '2026-03-14T09:00:00Z',
  },
];

const samplePendingRequests = [
  {
    id: 'r3',
    user_id: 'u2',
    user_name: 'Bob',
    absence_type: 'homeoffice',
    start_date: '2026-04-10',
    end_date: '2026-04-10',
    reason: 'WFH day',
    status: 'pending' as const,
    created_at: '2026-03-22T12:00:00Z',
  },
];

describe('AbsenceApprovalPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    mockPendingAbsenceRequests.mockRejectedValue(new Error('forbidden'));
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders page title and subtitle', async () => {
    render(<AbsenceApprovalPage />);
    await waitFor(() => {
      expect(screen.getByText('Absence Approval')).toBeInTheDocument();
      expect(screen.getByText('Request and manage absences')).toBeInTheDocument();
    });
  });

  it('shows help tooltip when help button is clicked', async () => {
    render(<AbsenceApprovalPage />);
    await waitFor(() => expect(screen.getByTitle('Help')).toBeInTheDocument());
    fireEvent.click(screen.getByTitle('Help'));
    expect(screen.getByText('Request time away from your parking spot. Admins will review and approve or reject.')).toBeInTheDocument();
  });

  it('shows empty state when no requests', async () => {
    render(<AbsenceApprovalPage />);
    await waitFor(() => {
      expect(screen.getByText('No absence requests yet')).toBeInTheDocument();
    });
  });

  it('renders my requests with status badges', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: sampleMyRequests });
    render(<AbsenceApprovalPage />);
    await waitFor(() => {
      expect(screen.getByText('Pending')).toBeInTheDocument();
      expect(screen.getByText('Approved')).toBeInTheDocument();
      expect(screen.getByText('Family trip')).toBeInTheDocument();
    });
  });

  it('shows reviewer comment on approved request', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: sampleMyRequests });
    render(<AbsenceApprovalPage />);
    await waitFor(() => {
      expect(screen.getByText('Get well soon')).toBeInTheDocument();
    });
  });

  it('shows admin tabs when user is admin', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: sampleMyRequests });
    mockPendingAbsenceRequests.mockResolvedValue({ success: true, data: samplePendingRequests });
    render(<AbsenceApprovalPage />);
    await waitFor(() => {
      expect(screen.getByText('My Requests')).toBeInTheDocument();
      expect(screen.getByText('Pending Queue')).toBeInTheDocument();
    });
  });

  it('renders submit form with all fields', async () => {
    render(<AbsenceApprovalPage />);
    await waitFor(() => {
      expect(screen.getByText('Submit Request', { selector: 'h3' })).toBeInTheDocument();
      expect(screen.getByText('Start Date')).toBeInTheDocument();
      expect(screen.getByText('End Date')).toBeInTheDocument();
      expect(screen.getByText('Reason')).toBeInTheDocument();
    });
  });

  it('shows pending badge count in admin tab', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    mockPendingAbsenceRequests.mockResolvedValue({ success: true, data: samplePendingRequests });
    render(<AbsenceApprovalPage />);
    await waitFor(() => {
      expect(screen.getByText('1')).toBeInTheDocument();
    });
  });
});
