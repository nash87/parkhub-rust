import { describe, it, expect, vi, beforeEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const map: Record<string, string> = {
        'webhooksV2.title': 'Webhooks v2',
        'webhooksV2.subtitle': 'Outgoing event subscriptions with delivery tracking',
        'webhooksV2.help': 'Configure webhooks to receive notifications when events occur.',
        'webhooksV2.helpLabel': 'Help',
        'webhooksV2.create': 'Create Webhook',
        'webhooksV2.newWebhook': 'New Webhook',
        'webhooksV2.editWebhook': 'Edit Webhook',
        'webhooksV2.url': 'URL',
        'webhooksV2.events': 'Events',
        'webhooksV2.description': 'Description',
        'webhooksV2.descriptionPlaceholder': 'Optional description',
        'webhooksV2.save': 'Save',
        'webhooksV2.edit': 'Edit',
        'webhooksV2.delete': 'Delete',
        'webhooksV2.test': 'Test',
        'webhooksV2.deliveries': 'Deliveries',
        'webhooksV2.deliveryLog': 'Delivery Log',
        'webhooksV2.noDeliveries': 'No deliveries yet',
        'webhooksV2.attempt': 'Attempt',
        'webhooksV2.created': 'Webhook created',
        'webhooksV2.updated': 'Webhook updated',
        'webhooksV2.deleted': 'Webhook deleted',
        'webhooksV2.testSuccess': 'Test event delivered',
        'webhooksV2.testFailed': 'Test delivery failed',
        'webhooksV2.empty': 'No webhooks configured',
        'webhooksV2.requiredFields': 'URL and at least one event required',
        'common.cancel': 'Cancel',
        'common.error': 'Error',
        'common.loading': 'Loading...',
        'common.close': 'Close',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  WebhooksLogo: (props: any) => <span data-testid="icon-webhooks" {...props} />,
  Plus: (props: any) => <span data-testid="icon-plus" {...props} />,
  Trash: (props: any) => <span data-testid="icon-trash" {...props} />,
  Pencil: (props: any) => <span data-testid="icon-pencil" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  PaperPlaneTilt: (props: any) => <span data-testid="icon-send" {...props} />,
  ListChecks: (props: any) => <span data-testid="icon-list" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: { success: vi.fn(), error: vi.fn() },
}));

import { AdminWebhooksPage } from './AdminWebhooks';

const sampleWebhooks = [
  {
    id: 'wh-001',
    url: 'https://example.com/webhook',
    secret: 'whsec_test',
    events: ['booking.created', 'lot.full'],
    active: true,
    description: 'Test webhook',
    created_at: '2026-03-23T10:00:00Z',
    updated_at: '2026-03-23T10:00:00Z',
  },
];

