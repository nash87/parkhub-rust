import { describe, it, expect, vi, afterEach } from 'vitest';
import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (k: string) => k }),
}));

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, ...props }: any, ref: any) => <div ref={ref} {...props}>{children}</div>),
  },
  AnimatePresence: ({ children }: any) => <>{children}</>,
}));

vi.mock('@phosphor-icons/react', () => ({
  Warning: (props: any) => <span data-testid="icon-warning" {...props} />,
}));

import { ConfirmDialog } from './ConfirmDialog';

describe('ConfirmDialog', () => {
  afterEach(() => vi.restoreAllMocks());

  it('renders nothing when closed', () => {
    const { container } = render(
      <ConfirmDialog open={false} title="T" message="M" onConfirm={vi.fn()} onCancel={vi.fn()} />,
    );
    expect(container.innerHTML).toBe('');
  });

  it('renders title and message when open', () => {
    render(<ConfirmDialog open={true} title="Delete?" message="This is permanent" onConfirm={vi.fn()} onCancel={vi.fn()} />);
    expect(screen.getByText('Delete?')).toBeInTheDocument();
    expect(screen.getByText('This is permanent')).toBeInTheDocument();
  });

  it('calls onCancel when cancel button clicked', () => {
    const cancel = vi.fn();
    render(<ConfirmDialog open={true} title="T" message="M" onConfirm={vi.fn()} onCancel={cancel} />);
    fireEvent.click(screen.getByText('common.cancel'));
    expect(cancel).toHaveBeenCalled();
  });

  it('calls onConfirm when confirm button clicked', () => {
    const confirm = vi.fn();
    render(<ConfirmDialog open={true} title="T" message="M" onConfirm={confirm} onCancel={vi.fn()} />);
    fireEvent.click(screen.getByText('common.delete'));
    expect(confirm).toHaveBeenCalled();
  });

  it('renders custom labels', () => {
    render(<ConfirmDialog open={true} title="T" message="M" confirmLabel="Yes" cancelLabel="No" onConfirm={vi.fn()} onCancel={vi.fn()} />);
    expect(screen.getByText('Yes')).toBeInTheDocument();
    expect(screen.getByText('No')).toBeInTheDocument();
  });

  it('shows warning icon for danger variant', () => {
    render(<ConfirmDialog open={true} title="T" message="M" variant="danger" onConfirm={vi.fn()} onCancel={vi.fn()} />);
    expect(screen.getByTestId('icon-warning')).toBeInTheDocument();
  });

  it('does not show warning icon for default variant', () => {
    render(<ConfirmDialog open={true} title="T" message="M" variant="default" onConfirm={vi.fn()} onCancel={vi.fn()} />);
    expect(screen.queryByTestId('icon-warning')).not.toBeInTheDocument();
  });

  it('calls onCancel on Escape key', () => {
    const cancel = vi.fn();
    render(<ConfirmDialog open={true} title="T" message="M" onConfirm={vi.fn()} onCancel={cancel} />);
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(cancel).toHaveBeenCalled();
  });

  it('calls onCancel when backdrop clicked', () => {
    const cancel = vi.fn();
    render(<ConfirmDialog open={true} title="T" message="M" onConfirm={vi.fn()} onCancel={cancel} />);
    // The backdrop is the first motion.div with aria-hidden
    const backdrop = document.querySelector('[aria-hidden="true"]');
    if (backdrop) fireEvent.click(backdrop);
    expect(cancel).toHaveBeenCalled();
  });

  it('has alertdialog role', () => {
    render(<ConfirmDialog open={true} title="T" message="M" onConfirm={vi.fn()} onCancel={vi.fn()} />);
    expect(screen.getByRole('alertdialog')).toBeInTheDocument();
  });
});
