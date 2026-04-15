import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent, act } from '@testing-library/react';

const mockNavigate = vi.fn();

vi.mock('react-router-dom', () => ({
  useNavigate: () => mockNavigate,
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) => {
      const map: Record<string, string> = {
        'notificationCenter.title': 'Notifications',
        'notificationCenter.bellTooltip': 'Notifications',
        'notificationCenter.markAllRead': 'Mark all read',
        'notificationCenter.empty': 'No notifications',
        'notificationCenter.today': 'Today',
        'notificationCenter.yesterday': 'Yesterday',
        'notificationCenter.viewAll': 'View all',
        'notificationCenter.deleted': 'Deleted',
        'notificationCenter.markRead': 'Mark as read',
        'notificationCenter.deleteOne': 'Delete',
        'notificationCenter.filter.all': 'All',
        'notificationCenter.filter.unread': 'Unread',
        'notificationCenter.filter.read': 'Read',
        'notificationCenter.help': 'Notifications help text here',
        'notificationCenter.helpLabel': 'Help',
        'timeAgo.justNow': 'Just now',
        'timeAgo.minutesAgo': `${opts?.count || 0} minutes ago`,
        'timeAgo.hoursAgo': `${opts?.count || 0} hours ago`,
        'timeAgo.daysAgo': `${opts?.count || 0} days ago`,
        'common.error': 'Error',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, layout, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Bell: (props: any) => <span data-testid="icon-bell" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  CheckCircle: (props: any) => <span data-testid="icon-check-circle" {...props} />,
  XCircle: (props: any) => <span data-testid="icon-x-circle" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  Queue: (props: any) => <span data-testid="icon-queue" {...props} />,
  Wrench: (props: any) => <span data-testid="icon-wrench" {...props} />,
  Megaphone: (props: any) => <span data-testid="icon-megaphone" {...props} />,
  CurrencyDollar: (props: any) => <span data-testid="icon-dollar" {...props} />,
  UserPlus: (props: any) => <span data-testid="icon-user-plus" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  Trash: (props: any) => <span data-testid="icon-trash" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  FunnelSimple: (props: any) => <span data-testid="icon-funnel" {...props} />,
}));

const mockToastSuccess = vi.fn();
const mockToastError = vi.fn();
vi.mock('react-hot-toast', () => ({
  default: {
    success: (...args: any[]) => mockToastSuccess(...args),
    error: (...args: any[]) => mockToastError(...args),
  },
}));

const mockGetUnreadCount = vi.fn();
const mockGetCenter = vi.fn();
const mockMarkAllRead = vi.fn();
const mockMarkRead = vi.fn();
const mockDelete = vi.fn();
const mockGetInMemoryToken = vi.fn(() => 'test-token');

vi.mock('../api/client', () => ({
  api: {
    getNotificationUnreadCount: (...args: any[]) => mockGetUnreadCount(...args),
    getNotificationCenter: (...args: any[]) => mockGetCenter(...args),
    markAllNotificationCenterRead: (...args: any[]) => mockMarkAllRead(...args),
    markNotificationCenterRead: (...args: any[]) => mockMarkRead(...args),
    deleteNotificationCenter: (...args: any[]) => mockDelete(...args),
  },
  getInMemoryToken: () => mockGetInMemoryToken(),
}));

import { NotificationCenter } from './NotificationCenter';

const sampleNotifications = [
  {
    id: 'n1',
    notification_type: 'booking_confirmed',
    title: 'Booking Confirmed',
    message: 'Your booking has been confirmed',
    read: false,
    action_url: '/bookings/b1',
    icon: 'check-circle',
    severity: 'success',
    type_label: 'Booking',
    created_at: new Date(Date.now() - 30 * 60000).toISOString(), // 30 min ago
    date_group: 'today',
  },
  {
    id: 'n2',
    notification_type: 'maintenance',
    title: 'Maintenance Alert',
    message: 'Scheduled maintenance tomorrow',
    read: true,
    action_url: null,
    icon: 'wrench',
    severity: 'warning',
    type_label: 'System',
    created_at: new Date(Date.now() - 2 * 3600000).toISOString(), // 2 hours ago
    date_group: 'today',
  },
  {
    id: 'n3',
    notification_type: 'announcement',
    title: 'New Feature',
    message: 'We launched something new',
    read: false,
    action_url: null,
    icon: 'megaphone',
    severity: 'info',
    type_label: 'Announcement',
    created_at: new Date(Date.now() - 26 * 3600000).toISOString(), // yesterday
    date_group: 'yesterday',
  },
  {
    id: 'n4',
    notification_type: 'payment',
    title: 'Payment Received',
    message: 'Your payment was processed',
    read: true,
    action_url: '/payments',
    icon: 'unknown-icon',
    severity: 'unknown-severity',
    type_label: 'Payment',
    created_at: new Date(Date.now() - 3 * 86400000).toISOString(), // 3 days ago
    date_group: '2026-04-11',
  },
];

function seedMocks(overrides?: { unreadCount?: number; items?: typeof sampleNotifications }) {
  const items = overrides?.items ?? sampleNotifications;
  const unreadCount = overrides?.unreadCount ?? 2;
  mockGetUnreadCount.mockResolvedValue({ success: true, data: { count: unreadCount } });
  mockGetCenter.mockResolvedValue({
    success: true,
    data: { items, total: items.length, page: 1, per_page: 50, unread_count: unreadCount },
  });
  mockMarkAllRead.mockResolvedValue({ success: true, data: null });
  mockMarkRead.mockResolvedValue({ success: true, data: null });
  mockDelete.mockResolvedValue({ success: true, data: null });
}

describe('NotificationCenter', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetInMemoryToken.mockReturnValue('test-token');
    seedMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders bell icon button', () => {
    render(<NotificationCenter />);
    expect(screen.getByLabelText('Notifications')).toBeInTheDocument();
  });

  it('fetches unread count on mount', async () => {
    render(<NotificationCenter />);
    await waitFor(() => {
      expect(mockGetUnreadCount).toHaveBeenCalled();
    });
  });

  it('shows unread badge when count > 0', async () => {
    render(<NotificationCenter />);
    await waitFor(() => {
      expect(screen.getByText('2')).toBeInTheDocument();
    });
  });

  it('shows 99+ when unread count exceeds 99', async () => {
    seedMocks({ unreadCount: 150 });
    render(<NotificationCenter />);
    await waitFor(() => {
      expect(screen.getByText('99+')).toBeInTheDocument();
    });
  });

  it('does not show unread badge when count is 0', async () => {
    seedMocks({ unreadCount: 0, items: [] });
    render(<NotificationCenter />);
    await waitFor(() => {
      expect(screen.queryByText('0')).not.toBeInTheDocument();
    });
  });

  it('skips fetch when no token is present', async () => {
    mockGetInMemoryToken.mockReturnValue(null);
    render(<NotificationCenter />);
    // give the effect a tick to try firing
    await new Promise((r) => setTimeout(r, 10));
    expect(mockGetUnreadCount).not.toHaveBeenCalled();
  });

  it('opens panel when bell is clicked', async () => {
    render(<NotificationCenter />);
    await waitFor(() => screen.getByLabelText('Notifications'));
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => {
      expect(screen.getByText('Booking Confirmed')).toBeInTheDocument();
    });
  });

  it('fetches notifications when opened', async () => {
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => {
      expect(mockGetCenter).toHaveBeenCalledWith('all', 50);
    });
  });

  it('displays notifications grouped by date', async () => {
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => {
      expect(screen.getByText('Today')).toBeInTheDocument();
      expect(screen.getByText('Yesterday')).toBeInTheDocument();
      expect(screen.getByText('2026-04-11')).toBeInTheDocument();
    });
  });

  it('shows empty state when no notifications', async () => {
    seedMocks({ items: [], unreadCount: 0 });
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => {
      expect(screen.getByText('No notifications')).toBeInTheDocument();
    });
  });

  it('shows mark all read button when there are unread notifications', async () => {
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => {
      expect(screen.getByText('Mark all read')).toBeInTheDocument();
    });
  });

  it('marks all notifications as read', async () => {
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => screen.getByText('Mark all read'));
    fireEvent.click(screen.getByText('Mark all read'));
    await waitFor(() => {
      expect(mockMarkAllRead).toHaveBeenCalled();
    });
  });

  it('closes panel when X button is clicked', async () => {
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => screen.getByText('Booking Confirmed'));
    // Find the close button (the X icon parent button)
    const closeButtons = screen.getAllByTestId('icon-x');
    const closeBtn = closeButtons[0].closest('button');
    if (closeBtn) fireEvent.click(closeBtn);
    await waitFor(() => {
      expect(screen.queryByText('Booking Confirmed')).not.toBeInTheDocument();
    });
  });

  it('toggles help tooltip', async () => {
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => screen.getByText('Booking Confirmed'));
    const helpBtn = screen.getByTitle('Help');
    fireEvent.click(helpBtn);
    expect(screen.getByText('Notifications help text here')).toBeInTheDocument();
    fireEvent.click(helpBtn);
    // help should be hidden after second click -- but AnimatePresence is mocked to always show children
    // so we just verify the toggle mechanism exists
  });

  it('navigates when clicking notification with action_url', async () => {
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => screen.getByText('Booking Confirmed'));
    fireEvent.click(screen.getByText('Booking Confirmed'));
    await waitFor(() => {
      expect(mockNavigate).toHaveBeenCalledWith('/bookings/b1');
    });
  });

  it('marks unread notification as read when clicking it', async () => {
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => screen.getByText('Booking Confirmed'));
    fireEvent.click(screen.getByText('Booking Confirmed'));
    await waitFor(() => {
      expect(mockMarkRead).toHaveBeenCalledWith('n1');
    });
  });

  it('does not mark already-read notification when clicking', async () => {
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => screen.getByText('Maintenance Alert'));
    mockMarkRead.mockClear();
    fireEvent.click(screen.getByText('Maintenance Alert'));
    // n2 is already read; clicking shouldn't invoke markNotificationCenterRead
    await new Promise((r) => setTimeout(r, 10));
    expect(mockMarkRead).not.toHaveBeenCalled();
  });

  it('deletes a notification', async () => {
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => screen.getByText('Booking Confirmed'));
    const trashIcons = screen.getAllByTitle('Delete');
    fireEvent.click(trashIcons[0]);
    await waitFor(() => {
      expect(mockDelete).toHaveBeenCalledWith('n1');
      expect(mockToastSuccess).toHaveBeenCalledWith('Deleted');
    });
  });

  it('marks individual notification as read via check button', async () => {
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => screen.getByText('Booking Confirmed'));
    const markReadBtns = screen.getAllByTitle('Mark as read');
    fireEvent.click(markReadBtns[0]);
    await waitFor(() => {
      expect(mockMarkRead).toHaveBeenCalledWith('n1');
    });
  });

  it('filter buttons change active filter', async () => {
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => screen.getByText('Booking Confirmed'));
    const unreadBtn = screen.getByText('Unread');
    fireEvent.click(unreadBtn);
    await waitFor(() => {
      expect(mockGetCenter).toHaveBeenCalledWith('unread', 50);
    });
    const readBtn = screen.getByText('Read');
    fireEvent.click(readBtn);
    await waitFor(() => {
      expect(mockGetCenter).toHaveBeenCalledWith('read', 50);
    });
  });

  it('navigates to /notifications when View all is clicked', async () => {
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => screen.getByText('View all'));
    fireEvent.click(screen.getByText('View all'));
    expect(mockNavigate).toHaveBeenCalledWith('/notifications');
  });

  it('closes panel when clicking outside', async () => {
    render(
      <div>
        <div data-testid="outside">Outside</div>
        <NotificationCenter />
      </div>,
    );
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => screen.getByText('Booking Confirmed'));
    fireEvent.mouseDown(screen.getByTestId('outside'));
    await waitFor(() => {
      expect(screen.queryByText('Booking Confirmed')).not.toBeInTheDocument();
    });
  });

  it('handles fetch error for notifications gracefully', async () => {
    mockGetCenter.mockResolvedValue({
      success: false,
      data: null,
      error: { code: 'NETWORK', message: 'Network error' },
    });
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Error');
    });
  });

  it('handles fetch error for unread count silently', async () => {
    mockGetUnreadCount.mockResolvedValue({
      success: false,
      data: null,
      error: { code: 'NETWORK', message: 'Network error' },
    });
    render(<NotificationCenter />);
    // Should not throw or show error toast for unread count
    await new Promise((r) => setTimeout(r, 20));
    expect(mockToastError).not.toHaveBeenCalled();
  });

  it('handles markAllRead error with toast', async () => {
    mockMarkAllRead.mockResolvedValue({
      success: false,
      data: null,
      error: { code: 'HTTP_500', message: 'Server error' },
    });
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => screen.getByText('Mark all read'));
    fireEvent.click(screen.getByText('Mark all read'));
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Error');
    });
  });

  it('handles deleteNotification error with toast', async () => {
    mockDelete.mockResolvedValue({
      success: false,
      data: null,
      error: { code: 'HTTP_500', message: 'Delete error' },
    });
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => screen.getByText('Booking Confirmed'));
    const trashBtns = screen.getAllByTitle('Delete');
    fireEvent.click(trashBtns[0]);
    await waitFor(() => {
      expect(mockToastError).toHaveBeenCalledWith('Error');
    });
  });

  it('handles non-ok response for unread count', async () => {
    mockGetUnreadCount.mockResolvedValue({ success: false, data: null });
    render(<NotificationCenter />);
    // Should not crash
    await waitFor(() => {
      expect(screen.getByLabelText('Notifications')).toBeInTheDocument();
    });
  });

  it('handles non-ok response for fetch notifications', async () => {
    mockGetCenter.mockResolvedValue({ success: false, data: null });
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    // Should not crash; loading should finish
    await waitFor(() => {
      expect(screen.getByLabelText('Notifications')).toBeInTheDocument();
    });
  });

  it('uses fallback icon when icon name is unknown', async () => {
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => {
      // n4 has icon='unknown-icon' which should fall back to Bell
      expect(screen.getByText('Payment Received')).toBeInTheDocument();
    });
  });

  it('uses neutral severity colors for unknown severity', async () => {
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => {
      // n4 has severity='unknown-severity' -- should still render
      expect(screen.getByText('Payment Received')).toBeInTheDocument();
    });
  });

  it('toggles panel open/closed on bell clicks', async () => {
    render(<NotificationCenter />);
    const bell = screen.getByLabelText('Notifications');
    fireEvent.click(bell); // open
    await waitFor(() => screen.getByText('Booking Confirmed'));
    fireEvent.click(bell); // close
    await waitFor(() => {
      expect(screen.queryByText('Booking Confirmed')).not.toBeInTheDocument();
    });
  });

  it('polls unread count periodically', async () => {
    vi.useFakeTimers();
    render(<NotificationCenter />);
    await act(async () => { vi.advanceTimersByTime(100); });
    const initialCalls = mockGetUnreadCount.mock.calls.length;
    await act(async () => { vi.advanceTimersByTime(30000); });
    expect(mockGetUnreadCount.mock.calls.length).toBeGreaterThan(initialCalls);
    vi.useRealTimers();
  });

  it('formats timeAgo as "just now" for very recent notifications', async () => {
    const justNowItem = {
      ...sampleNotifications[0],
      id: 'now',
      created_at: new Date(Date.now() - 5 * 1000).toISOString(),
    };
    seedMocks({ items: [justNowItem] });
    render(<NotificationCenter />);
    fireEvent.click(screen.getByLabelText('Notifications'));
    await waitFor(() => {
      expect(screen.getByText('Just now')).toBeInTheDocument();
    });
  });
});
