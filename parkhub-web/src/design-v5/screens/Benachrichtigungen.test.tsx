import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockList = vi.fn();
const mockRead = vi.fn();
const mockReadAll = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    getNotifications: (...a: unknown[]) => mockList(...a),
    markNotificationRead: (...a: unknown[]) => mockRead(...a),
    markAllNotificationsRead: (...a: unknown[]) => mockReadAll(...a),
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

const N1 = { id: 'n1', title: 'Neue Buchung', message: 'Platz A3', notification_type: 'booking', read: false, created_at: '2026-04-23T10:00:00Z' };
const N2 = { id: 'n2', title: 'Tauschanfrage', message: 'Swap', notification_type: 'swap', read: true, created_at: '2026-04-22T10:00:00Z' };

function renderScreen() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <BenachrichtigungenV5 navigate={vi.fn()} />
    </QueryClientProvider>
  );
}

describe('BenachrichtigungenV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state', async () => {
    mockList.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine Benachrichtigungen')).toBeInTheDocument());
  });

  it('renders error when query fails', async () => {
    mockList.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders rows and shows unread count', async () => {
    mockList.mockResolvedValue({ success: true, data: [N1, N2] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('benach-row')).toHaveLength(2));
    expect(screen.getByText('1 ungelesen')).toBeInTheDocument();
  });

  it('filters to unread only', async () => {
    mockList.mockResolvedValue({ success: true, data: [N1, N2] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('benach-row')).toHaveLength(2));
    fireEvent.click(screen.getByRole('button', { name: 'Ungelesen' }));
    await waitFor(() => expect(screen.getAllByTestId('benach-row')).toHaveLength(1));
  });

  it('mark-read mutation fires for individual item', async () => {
    mockList.mockResolvedValue({ success: true, data: [N1] });
    mockRead.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByLabelText('Benachrichtigung n1 als gelesen markieren')).toBeInTheDocument());
    fireEvent.click(screen.getByLabelText('Benachrichtigung n1 als gelesen markieren'));
    await waitFor(() => {
      expect(mockRead).toHaveBeenCalledWith('n1');
      expect(mockToast).toHaveBeenCalledWith('Als gelesen markiert', 'success');
    });
  });

  it('mark-read error surfaces via toast', async () => {
    mockList.mockResolvedValue({ success: true, data: [N1] });
    mockRead.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'fail' } });
    renderScreen();
    await waitFor(() => expect(screen.getByLabelText('Benachrichtigung n1 als gelesen markieren')).toBeInTheDocument());
    fireEvent.click(screen.getByLabelText('Benachrichtigung n1 als gelesen markieren'));
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('fail', 'error'));
  });

  it('mark-all button calls markAllNotificationsRead', async () => {
    mockList.mockResolvedValue({ success: true, data: [N1, N2] });
    mockReadAll.mockResolvedValue({ success: true, data: null });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('benach-mark-all')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('benach-mark-all'));
    await waitFor(() => {
      expect(mockReadAll).toHaveBeenCalled();
      expect(mockToast).toHaveBeenCalledWith('Alle als gelesen markiert', 'success');
    });
  });
});
