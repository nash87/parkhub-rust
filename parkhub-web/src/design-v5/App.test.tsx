import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { V5MobileNav } from './App';

describe('V5MobileNav', () => {
  it('renders all navigation groups and marks the active screen', () => {
    render(
      <V5MobileNav
        open
        active="dashboard"
        onClose={vi.fn()}
        onNavigate={vi.fn()}
      />,
    );

    expect(screen.getByRole('dialog', { name: 'Navigation' })).toBeInTheDocument();
    expect(screen.getByText('Grundlagen')).toBeInTheDocument();
    expect(screen.getByText('Flotte')).toBeInTheDocument();
    expect(screen.getByText('Admin')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Dashboard' })).toHaveAttribute(
      'aria-current',
      'page',
    );
    expect(screen.getByRole('button', { name: 'API-Schlüssel' })).toBeInTheDocument();
  });

  it('navigates and closes from the mobile sheet controls', () => {
    const onClose = vi.fn();
    const onNavigate = vi.fn();
    render(
      <V5MobileNav
        open
        active="dashboard"
        onClose={onClose}
        onNavigate={onNavigate}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Nutzer' }));
    fireEvent.click(screen.getAllByRole('button', { name: 'Navigation schließen' })[1]!);

    expect(onNavigate).toHaveBeenCalledWith('nutzer');
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('does not mount the dialog while closed', () => {
    render(
      <V5MobileNav
        open={false}
        active="dashboard"
        onClose={vi.fn()}
        onNavigate={vi.fn()}
      />,
    );

    expect(screen.queryByRole('dialog', { name: 'Navigation' })).toBeNull();
  });
});
