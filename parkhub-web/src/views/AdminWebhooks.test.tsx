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
});
