import { afterEach, describe, expect, it } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { HelpTip } from './HelpTip';

describe('HelpTip', () => {
  afterEach(() => cleanup());

  it('renders a button with the provided accessible label', () => {
    render(<HelpTip label="What is a credit?">Credits reset monthly.</HelpTip>);
    const btn = screen.getByRole('button', { name: 'What is a credit?' });
    expect(btn).toBeInTheDocument();
    expect(btn).toHaveAttribute('aria-expanded', 'false');
  });

  it('toggles the popover open on click with aria-expanded', () => {
    render(<HelpTip label="More info">The explanation text.</HelpTip>);
    const btn = screen.getByRole('button', { name: 'More info' });
    fireEvent.click(btn);
    expect(btn).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByRole('tooltip')).toHaveTextContent('The explanation text.');
  });

  it('closes on Escape keypress', () => {
    render(<HelpTip label="Tip">Body</HelpTip>);
    const btn = screen.getByRole('button', { name: 'Tip' });
    fireEvent.click(btn);
    expect(screen.getByRole('tooltip')).toBeInTheDocument();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('closes when clicking outside', () => {
    render(
      <>
        <HelpTip label="Tip">Body</HelpTip>
        <div data-testid="outside">outside</div>
      </>
    );
    const btn = screen.getByRole('button', { name: 'Tip' });
    fireEvent.click(btn);
    expect(screen.getByRole('tooltip')).toBeInTheDocument();
    fireEvent.mouseDown(screen.getByTestId('outside'));
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });
});
