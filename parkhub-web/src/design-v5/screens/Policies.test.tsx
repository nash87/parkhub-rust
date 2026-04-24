import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockList = vi.fn();
const mockUpdate = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    getPolicies: (...a: unknown[]) => mockList(...a),
    updatePolicy: (...a: unknown[]) => mockUpdate(...a),
  },
}));

const mockToast = vi.fn();
vi.mock('../Toast', () => ({
  useV5Toast: () => mockToast,
  V5ToastProvider: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

import { PoliciesV5 } from './Policies';

const P1 = { id: 'p1', title: 'AGB', slug: 'agb', body: 'Alter Text', updated_at: '2026-04-01T10:00:00Z' };
const P2 = { id: 'p2', title: 'Datenschutz', slug: 'dsg', body: 'DSGVO', updated_at: '2026-04-02T10:00:00Z' };

function renderScreen() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <PoliciesV5 navigate={vi.fn()} />
    </QueryClientProvider>
  );
}

describe('PoliciesV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders empty state', async () => {
    mockList.mockResolvedValue({ success: true, data: [] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Keine Richtlinien')).toBeInTheDocument());
  });

  it('renders error state', async () => {
    mockList.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'fail' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('selects first policy into editor', async () => {
    mockList.mockResolvedValue({ success: true, data: [P1, P2] });
    renderScreen();
    await waitFor(() => expect((screen.getByTestId('policies-editor') as HTMLTextAreaElement).value).toBe('Alter Text'));
  });

  it('clicking another policy switches editor content', async () => {
    mockList.mockResolvedValue({ success: true, data: [P1, P2] });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Datenschutz')).toBeInTheDocument());
    fireEvent.click(screen.getByText('Datenschutz'));
    await waitFor(() => expect((screen.getByTestId('policies-editor') as HTMLTextAreaElement).value).toBe('DSGVO'));
  });

  it('save mutation fires with updated body', async () => {
    mockList.mockResolvedValue({ success: true, data: [P1] });
    mockUpdate.mockResolvedValue({ success: true, data: { ...P1, body: 'Neu' } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('policies-editor')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('policies-editor'), { target: { value: 'Neu' } });
    fireEvent.click(screen.getByTestId('policies-save'));
    await waitFor(() => {
      expect(mockUpdate).toHaveBeenCalledWith('p1', 'Neu');
      expect(mockToast).toHaveBeenCalledWith('Richtlinie gespeichert', 'success');
    });
  });

  it('save error surfaces via toast', async () => {
    mockList.mockResolvedValue({ success: true, data: [P1] });
    mockUpdate.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('policies-editor')).toBeInTheDocument());
    fireEvent.change(screen.getByTestId('policies-editor'), { target: { value: 'Neu' } });
    fireEvent.click(screen.getByTestId('policies-save'));
    await waitFor(() => expect(mockToast).toHaveBeenCalledWith('denied', 'error'));
  });

  it('preview toggle flips editor ↔ preview', async () => {
    mockList.mockResolvedValue({ success: true, data: [P1] });
    renderScreen();
    await waitFor(() => expect(screen.getByTestId('policies-editor')).toBeInTheDocument());
    fireEvent.click(screen.getByTestId('policies-preview-toggle'));
    await waitFor(() => expect(screen.getByTestId('policies-preview')).toBeInTheDocument());
    expect(screen.queryByTestId('policies-editor')).toBeNull();
  });
});
