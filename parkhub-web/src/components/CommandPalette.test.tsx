import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

// ── Mocks ──

const mockNavigate = vi.fn();

vi.mock('react-router-dom', () => ({
  useNavigate: () => mockNavigate,
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
  CalendarCheck: (props: any) => <span data-testid="icon-calendar-check" {...props} />,
  Car: (props: any) => <span data-testid="icon-car" {...props} />,
  UserCircle: (props: any) => <span data-testid="icon-user" {...props} />,
  Users: (props: any) => <span data-testid="icon-users" {...props} />,
  GearSix: (props: any) => <span data-testid="icon-gear" {...props} />,
  Coins: (props: any) => <span data-testid="icon-coins" {...props} />,
  Calendar: (props: any) => <span data-testid="icon-calendar" {...props} />,
  CalendarPlus: (props: any) => <span data-testid="icon-calendar-plus" {...props} />,
}));

import { CommandPalette } from './CommandPalette';

describe('CommandPalette', () => {
  const onClose = vi.fn();

  beforeEach(() => {
    mockNavigate.mockClear();
    onClose.mockClear();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('renders nothing when closed', () => {
    const { container } = render(<CommandPalette open={false} onClose={onClose} />);
    expect(container.querySelector('[role="dialog"]')).not.toBeInTheDocument();
  });

  it('renders dialog when open', () => {
    render(<CommandPalette open={true} onClose={onClose} />);
    expect(screen.getByRole('dialog')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Type a command...')).toBeInTheDocument();
  });

  it('renders all action items', () => {
    render(<CommandPalette open={true} onClose={onClose} />);
    expect(screen.getByText('Book a Spot')).toBeInTheDocument();
    expect(screen.getByText('Bookings')).toBeInTheDocument();
    expect(screen.getByText('Vehicles')).toBeInTheDocument();
    expect(screen.getByText('Profile')).toBeInTheDocument();
    expect(screen.getByText('Admin')).toBeInTheDocument();
    expect(screen.getByText('Credits')).toBeInTheDocument();
    expect(screen.getByText('Calendar')).toBeInTheDocument();
    expect(screen.getByText('Team')).toBeInTheDocument();
  });

  it('shows keyboard shortcut for Book a Spot', () => {
    render(<CommandPalette open={true} onClose={onClose} />);
    expect(screen.getByText('Ctrl+B')).toBeInTheDocument();
  });

  it('filters actions based on search query', async () => {
    const user = userEvent.setup();
    render(<CommandPalette open={true} onClose={onClose} />);

    const input = screen.getByPlaceholderText('Type a command...');
    await user.type(input, 'book');

    expect(screen.getByText('Book a Spot')).toBeInTheDocument();
    expect(screen.getByText('Bookings')).toBeInTheDocument();
    expect(screen.queryByText('Vehicles')).not.toBeInTheDocument();
    expect(screen.queryByText('Profile')).not.toBeInTheDocument();
  });

  it('shows "No results" when search has no matches', async () => {
    const user = userEvent.setup();
    render(<CommandPalette open={true} onClose={onClose} />);

    const input = screen.getByPlaceholderText('Type a command...');
    await user.type(input, 'zzzzz');

    expect(screen.getByText('No results')).toBeInTheDocument();
  });

  it('navigates when clicking an action', async () => {
    const user = userEvent.setup();
    render(<CommandPalette open={true} onClose={onClose} />);

    await user.click(screen.getByText('Profile'));

    expect(onClose).toHaveBeenCalled();
    expect(mockNavigate).toHaveBeenCalledWith('/profile');
  });

  it('calls onClose when clicking backdrop', async () => {
    const user = userEvent.setup();
    render(<CommandPalette open={true} onClose={onClose} />);

    await user.click(screen.getByTestId('command-palette-backdrop'));
    expect(onClose).toHaveBeenCalled();
  });

  it('renders footer hints', () => {
    render(<CommandPalette open={true} onClose={onClose} />);
    expect(screen.getByText('navigate')).toBeInTheDocument();
    expect(screen.getByText('select')).toBeInTheDocument();
    expect(screen.getByText('close')).toBeInTheDocument();
  });

  it('renders search input with test id', () => {
    render(<CommandPalette open={true} onClose={onClose} />);
    expect(screen.getByTestId('command-palette-input')).toBeInTheDocument();
  });
});
