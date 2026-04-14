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

  it('shows loading state', async () => {
    mockMyAbsenceRequests.mockReturnValue(new Promise(() => {})); // never resolves
    render(<AbsenceApprovalPage />);
    expect(screen.getByText('Loading...')).toBeInTheDocument();
  });

  it('submits absence request successfully', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    mockSubmitAbsenceRequest.mockResolvedValue({ success: true });
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Submit Request', { selector: 'h3' }));

    // Fill start date
    const dateInputs = document.querySelectorAll('input[type="date"]');
    fireEvent.change(dateInputs[0], { target: { value: '2026-05-01' } });
    fireEvent.change(dateInputs[1], { target: { value: '2026-05-05' } });

    // Fill reason
    const textarea = screen.getByPlaceholderText('Why do you need this absence?');
    fireEvent.change(textarea, { target: { value: 'Vacation trip' } });

    // Submit
    fireEvent.click(screen.getByRole('button', { name: /Submit Request/i }));
    await waitFor(() => {
      expect(mockSubmitAbsenceRequest).toHaveBeenCalledWith(
        expect.objectContaining({
          absence_type: 'vacation',
          start_date: '2026-05-01',
          end_date: '2026-05-05',
          reason: 'Vacation trip',
        }),
      );
      expect(mockToastSuccess).toHaveBeenCalledWith('Request submitted');
    });
  });

  it('shows validation error when submitting with empty fields', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Submit Request', { selector: 'h3' }));

    // Submit without filling anything
    fireEvent.click(screen.getByRole('button', { name: /Submit Request/i }));
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Please fill all fields');
    });
  });

  it('handles submit request failure', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    mockSubmitAbsenceRequest.mockResolvedValue({ success: false, error: { message: 'Server error' } });
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Submit Request', { selector: 'h3' }));

    const dateInputs = document.querySelectorAll('input[type="date"]');
    fireEvent.change(dateInputs[0], { target: { value: '2026-05-01' } });
    fireEvent.change(dateInputs[1], { target: { value: '2026-05-05' } });
    fireEvent.change(screen.getByPlaceholderText('Why do you need this absence?'), { target: { value: 'Reason' } });

    fireEvent.click(screen.getByRole('button', { name: /Submit Request/i }));
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Server error');
    });
  });

  it('handles submit request network error', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    mockSubmitAbsenceRequest.mockRejectedValue(new Error('Network'));
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Submit Request', { selector: 'h3' }));

    const dateInputs = document.querySelectorAll('input[type="date"]');
    fireEvent.change(dateInputs[0], { target: { value: '2026-05-01' } });
    fireEvent.change(dateInputs[1], { target: { value: '2026-05-05' } });
    fireEvent.change(screen.getByPlaceholderText('Why do you need this absence?'), { target: { value: 'Reason' } });

    fireEvent.click(screen.getByRole('button', { name: /Submit Request/i }));
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Error');
    });
  });

  it('changes absence type in submit form', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Submit Request', { selector: 'h3' }));

    const typeSelect = screen.getByDisplayValue('Vacation');
    fireEvent.change(typeSelect, { target: { value: 'sick' } });
    expect(typeSelect).toHaveValue('sick');
  });

  it('switches back to my-requests tab after clicking pending queue', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: sampleMyRequests });
    mockPendingAbsenceRequests.mockResolvedValue({ success: true, data: samplePendingRequests });
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Pending Queue'));
    fireEvent.click(screen.getByText('Pending Queue'));
    await waitFor(() => screen.getByText('Bob'));
    fireEvent.click(screen.getByText('My Requests'));
    await waitFor(() => {
      expect(screen.queryByText('Bob')).not.toBeInTheDocument();
    });
  });

  it('shows admin pending queue when admin tab clicked', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: sampleMyRequests });
    mockPendingAbsenceRequests.mockResolvedValue({ success: true, data: samplePendingRequests });
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Pending Queue'));

    fireEvent.click(screen.getByText('Pending Queue'));
    await waitFor(() => {
      expect(screen.getByText('Bob')).toBeInTheDocument();
      expect(screen.getByText('WFH day')).toBeInTheDocument();
    });
  });

  it('shows no-pending message when admin queue is empty', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: sampleMyRequests });
    mockPendingAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Pending Queue'));

    fireEvent.click(screen.getByText('Pending Queue'));
    await waitFor(() => {
      expect(screen.getByText('No pending requests')).toBeInTheDocument();
    });
  });

  it('approves a pending request', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    mockPendingAbsenceRequests.mockResolvedValue({ success: true, data: samplePendingRequests });
    mockApproveAbsenceRequest.mockResolvedValue({ success: true });
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Pending Queue'));
    fireEvent.click(screen.getByText('Pending Queue'));
    await waitFor(() => screen.getByText('Bob'));

    fireEvent.click(screen.getByText('Approve'));
    await waitFor(() => {
      expect(mockApproveAbsenceRequest).toHaveBeenCalledWith('r3', undefined);
      expect(mockToastSuccess).toHaveBeenCalledWith('Request approved');
    });
  });

  it('approves with a comment', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    mockPendingAbsenceRequests.mockResolvedValue({ success: true, data: samplePendingRequests });
    mockApproveAbsenceRequest.mockResolvedValue({ success: true });
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Pending Queue'));
    fireEvent.click(screen.getByText('Pending Queue'));
    await waitFor(() => screen.getByText('Bob'));

    const commentInput = screen.getByPlaceholderText('Comment (optional)');
    fireEvent.change(commentInput, { target: { value: 'Sure, approved!' } });
    fireEvent.click(screen.getByText('Approve'));
    await waitFor(() => {
      expect(mockApproveAbsenceRequest).toHaveBeenCalledWith('r3', 'Sure, approved!');
    });
  });

  it('rejects a pending request with reason', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    mockPendingAbsenceRequests.mockResolvedValue({ success: true, data: samplePendingRequests });
    mockRejectAbsenceRequest.mockResolvedValue({ success: true });
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Pending Queue'));
    fireEvent.click(screen.getByText('Pending Queue'));
    await waitFor(() => screen.getByText('Bob'));

    const commentInput = screen.getByPlaceholderText('Comment (optional)');
    fireEvent.change(commentInput, { target: { value: 'Too many absences' } });
    fireEvent.click(screen.getByText('Reject'));
    await waitFor(() => {
      expect(mockRejectAbsenceRequest).toHaveBeenCalledWith('r3', 'Too many absences');
      expect(mockToastSuccess).toHaveBeenCalledWith('Request rejected');
    });
  });

  it('requires reason when rejecting', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    mockPendingAbsenceRequests.mockResolvedValue({ success: true, data: samplePendingRequests });
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Pending Queue'));
    fireEvent.click(screen.getByText('Pending Queue'));
    await waitFor(() => screen.getByText('Bob'));

    // Reject without entering a comment
    fireEvent.click(screen.getByText('Reject'));
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Rejection reason is required');
    });
  });

  it('handles approve failure', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    mockPendingAbsenceRequests.mockResolvedValue({ success: true, data: samplePendingRequests });
    mockApproveAbsenceRequest.mockResolvedValue({ success: false, error: { message: 'Approve failed' } });
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Pending Queue'));
    fireEvent.click(screen.getByText('Pending Queue'));
    await waitFor(() => screen.getByText('Bob'));

    fireEvent.click(screen.getByText('Approve'));
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Approve failed');
    });
  });

  it('handles reject failure', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    mockPendingAbsenceRequests.mockResolvedValue({ success: true, data: samplePendingRequests });
    mockRejectAbsenceRequest.mockResolvedValue({ success: false, error: { message: 'Reject failed' } });
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Pending Queue'));
    fireEvent.click(screen.getByText('Pending Queue'));
    await waitFor(() => screen.getByText('Bob'));

    const commentInput = screen.getByPlaceholderText('Comment (optional)');
    fireEvent.change(commentInput, { target: { value: 'No' } });
    fireEvent.click(screen.getByText('Reject'));
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Reject failed');
    });
  });

  it('handles approve network error', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    mockPendingAbsenceRequests.mockResolvedValue({ success: true, data: samplePendingRequests });
    mockApproveAbsenceRequest.mockRejectedValue(new Error('Network'));
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Pending Queue'));
    fireEvent.click(screen.getByText('Pending Queue'));
    await waitFor(() => screen.getByText('Bob'));

    fireEvent.click(screen.getByText('Approve'));
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Error');
    });
  });

  it('handles reject network error', async () => {
    mockMyAbsenceRequests.mockResolvedValue({ success: true, data: [] });
    mockPendingAbsenceRequests.mockResolvedValue({ success: true, data: samplePendingRequests });
    mockRejectAbsenceRequest.mockRejectedValue(new Error('Network'));
    render(<AbsenceApprovalPage />);
    await waitFor(() => screen.getByText('Pending Queue'));
    fireEvent.click(screen.getByText('Pending Queue'));
    await waitFor(() => screen.getByText('Bob'));

    const commentInput = screen.getByPlaceholderText('Comment (optional)');
    fireEvent.change(commentInput, { target: { value: 'No' } });
    fireEvent.click(screen.getByText('Reject'));
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Error');
    });
  });
});
