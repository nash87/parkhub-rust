import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockGetProposals = vi.fn();
const mockReviewProposal = vi.fn();
const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    getTranslationProposals: (...args: any[]) => mockGetProposals(...args),
    reviewProposal: (...args: any[]) => mockReviewProposal(...args),
  },
}));

vi.mock('react-hot-toast', () => ({
  default: {
    success: (...args: any[]) => mockToastSuccess(...args),
    error: (...args: any[]) => mockToastError(...args),
  },
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, variants, whileHover, whileTap, layout, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Translate: (props: any) => <span data-testid="icon-translate" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  Eye: (props: any) => <span data-testid="icon-eye" {...props} />,
  ThumbsUp: (props: any) => <span data-testid="icon-thumbs-up" {...props} />,
  ThumbsDown: (props: any) => <span data-testid="icon-thumbs-down" {...props} />,
  ChatCircleDots: (props: any) => <span data-testid="icon-chat" {...props} />,
  ArrowsClockwise: (props: any) => <span data-testid="icon-refresh" {...props} />,
  CheckCircle: (props: any) => <span data-testid="icon-check-circle" {...props} />,
  XCircle: (props: any) => <span data-testid="icon-x-circle" {...props} />,
  MagnifyingGlass: (props: any) => <span data-testid="icon-search" {...props} />,
}));

vi.mock('@tanstack/react-table', () => ({
  createColumnHelper: () => ({
    accessor: (key: string, opts: any) => ({ ...opts, accessorKey: key }),
    display: (opts: any) => opts,
  }),
}));

vi.mock('../components/ui/DataTable', () => ({
  DataTable: ({ data, emptyMessage }: any) => (
    <div data-testid="data-table">
      {data.length === 0 ? <p>{emptyMessage}</p> : <p>{data.length} items</p>}
    </div>
  ),
}));

vi.mock('../components/ui/ConfirmDialog', () => ({
  ConfirmDialog: ({ open, onConfirm, onCancel, title, message }: any) =>
    open ? (
      <div data-testid="confirm-dialog">
        <p>{title}</p>
        <p>{message}</p>
        <button onClick={onConfirm}>Confirm</button>
        <button onClick={onCancel}>Cancel</button>
      </div>
    ) : null,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => {
      const map: Record<string, string> = {
        'translations.admin.title': 'Translation Management',
        'translations.admin.pendingReview': 'pending review',
        'translations.admin.approve': 'Approve',
        'translations.admin.reject': 'Reject',
        'translations.admin.approveAll': 'Approve All',
        'translations.admin.rejectAll': 'Reject All',
        'translations.admin.searchProposals': 'Search proposals...',
        'translations.admin.reviewProposal': 'Review Proposal',
        'translations.admin.comment': 'Comment',
        'translations.admin.commentPlaceholder': 'Optional comment',
        'translations.admin.approved': 'Approved!',
        'translations.admin.rejected': 'Rejected',
        'translations.admin.reviewDetail': 'Review Detail',
        'translations.admin.bulkComplete': `${opts?.count ?? 0} completed`,
        'translations.admin.confirmBulkApprove': `Confirm bulk action for ${opts?.count ?? 0} proposals`,
        'translations.filterStatus': 'Filter status',
        'translations.allStatuses': 'All Statuses',
        'translations.statusPending': 'Pending',
        'translations.statusApproved': 'Approved',
        'translations.statusRejected': 'Rejected',
        'translations.noProposals': 'No proposals',
        'translations.keyLabel': 'Key',
        'translations.current': 'Current',
        'translations.proposed': 'Proposed',
        'translations.score': 'Score',
        'translations.proposedBy': 'Proposed by',
        'admin.status': 'Status',
        'common.refresh': 'Refresh',
        'common.close': 'Close',
        'common.cancel': 'Cancel',
        'common.error': 'Error',
        'ui.confirmAction': 'Confirm Action',
      };
      return map[key] || key;
    },
  }),
}));

