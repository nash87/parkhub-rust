import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Presence } from './Presence';

describe('Presence', () => {
  it('renders initials, tooltips, and implicit overflow', () => {
    render(
      <Presence
        maxShown={2}
        users={[
          { name: 'Lena Krug', color: '#2563eb', subtitle: 'L2-08' },
          { name: 'Max Bauer', color: '#16a34a' },
          { name: 'Nina Wolf', color: '#f97316' },
        ]}
      />,
    );

    expect(screen.getByRole('group', { name: 'Currently viewing' })).toBeInTheDocument();
    expect(screen.getByText('LK')).toHaveAttribute('title', 'Lena Krug · L2-08');
    expect(screen.getByText('MB')).toHaveAttribute('title', 'Max Bauer');
    expect(screen.getByText('+1')).toBeInTheDocument();
  });

  it('prefers explicit overflow counts over implicit truncation', () => {
    render(
      <Presence
        moreCount={7}
        users={[
          { name: 'Ada Lovelace', color: '#9333ea' },
          { name: 'Grace Hopper', color: '#dc2626' },
        ]}
      />,
    );

    expect(screen.getByText('AL')).toBeInTheDocument();
    expect(screen.getByText('GH')).toBeInTheDocument();
    expect(screen.getByText('+7')).toBeInTheDocument();
  });
});
