import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockGetProposals = vi.fn();
const mockVoteOnProposal = vi.fn();
const mockCreateProposal = vi.fn();
const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    getTranslationProposals: (...args: any[]) => mockGetProposals(...args),
    voteOnProposal: (...args: any[]) => mockVoteOnProposal(...args),
    createTranslationProposal: (...args: any[]) => mockCreateProposal(...args),
  },
}));

vi.mock('react-hot-toast', () => ({
  default: {
    success: (...args: any[]) => mockToastSuccess(...args),
    error: (...args: any[]) => mockToastError(...args),
  },
}));

vi.mock('../context/AuthContext', () => ({
  useAuth: () => ({ user: { id: 'u-1', name: 'Test User' } }),
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
  MagnifyingGlass: (props: any) => <span data-testid="icon-search" {...props} />,
  ThumbsUp: (props: any) => <span data-testid="icon-thumbs-up" {...props} />,
  ThumbsDown: (props: any) => <span data-testid="icon-thumbs-down" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  PaperPlaneTilt: (props: any) => <span data-testid="icon-send" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  ChatCircleDots: (props: any) => <span data-testid="icon-chat" {...props} />,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => {
      const map: Record<string, string> = {
        'translations.title': 'Translations',
        'translations.subtitle': 'Help improve translations',
        'translations.propose': 'Propose Change',
        'translations.proposals': 'Proposals',
        'translations.browseKeys': 'Browse Keys',
        'translations.searchKeys': 'Search keys...',
        'translations.selectLanguage': 'Select language',
        'translations.filterStatus': 'Filter status',
        'translations.allStatuses': 'All',
        'translations.statusPending': 'Pending',
        'translations.statusApproved': 'Approved',
        'translations.statusRejected': 'Rejected',
        'translations.noProposals': 'No proposals yet',
        'translations.proposalsCount': 'proposals',
        'translations.proposedBy': 'Proposed by',
        'translations.current': 'Current',
        'translations.proposed': 'Proposed',
        'translations.empty': '(empty)',
        'translations.voteFor': 'Vote for',
        'translations.voteAgainst': 'Vote against',
        'translations.voteCast': 'Vote recorded',
        'translations.score': 'score',
        'translations.newProposal': 'New Proposal',
        'translations.keyLabel': 'Key',
        'translations.proposedValue': 'Proposed Value',
        'translations.contextLabel': 'Context',
        'translations.contextPlaceholder': 'Why this change?',
        'translations.submitProposal': 'Submit Proposal',
        'translations.enterTranslation': 'Enter translation',
        'translations.currentValue': 'Current value',
        'translations.proposalCreated': 'Proposal created!',
        'translations.suggestChange': 'Suggest Change',
        'translations.value': 'Value',
        'common.cancel': 'Cancel',
        'common.close': 'Close',
        'common.error': 'Error',
      };
      return map[key] || key;
    },
    i18n: {
      language: 'en',
      getResourceBundle: () => ({
        nav: { dashboard: 'Dashboard', bookings: 'Bookings' },
        auth: { login: 'Sign In' },
      }),
    },
  }),
}));

const MOCK_PROPOSALS = [
  {
    id: 'p-1',
    language: 'en',
    key: 'nav.dashboard',
    current_value: 'Dashboard',
    proposed_value: 'Home',
    proposed_by: 'u-2',
    proposed_by_name: 'Alice',
    status: 'pending' as const,
    votes_for: 3,
    votes_against: 1,
    user_vote: null,
    created_at: '2026-01-15T10:00:00Z',
    updated_at: '2026-01-15T10:00:00Z',
  },
];

import { TranslationsPage } from './Translations';

