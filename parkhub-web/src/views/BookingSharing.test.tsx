import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => {
      const map: Record<string, string> = {
        'sharing.title': 'Share Booking',
        'sharing.help': 'Share your booking details with others via a secure link',
        'sharing.helpLabel': 'Help',
        'sharing.tabLink': 'Share Link',
        'sharing.tabInvite': 'Invite Guest',
        'sharing.expiryLabel': 'Link expires in',
        'sharing.expiry24h': '24 hours',
        'sharing.expiry3d': '3 days',
        'sharing.expiry7d': '7 days',
        'sharing.expiry30d': '30 days',
        'sharing.expiryNever': 'Never',
        'sharing.createLink': 'Create Share Link',
        'sharing.creating': 'Creating...',
        'sharing.linkCreated': 'Share link created!',
        'sharing.copied': 'Link copied!',
        'sharing.revokeLink': 'Revoke Link',
        'sharing.linkRevoked': 'Link revoked',
        'sharing.guestEmail': 'Guest Email',
        'sharing.emailPlaceholder': 'guest@example.com',
        'sharing.messageLabel': 'Message (optional)',
        'sharing.messagePlaceholder': 'Join me at the parking lot!',
        'sharing.sendInvite': 'Send Invite',
        'sharing.sending': 'Sending...',
        'sharing.inviteSent': `Invite sent to ${opts?.email || ''}`,
        'sharing.invalidEmail': 'Invalid email address',
        'sharing.expiresAt': `Expires ${opts?.date || ''}`,
        'common.error': 'Error',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  ShareNetwork: (props: any) => <span data-testid="icon-share" {...props} />,
  Link: (props: any) => <span data-testid="icon-link" {...props} />,
  Copy: (props: any) => <span data-testid="icon-copy" {...props} />,
  Envelope: (props: any) => <span data-testid="icon-envelope" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  Trash: (props: any) => <span data-testid="icon-trash" {...props} />,
  CheckCircle: (props: any) => <span data-testid="icon-check" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: { success: vi.fn(), error: vi.fn() },
}));

import { BookingSharingModal } from './BookingSharing';

const sampleShareLink = {
  id: 'share-1',
  booking_id: 'booking-1',
  code: 'abc123def456',
  url: '/shared/abc123def456',
  status: 'active' as const,
  message: null,
  created_at: '2026-03-23T10:00:00Z',
  expires_at: '2026-03-30T10:00:00Z',
  view_count: 0,
};

describe('BookingSharingModal', () => {
  beforeEach(() => {
    global.fetch = vi.fn(() =>
      Promise.resolve({ json: () => Promise.resolve({ success: true, data: sampleShareLink }) } as Response)
    ) as any;
    // Mock clipboard
    Object.assign(navigator, {
      clipboard: { writeText: vi.fn(() => Promise.resolve()) },
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the modal with title', () => {
    render(<BookingSharingModal bookingId="booking-1" />);
    expect(screen.getByText('Share Booking')).toBeInTheDocument();
  });

  it('renders both tabs', () => {
    render(<BookingSharingModal bookingId="booking-1" />);
    expect(screen.getByTestId('tab-link')).toBeInTheDocument();
    expect(screen.getByTestId('tab-invite')).toBeInTheDocument();
  });

  it('shows help text when help button clicked', () => {
    render(<BookingSharingModal bookingId="booking-1" />);
    fireEvent.click(screen.getByTestId('sharing-help-btn'));
    expect(screen.getByTestId('sharing-help')).toBeInTheDocument();
    expect(screen.getByText('Share your booking details with others via a secure link')).toBeInTheDocument();
  });

  it('renders link panel by default with create button', () => {
    render(<BookingSharingModal bookingId="booking-1" />);
    expect(screen.getByTestId('link-panel')).toBeInTheDocument();
    expect(screen.getByTestId('create-link-btn')).toBeInTheDocument();
  });

  it('renders expiry selector', () => {
    render(<BookingSharingModal bookingId="booking-1" />);
    expect(screen.getByTestId('expiry-select')).toBeInTheDocument();
    expect(screen.getByText('7 days')).toBeInTheDocument();
  });

  it('switches to invite tab', () => {
    render(<BookingSharingModal bookingId="booking-1" />);
    fireEvent.click(screen.getByTestId('tab-invite'));
    expect(screen.getByTestId('invite-panel')).toBeInTheDocument();
    expect(screen.getByTestId('invite-email-input')).toBeInTheDocument();
    expect(screen.getByTestId('invite-message-input')).toBeInTheDocument();
    expect(screen.getByTestId('send-invite-btn')).toBeInTheDocument();
  });

  it('creates share link on button click', async () => {
    render(<BookingSharingModal bookingId="booking-1" />);
    fireEvent.click(screen.getByTestId('create-link-btn'));
    await waitFor(() => {
      expect(global.fetch).toHaveBeenCalledWith(
        '/api/v1/bookings/booking-1/share',
        expect.objectContaining({ method: 'POST' })
      );
    });
  });

  it('calls onClose when close button clicked', () => {
    const onClose = vi.fn();
    render(<BookingSharingModal bookingId="booking-1" onClose={onClose} />);
    fireEvent.click(screen.getByTestId('sharing-close-btn'));
    expect(onClose).toHaveBeenCalled();
  });
});
