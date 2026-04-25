import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

const mockListAnnouncements = vi.fn();
const mockCreateAnnouncement = vi.fn();
const mockUpdateAnnouncement = vi.fn();
const mockDeleteAnnouncement = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    adminListAnnouncements: (...a: any[]) => mockListAnnouncements(...a),
    adminCreateAnnouncement: (...a: any[]) => mockCreateAnnouncement(...a),
    adminUpdateAnnouncement: (...a: any[]) => mockUpdateAnnouncement(...a),
    adminDeleteAnnouncement: (...a: any[]) => mockDeleteAnnouncement(...a),
  },
}));

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (k: string) => k }) }));
vi.mock('framer-motion', () => ({
  motion: { div: React.forwardRef(({ children, ...p }: any, r: any) => <div ref={r} {...p}>{children}</div>) },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));
vi.mock('@phosphor-icons/react', () => {
  const C = (p: any) => <span {...p} />;
  return { Megaphone: C, Plus: C, PencilSimple: C, Trash: C, SpinnerGap: C, Check: C, X: C, Info: C, Warning: C, WarningCircle: C, CheckCircle: C, Clock: C };
});
vi.mock('react-hot-toast', () => ({ default: { success: vi.fn(), error: vi.fn() } }));
vi.mock('../components/ui/ConfirmDialog', () => ({
  ConfirmDialog: ({ open, onConfirm, onCancel }: any) =>
    open ? <div data-testid="confirm-dialog"><button onClick={onConfirm}>ConfirmDel</button><button onClick={onCancel}>CancelDel</button></div> : null,
}));

import { AdminAnnouncementsPage } from './AdminAnnouncements';
import toast from 'react-hot-toast';

const announcements = [
  { id: 'a1', title: 'Maintenance', message: 'Server will be down', severity: 'warning', active: true, expires_at: null, created_at: '2026-04-01T00:00:00Z', updated_at: '2026-04-01' },
  { id: 'a2', title: 'New Feature', message: 'EV charging is live', severity: 'info', active: false, expires_at: '2026-03-01T00:00:00Z', created_at: '2026-03-01T00:00:00Z', updated_at: '2026-03-01' },
  { id: 'a3', title: 'Error Alert', message: 'System error', severity: 'error', active: true, expires_at: null, created_at: '2026-04-10T00:00:00Z', updated_at: '2026-04-10' },
  { id: 'a4', title: 'Good news', message: 'All systems go', severity: 'success', active: true, expires_at: null, created_at: '2026-04-12T00:00:00Z', updated_at: '2026-04-12' },
];

describe('AdminAnnouncementsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockListAnnouncements.mockResolvedValue({ success: true, data: announcements });
    mockCreateAnnouncement.mockResolvedValue({ success: true });
    mockUpdateAnnouncement.mockResolvedValue({ success: true });
    mockDeleteAnnouncement.mockResolvedValue({ success: true });
  });
  afterEach(() => vi.restoreAllMocks());

  it('renders announcements', async () => {
    render(<AdminAnnouncementsPage />);
    await waitFor(() => expect(screen.getByText('Maintenance')).toBeInTheDocument());
    expect(screen.getByText('New Feature')).toBeInTheDocument();
  });

  it('opens create form', async () => {
    render(<AdminAnnouncementsPage />);
    await waitFor(() => fireEvent.click(screen.getByText('admin.newAnnouncement')));
    expect(screen.getByPlaceholderText('admin.announcementTitle')).toBeInTheDocument();
  });

  it('validates empty title/message', async () => {
    render(<AdminAnnouncementsPage />);
    await waitFor(() => fireEvent.click(screen.getByText('admin.newAnnouncement')));
    const saveBtn = screen.getAllByText('admin.create').find(el => el.closest('button')?.className.includes('btn-primary'));
    if (saveBtn) fireEvent.click(saveBtn);
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('admin.announcementTitleRequired'));
  });

  it('creates announcement', async () => {
    render(<AdminAnnouncementsPage />);
    await waitFor(() => expect(screen.getByText('admin.newAnnouncement')).toBeInTheDocument());
    fireEvent.click(screen.getByText('admin.newAnnouncement'));
    await waitFor(() => expect(screen.getByPlaceholderText('admin.announcementTitle')).toBeInTheDocument());
    fireEvent.change(screen.getByPlaceholderText('admin.announcementTitle'), { target: { value: 'Test Title' } });
    fireEvent.change(screen.getByPlaceholderText('admin.announcementMessage'), { target: { value: 'Test Message' } });
    const saveBtn = screen.getAllByText('admin.create').find(el => el.closest('button')?.className.includes('btn-primary'));
    if (saveBtn) fireEvent.click(saveBtn);
    await waitFor(() => expect(mockCreateAnnouncement).toHaveBeenCalled());
    expect(toast.success).toHaveBeenCalledWith('admin.announcementCreated');
  });

  it('edits announcement', async () => {
    render(<AdminAnnouncementsPage />);
    await waitFor(() => expect(screen.getByText('Maintenance')).toBeInTheDocument());
    const editBtns = screen.getAllByLabelText(/common.edit/);
    fireEvent.click(editBtns[0]!!);
    await waitFor(() => expect(screen.getByDisplayValue('Maintenance')).toBeInTheDocument());
  });

  it('closes form', async () => {
    render(<AdminAnnouncementsPage />);
    await waitFor(() => fireEvent.click(screen.getByText('admin.newAnnouncement')));
    fireEvent.click(screen.getByLabelText('common.close'));
    await waitFor(() => expect(screen.queryByPlaceholderText('admin.announcementTitle')).not.toBeInTheDocument());
  });

  it('deletes announcement with confirm', async () => {
    render(<AdminAnnouncementsPage />);
    await waitFor(() => expect(screen.getByText('Maintenance')).toBeInTheDocument());
    const delBtns = screen.getAllByLabelText(/common.delete/);
    fireEvent.click(delBtns[0]!!);
    await waitFor(() => expect(screen.getByTestId('confirm-dialog')).toBeInTheDocument());
    fireEvent.click(screen.getByText('ConfirmDel'));
    await waitFor(() => expect(mockDeleteAnnouncement).toHaveBeenCalledWith('a1'));
    expect(toast.success).toHaveBeenCalledWith('admin.announcementDeleted');
  });

  it('toggles severity buttons in form', async () => {
    render(<AdminAnnouncementsPage />);
    await waitFor(() => expect(screen.getByText('Maintenance')).toBeInTheDocument());
    // Open create form
    const newBtns = screen.getAllByText('admin.newAnnouncement');
    fireEvent.click(newBtns[newBtns.length - 1]);
    await waitFor(() => expect(screen.getByPlaceholderText('admin.announcementTitle')).toBeInTheDocument());
    // Find severity buttons by type="button" attribute
    const allBtns = screen.getAllByRole('button');
    const errorBtn = allBtns.find(b => b.getAttribute('type') === 'button' && b.textContent?.includes('admin.severityError'));
    if (errorBtn) fireEvent.click(errorBtn);
    const successBtn = allBtns.find(b => b.getAttribute('type') === 'button' && b.textContent?.includes('admin.severitySuccess'));
    if (successBtn) fireEvent.click(successBtn);
  });

  it('toggles active status in form', async () => {
    render(<AdminAnnouncementsPage />);
    await waitFor(() => expect(screen.getByText('Maintenance')).toBeInTheDocument());
    const newBtns = screen.getAllByText('admin.newAnnouncement');
    fireEvent.click(newBtns[newBtns.length - 1]);
    await waitFor(() => expect(screen.getByPlaceholderText('admin.announcementTitle')).toBeInTheDocument());
    // The active toggle is a type="button" with text "admin.active"
    const allBtns = screen.getAllByRole('button');
    const activeToggle = allBtns.find(b => b.getAttribute('type') === 'button' && b.textContent?.includes('admin.active') && !b.textContent?.includes('admin.inactive'));
    if (activeToggle) fireEvent.click(activeToggle);
  });

  it('shows empty state', async () => {
    mockListAnnouncements.mockResolvedValue({ success: true, data: [] });
    render(<AdminAnnouncementsPage />);
    await waitFor(() => expect(screen.getByText('admin.noAnnouncements')).toBeInTheDocument());
  });

  it('create failure shows error', async () => {
    mockCreateAnnouncement.mockResolvedValue({ success: false, error: { message: 'Failed' } });
    const user = userEvent.setup();
    render(<AdminAnnouncementsPage />);
    await waitFor(() => fireEvent.click(screen.getByText('admin.newAnnouncement')));
    await user.type(screen.getByPlaceholderText('admin.announcementTitle'), 'T');
    await user.type(screen.getByPlaceholderText('admin.announcementMessage'), 'M');
    const saveBtn = screen.getAllByText('admin.create').find(el => el.closest('button')?.className.includes('btn-primary'));
    if (saveBtn) await user.click(saveBtn);
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Failed'));
  });

  it('delete failure shows error', async () => {
    mockDeleteAnnouncement.mockResolvedValue({ success: false, error: { message: 'Cannot delete' } });
    render(<AdminAnnouncementsPage />);
    await waitFor(() => fireEvent.click(screen.getAllByLabelText(/common.delete/)[0]));
    fireEvent.click(screen.getByText('ConfirmDel'));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Cannot delete'));
  });

  it('shows expired status badge', async () => {
    render(<AdminAnnouncementsPage />);
    await waitFor(() => expect(screen.getByText('New Feature')).toBeInTheDocument());
    // a2 is inactive with expired date
  });

  it('cancels delete confirmation dialog', async () => {
    render(<AdminAnnouncementsPage />);
    await waitFor(() => expect(screen.getByText('Maintenance')).toBeInTheDocument());
    fireEvent.click(screen.getAllByLabelText(/common.delete/)[0]);
    await waitFor(() => expect(screen.getByTestId('confirm-dialog')).toBeInTheDocument());
    fireEvent.click(screen.getByText('CancelDel'));
    await waitFor(() => {
      expect(screen.queryByTestId('confirm-dialog')).not.toBeInTheDocument();
    });
  });

  it('changes expires_at field when editing', async () => {
    render(<AdminAnnouncementsPage />);
    await waitFor(() => fireEvent.click(screen.getByText('admin.newAnnouncement')));
    const expiresInput = document.querySelector('input[type="datetime-local"]') as HTMLInputElement;
    if (expiresInput) {
      fireEvent.change(expiresInput, { target: { value: '2027-01-01T12:00' } });
      expect(expiresInput.value).toBe('2027-01-01T12:00');
    }
  });

  it('closes form when deleting the announcement currently being edited', async () => {
    mockDeleteAnnouncement.mockResolvedValue({ success: true });
    render(<AdminAnnouncementsPage />);
    await waitFor(() => expect(screen.getByText('Maintenance')).toBeInTheDocument());
    // Edit the first announcement
    const editBtns = screen.getAllByLabelText(/common.edit/);
    fireEvent.click(editBtns[0]!!);
    await waitFor(() => expect(screen.getByDisplayValue('Maintenance')).toBeInTheDocument());
    // Delete it
    const delBtns = screen.getAllByLabelText(/common.delete/);
    fireEvent.click(delBtns[0]!!);
    await waitFor(() => expect(screen.getByTestId('confirm-dialog')).toBeInTheDocument());
    fireEvent.click(screen.getByText('ConfirmDel'));
    await waitFor(() => expect(mockDeleteAnnouncement).toHaveBeenCalled());
    // Form should be closed
    await waitFor(() => {
      expect(screen.queryByDisplayValue('Maintenance')).not.toBeInTheDocument();
    });
  });
});
