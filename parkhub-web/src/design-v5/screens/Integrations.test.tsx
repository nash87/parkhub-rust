import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockList = vi.fn();
const mockConnect = vi.fn();
const mockDisconnect = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    getIntegrations: (...a: unknown[]) => mockList(...a),
    connectIntegration: (...a: unknown[]) => mockConnect(...a),
    disconnectIntegration: (...a: unknown[]) => mockDisconnect(...a),
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

import { IntegrationsV5 } from './Integrations';

const I_DISC = { id: 'slack', name: 'Slack', provider: 'slack', description: 'Benachrichtigungen', connected: false, connected_at: null, account_label: null };
const I_CONN = { id: 'gcal', name: 'Google Kalender', provider: 'google', description: 'Kalender sync', connected: true, connected_at: '2026-03-01T10:00:00Z', account_label: 'florian@example.com' };

function renderScreen() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <IntegrationsV5 navigate={vi.fn()} />
    </QueryClientProvider>
  );
}

describe('IntegrationsV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state', async () => {
    mockList.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine Integrationen verfügbar')).toBeInTheDocument());
  });

  it('renders error state', async () => {
    mockList.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'fail' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('lists cards with connection status', async () => {
    mockList.mockResolvedValue({ success: true, data: [I_DISC, I_CONN] });
    renderScreen();
    await waitFor(() => expect(screen.getAllByTestId('integrations-card')).toHaveLength(2));
    expect(screen.getByText('Slack')).toBeInTheDocument();
    expect(screen.getByText('Nicht verbunden')).toBeInTheDocument();
    expect(screen.getByText('Verbunden')).toBeInTheDocument();
  });

  it('connect mutation fires', async () => {
    mockList.mockResolvedValue({ success: true, data: [I_DISC] });
    mockConnect.mockResolvedValue({ success: true, data: { ...I_DISC, connected: true } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('integrations-connect')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('integrations-connect'));
    await waitFor(() => {
      expect(mockConnect).toHaveBeenCalledWith('slack');
      expect(mockToast).toHaveBeenCalledWith('Integration verbunden', 'success');
    });
  });

  it('connect error surfaces via toast', async () => {
    mockList.mockResolvedValue({ success: true, data: [I_DISC] });
    mockConnect.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('integrations-connect')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('integrations-connect'));
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('denied', 'error'));
  });

  it('disconnect mutation fires', async () => {
    mockList.mockResolvedValue({ success: true, data: [I_CONN] });
    mockDisconnect.mockResolvedValue({ success: true, data: { ...I_CONN, connected: false } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('integrations-disconnect')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('integrations-disconnect'));
    await waitFor(() => {
      expect(mockDisconnect).toHaveBeenCalledWith('gcal');
      expect(mockToast).toHaveBeenCalledWith('Integration getrennt', 'success');
    });
  });

  it('disconnect error surfaces via toast', async () => {
    mockList.mockResolvedValue({ success: true, data: [I_CONN] });
    mockDisconnect.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'fail' } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('integrations-disconnect')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('integrations-disconnect'));
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('fail', 'error'));
  });
});
