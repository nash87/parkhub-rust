import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';

vi.mock('framer-motion', () => ({
  motion: {
    span: React.forwardRef(({ children, initial, animate, exit, transition, ...props }: any, ref: any) => (
      <span ref={ref} {...props}>{children}</span>
    )),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

import { NotificationBadge } from './NotificationBadge';

describe('NotificationBadge', () => {
  it('does not render when the count is zero', () => {
    const { container } = render(<NotificationBadge count={0} />);

    expect(container).toBeEmptyDOMElement();
  });

  it('renders the unread count and aria label', () => {
    render(<NotificationBadge count={3} />);

    expect(screen.getByText('3')).toBeInTheDocument();
    expect(screen.getByLabelText('3 unread')).toBeInTheDocument();
  });

  it('caps the displayed count at the configured maximum', () => {
    render(<NotificationBadge count={14} max={9} />);

    expect(screen.getByText('9+')).toBeInTheDocument();
    expect(screen.getByLabelText('14 unread')).toBeInTheDocument();
  });
});
