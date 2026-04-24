import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

// v5 Benachrichtigungen is an admin-nav entry — it broadcasts
// system-wide announcements to the whole tenant. It must call the
// /api/v1/admin/announcements endpoints, NOT the per-user notification
// inbox (that feed is surfaced by the bell dropdown + NotificationCenter).
const mockList = vi.fn();
const mockCreate = vi.fn();
const mockUpdate = vi.fn();
const mockDelete = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    adminListAnnouncements: (...a: unknown[]) => mockList(...a),
    adminCreateAnnouncement: (...a: unknown[]) => mockCreate(...a),
    adminUpdateAnnouncement: (...a: unknown[]) => mockUpdate(...a),
    adminDeleteAnnouncement: (...a: unknown[]) => mockDelete(...a),
  },
}));

vi.mock('@number-flow/react', () => ({
  default: ({ value }: { value: number }) => <span>{value}</span>,
}));

const mockToast = vi.fn();
vi.mock('../Toast', () => ({
  useV5Toast: () => mockToast,
  V5ToastProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

import { BenachrichtigungenV5 } from './Benachrichtigungen';

const A1 = {
  id: 'a1', title: 'Wartung 1. Mai', message: 'Lot 3 geschlossen',
  severity: 'warning', active: true, created_at: '2026-04-23T10:00:00Z',
};
const A2 = {
  id: 'a2', title: 'Alte Info', message: 'Ignorieren',
  severity: 'info', active: false, created_at: '2026-04-10T10:00:00Z',
};

function renderScreen() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <BenachrichtigungenV5 navigate={vi.fn()} />
    </QueryClientProvider>
  );
}

describe('BenachrichtigungenV5 (admin)', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state', async () => {
    mockList.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine Ankündigungen')).toBeInTheDocument());
  });

  it('renders error when query fails', async () => {
    mockList.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders one row per announcement with active + inactive badges', async () => {
    mockList.mockResolvedValue({ success: true, data: [A1, A2] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('benach-row')).toHaveLength(2));
    expect(screen.getByText('Wartung 1. Mai')).toBeInTheDocument();
    expect(screen.getByText('Alte Info')).toBeInTheDocument();
  });

  it('creates a new announcement via adminCreateAnnouncement', async () => {
    mockList.mockResolvedValue({ success: true, data: [] });
    mockCreate.mockResolvedValue({ success: true, data: { ...A1, id: 'a-new' } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('benach-new-title')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('benach-new-title'), { target: { value: 'Neu' } });
    fireEvent.change(screen.getByTestId('benach-new-message'), { target: { value: 'Hallo Team' } });
    fireEvent.click(screen.getByTestId('benach-new-submit'));
    await waitFor(() => {
      expect(mockCreate).toHaveBeenCalledWith(expect.objectContaining({
        title: 'Neu',
        message: 'Hallo Team',
        active: true,
      }));
      expect(mockToast).toHaveBeenCalledWith('Ankündigung erstellt', 'success');
    });
  });

  it('surfaces create error when success:false', async () => {
    mockList.mockResolvedValue({ success: true, data: [] });
    mockCreate.mockResolvedValue({ success: false, data: null, error: { code: 'E', message: 'fail' } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('benach-new-title')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('benach-new-title'), { target: { value: 'x' } });
    fireEvent.change(screen.getByTestId('benach-new-message'), { target: { value: 'y' } });
    fireEvent.click(screen.getByTestId('benach-new-submit'));
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('fail', 'error'));
  });

  it('deletes an announcement via adminDeleteAnnouncement', async () => {
    mockList.mockResolvedValue({ success: true, data: [A1] });
    mockDelete.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByLabelText('Ankündigung a1 löschen')).toBeInTheDocument());
    fireEvent.click(screen.getByLabelText('Ankündigung a1 löschen'));
    await waitFor(() => {
      expect(mockDelete).toHaveBeenCalledWith('a1');
      expect(mockToast).toHaveBeenCalledWith('Ankündigung gelöscht', 'success');
    });
  });
});
