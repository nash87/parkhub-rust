import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (k: string) => k }) }));
vi.mock('framer-motion', () => ({
  motion: { div: React.forwardRef(({ children, ...p }: any, r: any) => <div ref={r} {...p}>{children}</div>) },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));
vi.mock('@phosphor-icons/react', () => {
  const C = (p: any) => <span {...p} />;
  return { WebhooksLogo: C, Plus: C, Trash: C, Pencil: C, Question: C, PaperPlaneTilt: C, ListChecks: C };
});
vi.mock('react-hot-toast', () => ({ default: { success: vi.fn(), error: vi.fn() } }));

import { AdminWebhooksPage } from './AdminWebhooks';
import toast from 'react-hot-toast';

const webhooks = [
  { id: 'wh1', url: 'https://a.com/hook', secret: 'sec', events: ['booking.created'], active: true, description: 'Main hook', created_at: '2026-01-01', updated_at: '2026-04-01' },
  { id: 'wh2', url: 'https://b.com/hook', secret: 'sec2', events: ['user.registered', 'lot.full'], active: true, description: null, created_at: '2026-02-01', updated_at: '2026-04-01' },
];
const deliveries = [
  { id: 'd1', event_type: 'booking.created', status_code: 200, success: true, attempt: 1, error: null, delivered_at: '2026-04-10T08:00:00Z' },
  { id: 'd2', event_type: 'booking.created', status_code: null, success: false, attempt: 2, error: 'timeout', delivered_at: '2026-04-10T08:01:00Z' },
];

describe('AdminWebhooksPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (url.includes('/deliveries')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: deliveries }) } as Response);
      if (url.includes('/test')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { success: true } }) } as Response);
      if (opts?.method === 'DELETE') return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      if (opts?.method === 'POST' || opts?.method === 'PUT') return Promise.resolve({ json: () => Promise.resolve({ success: true }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: webhooks }) } as Response);
    }) as any;
  });
  afterEach(() => vi.restoreAllMocks());

  it('renders webhooks list', async () => {
    render(<AdminWebhooksPage />);
    await waitFor(() => expect(screen.getByText('https://a.com/hook')).toBeInTheDocument());
  });

  it('shows description when present', async () => {
    render(<AdminWebhooksPage />);
    await waitFor(() => expect(screen.getByText('Main hook')).toBeInTheDocument());
  });

  it('shows help', async () => {
    render(<AdminWebhooksPage />);
    await waitFor(() => {
      const helpBtn = screen.getByLabelText('webhooksV2.helpLabel');
      fireEvent.click(helpBtn);
    });
    await waitFor(() => expect(screen.getByText('webhooksV2.help')).toBeInTheDocument());
  });

  it('opens create form', async () => {
    render(<AdminWebhooksPage />);
    await waitFor(() => fireEvent.click(screen.getByText('webhooksV2.create')));
    expect(screen.getByText('webhooksV2.newWebhook')).toBeInTheDocument();
  });

  it('validates empty fields', async () => {
    render(<AdminWebhooksPage />);
    await waitFor(() => fireEvent.click(screen.getByText('webhooksV2.create')));
    fireEvent.click(screen.getByText('webhooksV2.save'));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('webhooksV2.requiredFields'));
  });

  it('creates webhook', async () => {
    render(<AdminWebhooksPage />);
    await waitFor(() => fireEvent.click(screen.getByText('webhooksV2.create')));
    const urlInput = screen.getAllByRole('textbox')[0];
    fireEvent.change(urlInput, { target: { value: 'https://new.com' } });
    fireEvent.click(screen.getByText('booking.created')); // toggle event
    fireEvent.click(screen.getByText('webhooksV2.save'));
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('webhooksV2.created'));
  });

  it('edits webhook', async () => {
    render(<AdminWebhooksPage />);
    await waitFor(() => {
      const editBtns = screen.getAllByLabelText('webhooksV2.edit');
      fireEvent.click(editBtns[0]);
    });
    expect(screen.getByText('webhooksV2.editWebhook')).toBeInTheDocument();
  });

  it('deletes webhook', async () => {
    render(<AdminWebhooksPage />);
    await waitFor(() => {
      const delBtns = screen.getAllByLabelText('webhooksV2.delete');
      fireEvent.click(delBtns[0]);
    });
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('webhooksV2.deleted'));
  });

  it('tests webhook', async () => {
    render(<AdminWebhooksPage />);
    await waitFor(() => {
      const testBtns = screen.getAllByLabelText('webhooksV2.test');
      fireEvent.click(testBtns[0]);
    });
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('webhooksV2.testSuccess'));
  });

  it('test webhook failure response', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (url.includes('/test')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: { success: false } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: webhooks }) } as Response);
    }) as any;
    render(<AdminWebhooksPage />);
    await waitFor(() => fireEvent.click(screen.getAllByLabelText('webhooksV2.test')[0]));
    await waitFor(() => expect(toast.success).toHaveBeenCalledWith('webhooksV2.testFailed'));
  });

  it('loads deliveries', async () => {
    render(<AdminWebhooksPage />);
    await waitFor(() => {
      const delivBtns = screen.getAllByLabelText('webhooksV2.deliveries');
      fireEvent.click(delivBtns[0]);
    });
    await waitFor(() => expect(screen.getByText('webhooksV2.deliveryLog')).toBeInTheDocument());
  });

  it('closes delivery log', async () => {
    render(<AdminWebhooksPage />);
    await waitFor(() => fireEvent.click(screen.getAllByLabelText('webhooksV2.deliveries')[0]));
    await waitFor(() => expect(screen.getByText('webhooksV2.deliveryLog')).toBeInTheDocument());
    fireEvent.click(screen.getByText('common.close'));
    await waitFor(() => expect(screen.queryByText('webhooksV2.deliveryLog')).not.toBeInTheDocument());
  });

  it('shows empty state', async () => {
    globalThis.fetch = vi.fn(() => Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response)) as any;
    render(<AdminWebhooksPage />);
    await waitFor(() => expect(screen.getByText('webhooksV2.empty')).toBeInTheDocument());
  });

  it('empty deliveries', async () => {
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/deliveries')) return Promise.resolve({ json: () => Promise.resolve({ success: true, data: [] }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: webhooks }) } as Response);
    }) as any;
    render(<AdminWebhooksPage />);
    await waitFor(() => fireEvent.click(screen.getAllByLabelText('webhooksV2.deliveries')[0]));
    await waitFor(() => expect(screen.getByText('webhooksV2.noDeliveries')).toBeInTheDocument());
  });

  it('toggle event on/off', async () => {
    render(<AdminWebhooksPage />);
    await waitFor(() => fireEvent.click(screen.getByText('webhooksV2.create')));
    // In the form, event buttons are in a flex-wrap container
    const evBtns = screen.getAllByText('booking.created');
    // The last one is in the form (others are in the webhook list)
    const formBtn = evBtns[evBtns.length - 1];
    fireEvent.click(formBtn); // toggle on
    fireEvent.click(formBtn); // toggle off
  });

  it('cancel form', async () => {
    render(<AdminWebhooksPage />);
    await waitFor(() => fireEvent.click(screen.getByText('webhooksV2.create')));
    fireEvent.click(screen.getByText('common.cancel'));
    await waitFor(() => expect(screen.queryByText('webhooksV2.newWebhook')).not.toBeInTheDocument());
  });

  it('handles save error response', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST' && !url.includes('/test')) return Promise.resolve({ json: () => Promise.resolve({ success: false, error: { message: 'Invalid URL' } }) } as Response);
      return Promise.resolve({ json: () => Promise.resolve({ success: true, data: webhooks }) } as Response);
    }) as any;
    render(<AdminWebhooksPage />);
    await waitFor(() => fireEvent.click(screen.getByText('webhooksV2.create')));
    fireEvent.change(screen.getAllByRole('textbox')[0], { target: { value: 'bad' } });
    fireEvent.click(screen.getByText('booking.created'));
    fireEvent.click(screen.getByText('webhooksV2.save'));
    await waitFor(() => expect(toast.error).toHaveBeenCalledWith('Invalid URL'));
  });

  it('handles network errors', async () => {
    globalThis.fetch = vi.fn(() => Promise.reject(new Error('net'))) as any;
    render(<AdminWebhooksPage />);
    await waitFor(() => expect(toast.error).toHaveBeenCalled());
  });
});
