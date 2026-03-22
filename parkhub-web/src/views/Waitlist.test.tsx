import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, any>) => {
      const map: Record<string, string> = {
        'waitlistExt.title': 'Waitlist',
        'waitlistExt.subtitle': 'Get notified when a spot becomes available',
        'waitlistExt.help': 'Join the waitlist to get notified when a parking spot becomes available. You will receive a notification and have 15 minutes to accept.',
        'waitlistExt.helpLabel': 'Help',
        'waitlistExt.yourEntries': 'Your Waitlist Entries',
        'waitlistExt.fullLots': 'Full Parking Lots',
        'waitlistExt.joinWaitlist': 'Join Waitlist',
        'waitlistExt.joiningWaitlist': 'Joining...',
        'waitlistExt.leave': 'Leave',
        'waitlistExt.accept': 'Accept',
        'waitlistExt.decline': 'Decline',
        'waitlistExt.joined': 'Joined waitlist',
        'waitlistExt.left': 'Left waitlist',
        'waitlistExt.accepted': 'Offer accepted',
        'waitlistExt.declined': 'Offer declined',
        'waitlistExt.noFullLots': 'All lots have available spots',
        'waitlistExt.status.waiting': 'Waiting',
        'waitlistExt.status.offered': 'Offered',
        'waitlistExt.status.accepted': 'Accepted',
        'waitlistExt.status.declined': 'Declined',
        'waitlistExt.status.expired': 'Expired',
        'waitlistExt.position': `Position #${opts?.pos}`,
        'waitlistExt.estimatedWait': `~${opts?.minutes} min`,
        'waitlistExt.lotFull': `${opts?.total} spots — all occupied`,
        'common.error': 'Error',
      };
      return map[key] || key;
    },
  }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <div ref={ref} {...props}>{children}</div>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Bell: (props: any) => <span data-testid="icon-bell" {...props} />,
  Queue: (props: any) => <span data-testid="icon-queue" {...props} />,
  Check: (props: any) => <span data-testid="icon-check" {...props} />,
  X: (props: any) => <span data-testid="icon-x" {...props} />,
  Question: (props: any) => <span data-testid="icon-question" {...props} />,
  Clock: (props: any) => <span data-testid="icon-clock" {...props} />,
  ArrowUp: (props: any) => <span data-testid="icon-arrow-up" {...props} />,
}));

import { WaitlistPage } from './Waitlist';

const sampleLots = [
  { id: 'lot-1', name: 'Garage Alpha', total_slots: 20, available_slots: 0 },
  { id: 'lot-2', name: 'Garage Beta', total_slots: 10, available_slots: 3 },
];

const sampleWaitlistResponse = {
  success: true,
  data: {
    total: 5,
    entries: [
      {
        entry: {
          id: 'w1',
          user_id: 'user-1',
          lot_id: 'lot-1',
          created_at: '2026-03-20T08:00:00Z',
          notified_at: null,
          status: 'waiting',
          offer_expires_at: null,
          accepted_booking_id: null,
        },
        position: 3,
        total_ahead: 2,
        estimated_wait_minutes: 60,
      },
    ],
  },
};

describe('WaitlistPage', () => {
  beforeEach(() => {
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/api/v1/lots') && !url.includes('waitlist')) {
        return Promise.resolve({
          json: () => Promise.resolve({ success: true, data: sampleLots }),
        } as Response);
      }
      if (typeof url === 'string' && url.includes('/waitlist') && !url.includes('subscribe')) {
        return Promise.resolve({
          json: () => Promise.resolve(sampleWaitlistResponse),
        } as Response);
      }
      return Promise.resolve({
        json: () => Promise.resolve({ success: true, data: {} }),
      } as Response);
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders the waitlist page with title', async () => {
    render(<WaitlistPage />);
    await waitFor(() => expect(screen.getByText('Waitlist')).toBeTruthy());
  });

  it('shows subtitle', async () => {
    render(<WaitlistPage />);
    await waitFor(() =>
      expect(screen.getByText('Get notified when a spot becomes available')).toBeTruthy()
    );
  });

  it('shows help tooltip when clicking question icon', async () => {
    render(<WaitlistPage />);
    await waitFor(() => screen.getByText('Waitlist'));
    const helpBtn = screen.getByLabelText('Help');
    fireEvent.click(helpBtn);
    await waitFor(() =>
      expect(screen.getByText(/Join the waitlist to get notified/)).toBeTruthy()
    );
  });

  it('displays full lots with join button', async () => {
    // Return no waitlist entries for the user
    global.fetch = vi.fn((url: string) => {
      if (typeof url === 'string' && url.includes('/api/v1/lots') && !url.includes('waitlist')) {
        return Promise.resolve({
          json: () => Promise.resolve({ success: true, data: sampleLots }),
        } as Response);
      }
      return Promise.resolve({
        json: () => Promise.resolve({ success: true, data: { total: 0, entries: [] } }),
      } as Response);
    });

    render(<WaitlistPage />);
    await waitFor(() => {
      expect(screen.getByText('Garage Alpha')).toBeTruthy();
      expect(screen.getByText('Join Waitlist')).toBeTruthy();
    });
  });

  it('shows waiting status with position', async () => {
    render(<WaitlistPage />);
    await waitFor(() => {
      expect(screen.getByText('Waiting')).toBeTruthy();
      expect(screen.getByText('Position #3')).toBeTruthy();
    });
  });

  it('shows estimated wait time', async () => {
    render(<WaitlistPage />);
    await waitFor(() => {
      expect(screen.getByText('~60 min')).toBeTruthy();
    });
  });

  it('shows empty state when no full lots', async () => {
    global.fetch = vi.fn(() =>
      Promise.resolve({
        json: () => Promise.resolve({
          success: true,
          data: [{ id: 'lot-1', name: 'Garage A', total_slots: 10, available_slots: 5 }],
        }),
      } as Response)
    );

    render(<WaitlistPage />);
    await waitFor(() =>
      expect(screen.getByText('All lots have available spots')).toBeTruthy()
    );
  });
});
