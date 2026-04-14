import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockAdminListAnnouncements = vi.fn();
const mockAdminCreateAnnouncement = vi.fn();
const mockAdminDeleteAnnouncement = vi.fn();
const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    adminListAnnouncements: (...args: any[]) => mockAdminListAnnouncements(...args),
    adminCreateAnnouncement: (...args: any[]) => mockAdminCreateAnnouncement(...args),
    adminUpdateAnnouncement: vi.fn(),
    adminDeleteAnnouncement: (...args: any[]) => mockAdminDeleteAnnouncement(...args),
  },
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
  Megaphone: (props: any) => <span data-testid="icon-megaphone" {...props} />,
  Plus: (props: any) => <span data-testid="icon-plus" {...props} />,
  PencilSimple: (props: any) => <span data-testid="icon-pencil" {...props} />,
  Trash: (props: any) => <span data-testid="icon-trash" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  Info: (props: any) => <span data-testid="icon-info" {...props} />,
  Warning: (props: any) => <span data-testid="icon-warning" {...props} />,
  WarningCircle: (props: any) => <span data-testid="icon-warning-circle" {...props} />,
  CheckCircle: (props: any) => <span data-testid="icon-check-circle" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: {
    success: (...args: any[]) => mockToastSuccess(...args),
    error: (...args: any[]) => mockToastError(...args),
  },
}));

import { AdminAnnouncementsPage } from './AdminAnnouncements';

