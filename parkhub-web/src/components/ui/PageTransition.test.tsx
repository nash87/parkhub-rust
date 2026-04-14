import { describe, it, expect, vi } from 'vitest';
import React from 'react';
import { render, screen } from '@testing-library/react';

vi.mock('framer-motion', () => ({
  motion: {
    div: React.forwardRef(({ children, initial, animate, exit, transition, variants, ...props }: any, ref: any) => (
      <div ref={ref} {...props} data-testid="motion-div">{children}</div>
    )),
  },
}));

import { PageTransition } from './PageTransition';

describe('PageTransition', () => {
  it('renders children', () => {
    render(
      <PageTransition>
        <p>Page content</p>
      </PageTransition>
    );
    expect(screen.getByText('Page content')).toBeInTheDocument();
  });

  it('renders a motion.div wrapper', () => {
    render(
      <PageTransition>
        <span>inner</span>
      </PageTransition>
    );
    expect(screen.getByTestId('motion-div')).toBeInTheDocument();
  });

  it('applies className prop', () => {
    render(
      <PageTransition className="page-wrapper">
        content
      </PageTransition>
    );
    expect(screen.getByTestId('motion-div')).toHaveClass('page-wrapper');
  });

  it('renders without className (optional)', () => {
    render(
      <PageTransition>
        no class
      </PageTransition>
    );
    expect(screen.getByText('no class')).toBeInTheDocument();
  });
});