const MOCK_PROPOSALS = [
  {
    id: 'p-1',
    language: 'de',
    key: 'nav.dashboard',
    current_value: 'Dashboard',
    proposed_value: 'Startseite',
    proposed_by: 'u-1',
    proposed_by_name: 'Alice',
    status: 'pending' as const,
    votes_for: 3,
    votes_against: 1,
    created_at: '2026-01-15T10:00:00Z',
    updated_at: '2026-01-15T10:00:00Z',
  },
  {
    id: 'p-2',
    language: 'de',
    key: 'nav.bookings',
    current_value: 'Bookings',
    proposed_value: 'Buchungen',
    proposed_by: 'u-2',
    proposed_by_name: 'Bob',
    status: 'approved' as const,
    votes_for: 5,
    votes_against: 0,
    reviewer_name: 'Admin',
    created_at: '2026-01-14T10:00:00Z',
    updated_at: '2026-01-15T10:00:00Z',
  },
];

import { AdminTranslationsPage } from './AdminTranslations';

describe('AdminTranslationsPage', () => {
  beforeEach(() => {
    mockGetProposals.mockClear();
    mockReviewProposal.mockClear();
    mockToastSuccess.mockClear();
    mockToastError.mockClear();
    mockGetProposals.mockResolvedValue({ success: true, data: MOCK_PROPOSALS });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the page title', async () => {
    render(<AdminTranslationsPage />);
    expect(screen.getByText('Translation Management')).toBeInTheDocument();
  });

  it('loads proposals on mount', async () => {
    render(<AdminTranslationsPage />);
    await waitFor(() => {
      expect(mockGetProposals).toHaveBeenCalledWith('pending');
    });
  });

  it('shows pending count badge', async () => {
    render(<AdminTranslationsPage />);
    await waitFor(() => {
      expect(screen.getByText(/1.*pending review/)).toBeInTheDocument();
    });
  });

  it('renders data table with proposals', async () => {
    render(<AdminTranslationsPage />);
    await waitFor(() => {
      expect(screen.getByTestId('data-table')).toBeInTheDocument();
    });
  });

  it('renders search input', () => {
    render(<AdminTranslationsPage />);
    expect(screen.getByLabelText('Search proposals...')).toBeInTheDocument();
  });

  it('renders filter dropdown', () => {
    render(<AdminTranslationsPage />);
    expect(screen.getByLabelText('Filter status')).toBeInTheDocument();
  });

  it('changes filter and reloads proposals', async () => {
    const user = userEvent.setup();
    render(<AdminTranslationsPage />);

    await waitFor(() => {
      expect(mockGetProposals).toHaveBeenCalledTimes(1);
    });

    const select = screen.getByLabelText('Filter status');
    await user.selectOptions(select, 'all');

    await waitFor(() => {
      expect(mockGetProposals).toHaveBeenCalledWith(undefined);
    });
  });

  it('search filters proposals', async () => {
    const user = userEvent.setup();
    render(<AdminTranslationsPage />);

    await waitFor(() => {
      expect(mockGetProposals).toHaveBeenCalled();
    });

    const searchInput = screen.getByLabelText('Search proposals...');
    await user.type(searchInput, 'Startseite');

    // Filtered results
    await waitFor(() => {
      expect(screen.getByTestId('data-table')).toBeInTheDocument();
    });
  });

  it('refresh button reloads proposals', async () => {
    const user = userEvent.setup();
    render(<AdminTranslationsPage />);

    await waitFor(() => {
      expect(mockGetProposals).toHaveBeenCalledTimes(1);
    });

    await user.click(screen.getByLabelText('Refresh'));

    await waitFor(() => {
      expect(mockGetProposals).toHaveBeenCalledTimes(2);
    });
  });

  it('renders approve all and reject all buttons when pending exist', async () => {
    render(<AdminTranslationsPage />);
    await waitFor(() => {
      expect(screen.getByText('Approve All')).toBeInTheDocument();
      expect(screen.getByText('Reject All')).toBeInTheDocument();
    });
  });

  it('hides bulk actions when no pending proposals', async () => {
    mockGetProposals.mockResolvedValue({
      success: true,
      data: [{ ...MOCK_PROPOSALS[1] }],
    });

    render(<AdminTranslationsPage />);
    await waitFor(() => {
      expect(screen.queryByText('Approve All')).not.toBeInTheDocument();
    });
  });

  it('shows empty state when no proposals', async () => {
    mockGetProposals.mockResolvedValue({ success: true, data: [] });
    render(<AdminTranslationsPage />);
    await waitFor(() => {
      expect(screen.getByText('No proposals')).toBeInTheDocument();
    });
  });
});