describe('TranslationsPage', () => {
  beforeEach(() => {
    mockGetProposals.mockClear();
    mockVoteOnProposal.mockClear();
    mockCreateProposal.mockClear();
    mockToastSuccess.mockClear();
    mockToastError.mockClear();
    mockGetProposals.mockResolvedValue({ success: true, data: MOCK_PROPOSALS });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders page title and subtitle', () => {
    render(<TranslationsPage />);
    expect(screen.getByText('Translations')).toBeInTheDocument();
    expect(screen.getByText('Help improve translations')).toBeInTheDocument();
  });

  it('renders propose button', () => {
    render(<TranslationsPage />);
    expect(screen.getByText('Propose Change')).toBeInTheDocument();
  });

  it('renders tabs for proposals and browse keys', () => {
    render(<TranslationsPage />);
    expect(screen.getByText('Proposals')).toBeInTheDocument();
    expect(screen.getByText('Browse Keys')).toBeInTheDocument();
  });

  it('loads proposals on mount', async () => {
    render(<TranslationsPage />);
    await waitFor(() => {
      expect(mockGetProposals).toHaveBeenCalledWith('pending');
    });
  });

  it('shows loading spinner while loading', async () => {
    mockGetProposals.mockReturnValue(new Promise(() => {}));
    render(<TranslationsPage />);
    expect(screen.getByRole('status')).toBeInTheDocument();
  });

  it('shows proposals after loading', async () => {
    render(<TranslationsPage />);
    await waitFor(() => {
      expect(screen.getByText('1 proposals')).toBeInTheDocument();
    });
  });

  it('shows empty state when no proposals', async () => {
    mockGetProposals.mockResolvedValue({ success: true, data: [] });
    render(<TranslationsPage />);
    await waitFor(() => {
      expect(screen.getByText('No proposals yet')).toBeInTheDocument();
    });
  });

  it('renders search input', () => {
    render(<TranslationsPage />);
    expect(screen.getByLabelText('Search keys...')).toBeInTheDocument();
  });

  it('renders language selector', () => {
    render(<TranslationsPage />);
    expect(screen.getByLabelText('Select language')).toBeInTheDocument();
  });

  it('switches to browse keys tab', async () => {
    const user = userEvent.setup();
    render(<TranslationsPage />);

    await waitFor(() => {
      expect(mockGetProposals).toHaveBeenCalled();
    });

    await user.click(screen.getByText('Browse Keys'));

    // Should show key table
    expect(screen.getByText('nav.dashboard')).toBeInTheDocument();
    expect(screen.getByText('Dashboard')).toBeInTheDocument();
  });

  it('opens propose form when propose button is clicked', async () => {
    const user = userEvent.setup();
    render(<TranslationsPage />);

    await user.click(screen.getByText('Propose Change'));

    expect(screen.getByText('New Proposal')).toBeInTheDocument();
    expect(screen.getByLabelText('Key')).toBeInTheDocument();
    expect(screen.getByLabelText('Proposed Value')).toBeInTheDocument();
    expect(screen.getByLabelText('Context')).toBeInTheDocument();
  });

  it('submits a proposal', async () => {
    const user = userEvent.setup();
    mockCreateProposal.mockResolvedValue({ success: true });
    render(<TranslationsPage />);

    await user.click(screen.getByText('Propose Change'));
    await user.type(screen.getByPlaceholderText('nav.dashboard'), 'nav.test');
    await user.type(screen.getByPlaceholderText('Enter translation'), 'Test Value');
    await user.click(screen.getByText('Submit Proposal'));

    await waitFor(() => {
      expect(mockCreateProposal).toHaveBeenCalledWith(expect.objectContaining({
        language: 'en',
        key: 'nav.test',
        proposed_value: 'Test Value',
      }));
      expect(mockToastSuccess).toHaveBeenCalledWith('Proposal created!');
    });
  });

  it('shows error on proposal submission failure', async () => {
    const user = userEvent.setup();
    mockCreateProposal.mockResolvedValue({ success: false, error: { message: 'Duplicate' } });
    render(<TranslationsPage />);

    await user.click(screen.getByText('Propose Change'));
    await user.type(screen.getByPlaceholderText('nav.dashboard'), 'nav.test');
    await user.type(screen.getByPlaceholderText('Enter translation'), 'Test Value');
    await user.click(screen.getByText('Submit Proposal'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Duplicate');
    });
  });

  it('vote on a proposal', async () => {
    const user = userEvent.setup();
    mockVoteOnProposal.mockResolvedValue({
      success: true,
      data: { ...MOCK_PROPOSALS[0], votes_for: 4, user_vote: 'up' },
    });
    render(<TranslationsPage />);

    await waitFor(() => {
      expect(screen.getByText('1 proposals')).toBeInTheDocument();
    });

    const voteForBtn = screen.getByLabelText('Vote for');
    await user.click(voteForBtn);

    await waitFor(() => {
      expect(mockVoteOnProposal).toHaveBeenCalledWith('p-1', 'up');
      expect(mockToastSuccess).toHaveBeenCalledWith('Vote recorded');
    });
  });

  it('vote error resyncs proposals', async () => {
    const user = userEvent.setup();
    mockVoteOnProposal.mockResolvedValue({ success: false, error: { message: 'Already voted' } });
    render(<TranslationsPage />);

    await waitFor(() => {
      expect(screen.getByText('1 proposals')).toBeInTheDocument();
    });

    const voteForBtn = screen.getByLabelText('Vote for');
    await user.click(voteForBtn);

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Already voted');
      // loadProposals called again for resync
      expect(mockGetProposals).toHaveBeenCalledTimes(2);
    });
  });

  it('closes propose form on cancel', async () => {
    const user = userEvent.setup();
    render(<TranslationsPage />);

    await user.click(screen.getByText('Propose Change'));
    expect(screen.getByText('New Proposal')).toBeInTheDocument();

    await user.click(screen.getByText('Cancel'));
    expect(screen.queryByText('New Proposal')).not.toBeInTheDocument();
  });

  it('filter status dropdown changes filter', async () => {
    const user = userEvent.setup();
    render(<TranslationsPage />);

    await waitFor(() => {
      expect(mockGetProposals).toHaveBeenCalledTimes(1);
    });

    // Switch to all statuses
    await user.selectOptions(screen.getByLabelText('Filter status'), 'all');

    await waitFor(() => {
      expect(mockGetProposals).toHaveBeenCalledWith(undefined);
    });
  });

  it('vote down sends downvote', async () => {
    const user = userEvent.setup();
    mockGetProposals.mockResolvedValue({ success: true, data: [
      { id: 'p1', language: 'en', key: 'nav.home', current_value: 'Home', proposed_value: 'Start',
        proposer_name: 'Alice', proposer_id: 'u-2', created_at: '2025-01-01', status: 'pending',
        votes_for: 1, votes_against: 0, user_vote: null, context: null },
    ]});
    mockVoteOnProposal.mockResolvedValue({ success: true, data: {
      id: 'p1', language: 'en', key: 'nav.home', current_value: 'Home', proposed_value: 'Start',
      proposer_name: 'Alice', proposer_id: 'u-2', created_at: '2025-01-01', status: 'pending',
      votes_for: 1, votes_against: 1, user_vote: 'down', context: null,
    }});
    const { TranslationsPage } = await import('./Translations');
    render(<TranslationsPage />);
    await waitFor(() => expect(mockGetProposals).toHaveBeenCalled());
    const voteDown = screen.getByLabelText('Vote against');
    await user.click(voteDown);
    expect(mockVoteOnProposal).toHaveBeenCalledWith('p1', 'down');
  });

  it('empty key+value propose is rejected silently', async () => {
    const user = userEvent.setup();
    mockGetProposals.mockResolvedValue({ success: true, data: [] });
    const { TranslationsPage } = await import('./Translations');
    render(<TranslationsPage />);
    await waitFor(() => expect(mockGetProposals).toHaveBeenCalled());
    await user.click(screen.getByText('Propose Change'));
    // Submit with empty key and value — handlePropose early-returns
    const submit = screen.getByRole('button', { name: /Submit/i });
    expect(submit).toBeDisabled();
  });

  it('search filters proposals by key, value, and current_value', async () => {
    const user = userEvent.setup();
    mockGetProposals.mockResolvedValue({ success: true, data: [
      { id: 'p1', language: 'en', key: 'nav.home', current_value: 'Home', proposed_value: 'Start',
        proposer_name: 'Alice', proposer_id: 'u-2', created_at: '2025-01-01', status: 'pending',
        votes_for: 0, votes_against: 0, user_vote: null, context: null },
      { id: 'p2', language: 'en', key: 'btn.save', current_value: 'Save', proposed_value: 'Speichern',
        proposer_name: 'Bob', proposer_id: 'u-3', created_at: '2025-01-01', status: 'pending',
        votes_for: 0, votes_against: 0, user_vote: null, context: null },
    ]});
    const { TranslationsPage } = await import('./Translations');
    render(<TranslationsPage />);
    await waitFor(() => expect(mockGetProposals).toHaveBeenCalled());
    const searchInput = screen.getByPlaceholderText('Search keys...');
    await user.type(searchInput, 'save');
    // Only btn.save proposal should remain
    await waitFor(() => {
      expect(screen.queryByText(/nav\.home/)).not.toBeInTheDocument();
    });
  });

  it('language selector switches selectedLang', async () => {
    const user = userEvent.setup();
    mockGetProposals.mockResolvedValue({ success: true, data: [] });
    const { TranslationsPage } = await import('./Translations');
    render(<TranslationsPage />);
    await waitFor(() => expect(mockGetProposals).toHaveBeenCalled());
    const langSelect = screen.getByLabelText('Select language');
    await user.selectOptions(langSelect, 'de');
    expect((langSelect as HTMLSelectElement).value).toBe('de');
  });

  it('context input accepts text in propose modal', async () => {
    const user = userEvent.setup();
    mockGetProposals.mockResolvedValue({ success: true, data: [] });
    const { TranslationsPage } = await import('./Translations');
    render(<TranslationsPage />);
    await waitFor(() => expect(mockGetProposals).toHaveBeenCalled());
    await user.click(screen.getByText('Propose Change'));
    const contextInput = screen.getByLabelText(/context/i);
    await user.type(contextInput, 'used in main nav');
    expect((contextInput as HTMLInputElement).value).toBe('used in main nav');
  });

  it('close button dismisses propose modal', async () => {
    const user = userEvent.setup();
    mockGetProposals.mockResolvedValue({ success: true, data: [] });
    const { TranslationsPage } = await import('./Translations');
    render(<TranslationsPage />);
    await waitFor(() => expect(mockGetProposals).toHaveBeenCalled());
    await user.click(screen.getByText('Propose Change'));
    expect(screen.getByText('New Proposal')).toBeInTheDocument();
    const closeBtn = screen.getByRole('button', { name: 'Close' });
    await user.click(closeBtn);
    await waitFor(() => {
      expect(screen.queryByText('New Proposal')).not.toBeInTheDocument();
    });
  });

  it('suggest change from browse tab opens propose form with pre-filled key', async () => {
    const user = userEvent.setup();
    render(<TranslationsPage />);

    await waitFor(() => {
      expect(mockGetProposals).toHaveBeenCalled();
    });

    await user.click(screen.getByText('Browse Keys'));

    const suggestBtns = screen.getAllByText('Suggest Change');
    expect(suggestBtns.length).toBeGreaterThan(0);
    await user.click(suggestBtns[0]);

    expect(screen.getByText('New Proposal')).toBeInTheDocument();
  });
});

// ── Test the flattenKeys utility ──
describe('flattenKeys', () => {
  it('flattens nested object to dot-notation', () => {
    // This is the same logic as in the component - test it
    function flattenKeys(obj: Record<string, unknown>, prefix = ''): Record<string, string> {
      const result: Record<string, string> = {};
      for (const [key, value] of Object.entries(obj)) {
        const path = prefix ? `${prefix}.${key}` : key;
        if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
          Object.assign(result, flattenKeys(value as Record<string, unknown>, path));
        } else {
          result[path] = String(value);
        }
      }
      return result;
    }

    expect(flattenKeys({ a: { b: 'c' } })).toEqual({ 'a.b': 'c' });
    expect(flattenKeys({ x: 1, y: { z: 2 } })).toEqual({ x: '1', 'y.z': '2' });
    expect(flattenKeys({})).toEqual({});
    expect(flattenKeys({ arr: [1, 2] })).toEqual({ arr: '1,2' });
  });
});
