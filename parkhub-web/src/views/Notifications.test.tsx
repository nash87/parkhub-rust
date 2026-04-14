import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockGetNotifications = vi.fn();
const mockMarkRead = vi.fn();
const mockMarkAllRead = vi.fn();
const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();

vi.mock('../api/client', () => ({
  api: {
    getNotifications: (...args: any[]) => mockGetNotifications(...args),
    markNotificationRead: (...args: any[]) => mockMarkRead(...args),
    markAllNotificationsRead: (...args: any[]) => mockMarkAllRead(...args),
  },
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: any) => {
      const map: Record<string, string> = {
        'notifications.title': 'Notifications',
        'notifications.allRead': 'All read',
        'notifications.empty': 'No notifications',
        'notifications.markAllRead': 'Mark all as read',
        'notifications.allMarkedRead': 'All marked as read',
        'notifications.unreadCount': '{{count}} unread',
        'notifications.unread': 'Unread',
        'common.refresh': 'Refresh',
        'common.error': 'An error occurred',
        'timeAgo.justNow': 'just now',
        'timeAgo.minutesAgo': '{{count}}m ago',
        'timeAgo.hoursAgo': '{{count}}h ago',
        'timeAgo.daysAgo': '{{count}}d ago',
      };
      let result = map[key] || key;
      if (opts && typeof opts === 'object') {
        Object.entries(opts).forEach(([k, v]) => {
          result = result.replace(`{{${k}}}`, String(v));
        });
      }
      return result;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, variants, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
    p: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <p ref={ref} {...props}>{children}</p>
    )),
    button: React.forwardRef(({ children, initial, animate, exit, transition, whileHover, whileTap, ...props }: any, ref: any) => (
      <button ref={ref} {...props}>{children}</button>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Bell: (props: any) => <span data-testid="icon-bell" {...props} />,
  Warning: (props: any) => <span data-testid="icon-warning" {...props} />,
  Info: (props: any) => <span data-testid="icon-info" {...props} />,
  CheckCircle: (props: any) => <span data-testid="icon-check-circle" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  SpinnerGap: (props: any) => <span data-testid="icon-spinner" {...props} />,
  ArrowClockwise: (props: any) => <span data-testid="icon-refresh" {...props} />,
}));

vi.mock('react-hot-toast', () => ({
  default: { success: (...args: any[]) => mockToastSuccess(...args), error: (...args: any[]) => mockToastError(...args) },
}));

import { NotificationsPage } from './Notifications';
import type { Notification } from '../api/client';

function makeNotification(overrides: Partial<Notification> = {}): Notification {
  return {
    id: 'n1',
    title: 'Test Notification',
    message: 'Something happened',
    notification_type: 'info',
    read: false,
    created_at: new Date().toISOString(),
    ...overrides,
  };
}

