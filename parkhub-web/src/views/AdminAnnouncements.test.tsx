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
});