describe('AdminAnnouncementsPage', () => {
  beforeEach(() => {
    mockAdminListAnnouncements.mockClear();
    mockAdminCreateAnnouncement.mockClear();
    mockAdminDeleteAnnouncement.mockClear();
    mockToastSuccess.mockClear();
    mockToastError.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('shows loading spinner initially', () => {
    mockAdminListAnnouncements.mockReturnValue(new Promise(() => {}));
    render(<AdminAnnouncementsPage />);
    expect(screen.getByTestId('icon-spinner')).toBeInTheDocument();
  });

  it('renders empty state when no announcements', async () => {
    mockAdminListAnnouncements.mockResolvedValue({ success: true, data: [] });
    render(<AdminAnnouncementsPage />);

    await waitFor(() => {
      expect(screen.getByText('No announcements yet.')).toBeInTheDocument();
    });
  });

  it('renders announcements heading and new button', async () => {
    mockAdminListAnnouncements.mockResolvedValue({ success: true, data: [] });
    render(<AdminAnnouncementsPage />);

    await waitFor(() => {
      expect(screen.getByText('Announcements')).toBeInTheDocument();
    });
    expect(screen.getByText('New Announcement')).toBeInTheDocument();
  });

  it('renders announcement cards', async () => {
    mockAdminListAnnouncements.mockResolvedValue({
      success: true,
      data: [
        {
          id: 'a-1',
          title: 'Maintenance Tonight',
          message: 'System will be down from 2-4am',
          severity: 'warning',
          active: true,
          created_at: '2026-03-15T00:00:00Z',
        },
        {
          id: 'a-2',
          title: 'New Feature',
          message: 'Credits system is now live',
          severity: 'info',
          active: true,
          created_at: '2026-03-14T00:00:00Z',
        },
      ],
    });

    render(<AdminAnnouncementsPage />);

    await waitFor(() => {
      expect(screen.getByText('Maintenance Tonight')).toBeInTheDocument();
    });
    expect(screen.getByText('System will be down from 2-4am')).toBeInTheDocument();
    expect(screen.getByText('New Feature')).toBeInTheDocument();
    expect(screen.getByText('Credits system is now live')).toBeInTheDocument();
  });

  it('opens the create form when clicking New Announcement', async () => {
    mockAdminListAnnouncements.mockResolvedValue({ success: true, data: [] });
    const user = userEvent.setup();
    render(<AdminAnnouncementsPage />);

    await waitFor(() => {
      expect(screen.getByText('New Announcement')).toBeInTheDocument();
    });

    await user.click(screen.getByText('New Announcement'));

    expect(screen.getByPlaceholderText('Title')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Message')).toBeInTheDocument();
    expect(screen.getByText('Severity')).toBeInTheDocument();
    expect(screen.getByText('Status')).toBeInTheDocument();
  });

  it('shows severity buttons in the form', async () => {
    mockAdminListAnnouncements.mockResolvedValue({ success: true, data: [] });
    const user = userEvent.setup();
    render(<AdminAnnouncementsPage />);

    await waitFor(() => {
      expect(screen.getByText('New Announcement')).toBeInTheDocument();
    });

    await user.click(screen.getByText('New Announcement'));

    expect(screen.getByText('Info')).toBeInTheDocument();
    expect(screen.getByText('Warning')).toBeInTheDocument();
    expect(screen.getByText('Error')).toBeInTheDocument();
    expect(screen.getByText('Success')).toBeInTheDocument();
  });

  it('shows severity and status badges on announcements', async () => {
    mockAdminListAnnouncements.mockResolvedValue({
      success: true,
      data: [
        {
          id: 'a-1',
          title: 'Test',
          message: 'Test message',
          severity: 'info',
          active: true,
          created_at: '2026-03-15T00:00:00Z',
        },
      ],
    });

    render(<AdminAnnouncementsPage />);

    await waitFor(() => {
      expect(screen.getByText('Test')).toBeInTheDocument();
    });
    expect(screen.getByText('Info')).toBeInTheDocument();
    expect(screen.getByText('Active')).toBeInTheDocument();
  });

  it('shows inactive badge for inactive announcements', async () => {
    mockAdminListAnnouncements.mockResolvedValue({
      success: true,
      data: [
        { id: 'a-1', title: 'Disabled', message: 'Msg', severity: 'warning', active: false, created_at: '2026-03-15T00:00:00Z' },
      ],
    });
    render(<AdminAnnouncementsPage />);
    await waitFor(() => {
      expect(screen.getByText('Disabled')).toBeInTheDocument();
      expect(screen.getByText('Inactive')).toBeInTheDocument();
    });
  });

  it('shows expired badge for expired announcements', async () => {
    mockAdminListAnnouncements.mockResolvedValue({
      success: true,
      data: [
        { id: 'a-1', title: 'Old', message: 'Old msg', severity: 'info', active: true, created_at: '2026-01-01T00:00:00Z', expires_at: '2026-01-02T00:00:00Z' },
      ],
    });
    render(<AdminAnnouncementsPage />);
    await waitFor(() => {
      expect(screen.getByText('Old')).toBeInTheDocument();
      expect(screen.getByText('Expired')).toBeInTheDocument();
    });
  });

  it('creates an announcement successfully', async () => {
    mockAdminListAnnouncements.mockResolvedValue({ success: true, data: [] });
    mockAdminCreateAnnouncement.mockResolvedValue({ success: true, data: { id: 'new-1', title: 'New One' } });
    const user = userEvent.setup();
    render(<AdminAnnouncementsPage />);

    await waitFor(() => expect(screen.getByText('New Announcement')).toBeInTheDocument());
    await user.click(screen.getByText('New Announcement'));
    await user.type(screen.getByPlaceholderText('Title'), 'Test Title');
    await user.type(screen.getByPlaceholderText('Message'), 'Test Message');

    // Reset for reload
    mockAdminListAnnouncements.mockResolvedValue({ success: true, data: [{ id: 'new-1', title: 'Test Title', message: 'Test Message', severity: 'info', active: true, created_at: new Date().toISOString() }] });
    await user.click(screen.getByText('Create'));

    await waitFor(() => {
      expect(mockAdminCreateAnnouncement).toHaveBeenCalled();
      expect(mockToastSuccess).toHaveBeenCalled();
    });
  });

  it('shows error toast when title is empty on save', async () => {
    mockAdminListAnnouncements.mockResolvedValue({ success: true, data: [] });
    const user = userEvent.setup();
    render(<AdminAnnouncementsPage />);

    await waitFor(() => expect(screen.getByText('New Announcement')).toBeInTheDocument());
    await user.click(screen.getByText('New Announcement'));
    await user.click(screen.getByText('Create'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalled();
    });
    expect(mockAdminCreateAnnouncement).not.toHaveBeenCalled();
  });

  it('shows error toast on failed create', async () => {
    mockAdminListAnnouncements.mockResolvedValue({ success: true, data: [] });
    mockAdminCreateAnnouncement.mockResolvedValue({ success: false, data: null, error: { code: 'ERR', message: 'Save failed' } });
    const user = userEvent.setup();
    render(<AdminAnnouncementsPage />);

    await waitFor(() => expect(screen.getByText('New Announcement')).toBeInTheDocument());
    await user.click(screen.getByText('New Announcement'));
    await user.type(screen.getByPlaceholderText('Title'), 'T');
    await user.type(screen.getByPlaceholderText('Message'), 'M');
    await user.click(screen.getByText('Create'));

    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Save failed');
    });
  });

  it('opens edit form and populates fields', async () => {
    mockAdminListAnnouncements.mockResolvedValue({
      success: true,
      data: [{ id: 'a-1', title: 'Editable', message: 'Edit me', severity: 'warning', active: true, created_at: '2026-03-15T00:00:00Z' }],
    });
    const user = userEvent.setup();
    render(<AdminAnnouncementsPage />);

    await waitFor(() => expect(screen.getByText('Editable')).toBeInTheDocument());
    const editBtn = screen.getByLabelText(/Edit Editable/i);
    await user.click(editBtn);

    await waitFor(() => {
      expect(screen.getByDisplayValue('Editable')).toBeInTheDocument();
      expect(screen.getByDisplayValue('Edit me')).toBeInTheDocument();
    });
  });

  it('closes form on close button click', async () => {
    mockAdminListAnnouncements.mockResolvedValue({ success: true, data: [] });
    const user = userEvent.setup();
    render(<AdminAnnouncementsPage />);

    await waitFor(() => expect(screen.getByText('New Announcement')).toBeInTheDocument());
    await user.click(screen.getByText('New Announcement'));
    expect(screen.getByPlaceholderText('Title')).toBeInTheDocument();

    await user.click(screen.getByLabelText('Close'));
    expect(screen.queryByPlaceholderText('Title')).not.toBeInTheDocument();
  });

  it('closes form on cancel button click', async () => {
    mockAdminListAnnouncements.mockResolvedValue({ success: true, data: [] });
    const user = userEvent.setup();
    render(<AdminAnnouncementsPage />);

    await waitFor(() => expect(screen.getByText('New Announcement')).toBeInTheDocument());
    await user.click(screen.getByText('New Announcement'));
    await user.click(screen.getByText('Cancel'));
    expect(screen.queryByPlaceholderText('Title')).not.toBeInTheDocument();
  });

  it('toggles status in the form', async () => {
    mockAdminListAnnouncements.mockResolvedValue({ success: true, data: [] });
    const user = userEvent.setup();
    render(<AdminAnnouncementsPage />);

    await waitFor(() => expect(screen.getByText('New Announcement')).toBeInTheDocument());
    await user.click(screen.getByText('New Announcement'));

    // Status starts as Active, toggle to Inactive
    await user.click(screen.getByText('Active'));
    expect(screen.getByText('Inactive')).toBeInTheDocument();
  });

  it('displays all four severity types in announcement cards', async () => {
    mockAdminListAnnouncements.mockResolvedValue({
      success: true,
      data: [
        { id: 'a-1', title: 'Info One', message: 'M1', severity: 'info', active: true, created_at: '2026-03-15T00:00:00Z' },
        { id: 'a-2', title: 'Warn One', message: 'M2', severity: 'warning', active: true, created_at: '2026-03-15T00:00:00Z' },
        { id: 'a-3', title: 'Error One', message: 'M3', severity: 'error', active: true, created_at: '2026-03-15T00:00:00Z' },
        { id: 'a-4', title: 'Success One', message: 'M4', severity: 'success', active: true, created_at: '2026-03-15T00:00:00Z' },
      ],
    });
    render(<AdminAnnouncementsPage />);
    await waitFor(() => {
      expect(screen.getByText('Info One')).toBeInTheDocument();
      expect(screen.getByText('Warn One')).toBeInTheDocument();
      expect(screen.getByText('Error One')).toBeInTheDocument();
      expect(screen.getByText('Success One')).toBeInTheDocument();
    });
  });

  it('handles API failure gracefully', async () => {
    mockAdminListAnnouncements.mockResolvedValue({ success: false, data: null });
    render(<AdminAnnouncementsPage />);
    await waitFor(() => {
      expect(screen.getByText('No announcements yet.')).toBeInTheDocument();
    });
  });

  it('shows expires_at date when present', async () => {
    mockAdminListAnnouncements.mockResolvedValue({
      success: true,
      data: [
        { id: 'a-1', title: 'Expiring', message: 'M', severity: 'info', active: true, created_at: '2026-03-15T00:00:00Z', expires_at: '2026-12-31T00:00:00Z' },
      ],
    });
    render(<AdminAnnouncementsPage />);
    await waitFor(() => {
      expect(screen.getByText('Expiring')).toBeInTheDocument();
      // Should show expiry date text
      expect(screen.getByText(/Expires/i, { exact: false })).toBeInTheDocument();
    });
  });
});