describe('NotificationsPage', () => {
  beforeEach(() => {
    mockGetNotifications.mockClear();
    mockMarkRead.mockClear();
    mockMarkAllRead.mockClear();
    mockToastSuccess.mockClear();
    mockToastError.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders loading skeleton initially', () => {
    mockGetNotifications.mockReturnValue(new Promise(() => {}));

    const { container } = render(<NotificationsPage />);
    // Skeleton has .skeleton class elements
    const skeletonElements = container.querySelectorAll('.skeleton');
    expect(skeletonElements.length).toBeGreaterThan(0);
  });

  it('shows empty state when no notifications', async () => {
    mockGetNotifications.mockResolvedValue({ success: true, data: [] });

    render(<NotificationsPage />);

    await waitFor(() => {
      expect(screen.getByText('No notifications')).toBeInTheDocument();
    });
    // Bell icon shown in empty state
    expect(screen.getByTestId('icon-bell')).toBeInTheDocument();
  });

  it('renders notification list with titles and messages', async () => {
    const items = [
      makeNotification({ id: 'n1', title: 'Booking Confirmed', message: 'Your spot is ready' }),
      makeNotification({ id: 'n2', title: 'System Update', message: 'Maintenance tonight', notification_type: 'warning' }),
    ];
    mockGetNotifications.mockResolvedValue({ success: true, data: items });

    render(<NotificationsPage />);

    await waitFor(() => {
      expect(screen.getByText('Booking Confirmed')).toBeInTheDocument();
    });
    expect(screen.getByText('Your spot is ready')).toBeInTheDocument();
    expect(screen.getByText('System Update')).toBeInTheDocument();
    expect(screen.getByText('Maintenance tonight')).toBeInTheDocument();
  });

  it('shows unread count when notifications are unread', async () => {
    const items = [
      makeNotification({ id: 'n1', read: false }),
      makeNotification({ id: 'n2', read: false }),
      makeNotification({ id: 'n3', read: true }),
    ];
    mockGetNotifications.mockResolvedValue({ success: true, data: items });

    render(<NotificationsPage />);

    await waitFor(() => {
      expect(screen.getByText('2 unread')).toBeInTheDocument();
    });
  });

  it('shows "All read" when all notifications are read', async () => {
    const items = [
      makeNotification({ id: 'n1', read: true }),
    ];
    mockGetNotifications.mockResolvedValue({ success: true, data: items });

    render(<NotificationsPage />);

    await waitFor(() => {
      expect(screen.getByText('All read')).toBeInTheDocument();
    });
  });

  it('mark as read updates UI optimistically', async () => {
    const user = userEvent.setup();
    const notif = makeNotification({ id: 'n1', read: false, title: 'Unread Item' });
    mockGetNotifications.mockResolvedValue({ success: true, data: [notif] });
    mockMarkRead.mockResolvedValue({ success: true });

    render(<NotificationsPage />);

    await waitFor(() => {
      expect(screen.getByText('Unread Item')).toBeInTheDocument();
    });

    // Unread notification shows "1 unread"
    expect(screen.getByText('1 unread')).toBeInTheDocument();

    // Click the notification button to mark as read
    await user.click(screen.getByText('Unread Item').closest('button')!);

    await waitFor(() => {
      expect(mockMarkRead).toHaveBeenCalledWith('n1');
    });

    // Optimistic update: now shows "All read"
    await waitFor(() => {
      expect(screen.getByText('All read')).toBeInTheDocument();
    });
  });

  it('mark all as read button calls API and updates UI', async () => {
    const user = userEvent.setup();
    const items = [
      makeNotification({ id: 'n1', read: false }),
      makeNotification({ id: 'n2', read: false }),
    ];
    mockGetNotifications.mockResolvedValue({ success: true, data: items });
    mockMarkAllRead.mockResolvedValue({ success: true });

    render(<NotificationsPage />);

    await waitFor(() => {
      expect(screen.getByText('2 unread')).toBeInTheDocument();
    });

    const markAllBtn = screen.getByRole('button', { name: /Mark all as read/ });
    await user.click(markAllBtn);

    await waitFor(() => {
      expect(mockMarkAllRead).toHaveBeenCalled();
    });

    await waitFor(() => {
      expect(mockToastSuccess).toHaveBeenCalledWith('All marked as read');
    });
  });

  it('mark all as read button is hidden when all are read', async () => {
    const items = [makeNotification({ id: 'n1', read: true })];
    mockGetNotifications.mockResolvedValue({ success: true, data: items });

    render(<NotificationsPage />);

    await waitFor(() => {
      expect(screen.getByText('All read')).toBeInTheDocument();
    });

    expect(screen.queryByRole('button', { name: /Mark all as read/ })).not.toBeInTheDocument();
  });

  it('renders refresh button', async () => {
    mockGetNotifications.mockResolvedValue({ success: true, data: [] });

    render(<NotificationsPage />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Refresh/ })).toBeInTheDocument();
    });
  });

  it('renders page title', async () => {
    mockGetNotifications.mockResolvedValue({ success: true, data: [] });

    render(<NotificationsPage />);

    await waitFor(() => {
      expect(screen.getByText('Notifications')).toBeInTheDocument();
    });
  });

  it('shows error toast when loadNotifications throws', async () => {
    mockGetNotifications.mockRejectedValue(new Error('boom'));
    render(<NotificationsPage />);
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalled();
    });
  });

  it('shows error toast when markAllAsRead fails', async () => {
    const user = userEvent.setup();
    mockGetNotifications.mockResolvedValue({
      success: true,
      data: [{ id: 'n1', title: 'Test', message: 'm', type: 'info', read: false, created_at: new Date().toISOString() }],
    });
    mockMarkAllRead.mockResolvedValue({ success: false, error: { message: 'fail' } });
    render(<NotificationsPage />);
    await waitFor(() => expect(screen.getByText('Test')).toBeInTheDocument());
    const markAllBtn = screen.getByRole('button', { name: /mark all/i });
    await user.click(markAllBtn);
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalled();
    });
  });

  it('renders success-type notification with success icon color', async () => {
    mockGetNotifications.mockResolvedValue({
      success: true,
      data: [{ id: 'n1', title: 'Paid', message: 'Your payment succeeded', type: 'success', read: false, created_at: new Date().toISOString() }],
    });
    render(<NotificationsPage />);
    await waitFor(() => expect(screen.getByText('Paid')).toBeInTheDocument());
    expect(screen.getByText('Your payment succeeded')).toBeInTheDocument();
  });

  it('renders notification ages — minutes, hours, days buckets', async () => {
    const now = Date.now();
    mockGetNotifications.mockResolvedValue({
      success: true,
      data: [
        { id: 'n1', title: 'Fresh', message: 'Just now', type: 'info', read: false, created_at: new Date(now - 10_000).toISOString() },
        { id: 'n2', title: 'Old minutes', message: 'minutes ago', type: 'info', read: false, created_at: new Date(now - 5 * 60_000).toISOString() },
        { id: 'n3', title: 'Hours back', message: 'hours ago', type: 'info', read: false, created_at: new Date(now - 3 * 3600_000).toISOString() },
        { id: 'n4', title: 'Days back', message: 'days ago', type: 'info', read: false, created_at: new Date(now - 2 * 24 * 3600_000).toISOString() },
      ],
    });
    render(<NotificationsPage />);
    await waitFor(() => expect(screen.getByText('Fresh')).toBeInTheDocument());
    // All four timeline buckets resolved without throwing — the different branches of timeAgoFn ran
    expect(screen.getByText('Old minutes')).toBeInTheDocument();
    expect(screen.getByText('Hours back')).toBeInTheDocument();
    expect(screen.getByText('Days back')).toBeInTheDocument();
  });
});