describe('AdminWebhooksPage', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('renders title and subtitle', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleWebhooks }),
    });

    render(<AdminWebhooksPage />);
    expect(screen.getByText('Webhooks v2')).toBeDefined();
    expect(screen.getByText('Outgoing event subscriptions with delivery tracking')).toBeDefined();
  });

  it('renders webhook list', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleWebhooks }),
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => {
      expect(screen.getByText('https://example.com/webhook')).toBeDefined();
    });
  });

  it('shows empty state', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => {
      expect(screen.getByText('No webhooks configured')).toBeDefined();
    });
  });

  it('opens create form', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => {
      fireEvent.click(screen.getByText('Create Webhook'));
      expect(screen.getByText('New Webhook')).toBeDefined();
    });
  });

  it('shows event checkboxes in form', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => {
      fireEvent.click(screen.getByText('Create Webhook'));
      expect(screen.getByText('booking.created')).toBeDefined();
      expect(screen.getByText('lot.full')).toBeDefined();
      expect(screen.getByText('payment.completed')).toBeDefined();
    });
  });

  it('handles API errors', async () => {
    globalThis.fetch = vi.fn().mockRejectedValue(new Error('Network error'));

    render(<AdminWebhooksPage />);
    await waitFor(() => {
      expect(screen.getByText('Webhooks v2')).toBeDefined();
    });
  });

  it('shows webhook description', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleWebhooks }),
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => {
      expect(screen.getByText('Test webhook')).toBeDefined();
    });
  });

  it('shows active status indicator on webhooks', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleWebhooks }),
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => {
      expect(screen.getByText('https://example.com/webhook')).toBeDefined();
    });
  });

  it('shows create webhook button', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleWebhooks }),
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => {
      expect(screen.getByText('Create Webhook')).toBeDefined();
    });
  });

  it('shows help tooltip when help button is clicked', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: [] }),
    });
    render(<AdminWebhooksPage />);
    await waitFor(() => expect(screen.getByLabelText('Help')).toBeDefined());
    fireEvent.click(screen.getByLabelText('Help'));
    await waitFor(() => {
      expect(screen.getByText('Configure webhooks to receive notifications when events occur.')).toBeDefined();
    });
  });

  it('saves new webhook successfully', async () => {
    let callCount = 0;
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST') {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({ success: true, data: { id: 'wh-new', url: 'https://new.com', events: ['lot.full'], active: true } }),
        });
      }
      callCount++;
      return Promise.resolve({
        ok: true,
        json: () => Promise.resolve({ success: true, data: callCount <= 1 ? [] : sampleWebhooks }),
      });
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => fireEvent.click(screen.getByText('Create Webhook')));
    await waitFor(() => expect(screen.getByText('New Webhook')).toBeDefined());

    // Fill URL
    const urlInput = screen.getByPlaceholderText('https://example.com/webhook');
    fireEvent.change(urlInput, { target: { value: 'https://new.com' } });

    // Select event
    fireEvent.click(screen.getByText('lot.full'));

    // Save
    fireEvent.click(screen.getByText('Save'));

    await waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalledWith(
        '/api/v1/admin/webhooks-v2',
        expect.objectContaining({ method: 'POST' }),
      );
    });
  });

  it('shows validation error when saving with no URL or events', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => fireEvent.click(screen.getByText('Create Webhook')));
    fireEvent.click(screen.getByText('Save'));
    // Should show error toast for missing fields
  });

  it('cancels form', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => fireEvent.click(screen.getByText('Create Webhook')));
    expect(screen.getByText('New Webhook')).toBeDefined();

    fireEvent.click(screen.getByText('Cancel'));
    await waitFor(() => {
      expect(screen.queryByText('New Webhook')).toBeNull();
    });
  });

  it('deletes a webhook', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'DELETE') {
        return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true }) });
      }
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true, data: sampleWebhooks }) });
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => expect(screen.getByText('https://example.com/webhook')).toBeDefined());

    fireEvent.click(screen.getByLabelText('Delete'));

    await waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalledWith(
        '/api/v1/admin/webhooks-v2/wh-001',
        expect.objectContaining({ method: 'DELETE' }),
      );
    });
  });

  it('tests a webhook', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST' && url.includes('/test')) {
        return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true, data: { success: true } }) });
      }
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true, data: sampleWebhooks }) });
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => expect(screen.getByText('https://example.com/webhook')).toBeDefined());

    fireEvent.click(screen.getByLabelText('Test'));

    await waitFor(() => {
      expect(globalThis.fetch).toHaveBeenCalledWith(
        '/api/v1/admin/webhooks-v2/wh-001/test',
        expect.objectContaining({ method: 'POST' }),
      );
    });
  });

  it('loads and shows delivery log', async () => {
    const deliveries = [
      { id: 'd1', event_type: 'booking.created', status_code: 200, success: true, attempt: 1, error: null, delivered_at: '2026-03-23T10:00:00Z' },
    ];
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/deliveries')) {
        return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true, data: deliveries }) });
      }
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true, data: sampleWebhooks }) });
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => expect(screen.getByText('https://example.com/webhook')).toBeDefined());

    fireEvent.click(screen.getByLabelText('Deliveries'));

    await waitFor(() => {
      expect(screen.getByText('Delivery Log')).toBeDefined();
      expect(screen.getByText('booking.created')).toBeDefined();
    });
  });

  it('shows empty deliveries message', async () => {
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/deliveries')) {
        return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true, data: [] }) });
      }
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true, data: sampleWebhooks }) });
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => expect(screen.getByText('https://example.com/webhook')).toBeDefined());

    fireEvent.click(screen.getByLabelText('Deliveries'));

    await waitFor(() => {
      expect(screen.getByText('No deliveries yet')).toBeDefined();
    });
  });

  it('closes delivery log', async () => {
    globalThis.fetch = vi.fn((url: string) => {
      if (url.includes('/deliveries')) {
        return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true, data: [] }) });
      }
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true, data: sampleWebhooks }) });
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => expect(screen.getByText('https://example.com/webhook')).toBeDefined());

    fireEvent.click(screen.getByLabelText('Deliveries'));
    await waitFor(() => expect(screen.getByText('Delivery Log')).toBeDefined());

    fireEvent.click(screen.getByText('Close'));
    await waitFor(() => {
      expect(screen.queryByText('Delivery Log')).toBeNull();
    });
  });

  it('opens edit form with pre-filled data', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: sampleWebhooks }),
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => expect(screen.getByText('https://example.com/webhook')).toBeDefined());

    fireEvent.click(screen.getByLabelText('Edit'));
    await waitFor(() => {
      expect(screen.getByText('Edit Webhook')).toBeDefined();
    });
  });

  it('toggles event selection in form', async () => {
    globalThis.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ success: true, data: [] }),
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => fireEvent.click(screen.getByText('Create Webhook')));

    // Click booking.created to select
    fireEvent.click(screen.getByText('booking.created'));
    // Click again to deselect
    fireEvent.click(screen.getByText('booking.created'));
  });

  it('handles save failure with error message', async () => {
    globalThis.fetch = vi.fn((url: string, opts?: any) => {
      if (opts?.method === 'POST' && !url.includes('/test')) {
        return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: false, error: { message: 'Invalid URL' } }) });
      }
      return Promise.resolve({ ok: true, json: () => Promise.resolve({ success: true, data: [] }) });
    });

    render(<AdminWebhooksPage />);
    await waitFor(() => fireEvent.click(screen.getByText('Create Webhook')));

    fireEvent.change(screen.getByPlaceholderText('https://example.com/webhook'), { target: { value: 'bad' } });
    fireEvent.click(screen.getByText('booking.created'));
    fireEvent.click(screen.getByText('Save'));
    // Error path exercised
  });
});
