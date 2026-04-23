import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { Badge, Card, LiveDot, Row, SectionLabel, StatCard, Toggle, V5Icon, V5NamedIcon } from './index';

describe('v5 primitives', () => {
  describe('Badge', () => {
    it('renders label', () => {
      render(<Badge>42 items</Badge>);
      expect(screen.getByText('42 items')).toBeInTheDocument();
    });

    it('renders a dot when requested', () => {
      const { container } = render(<Badge dot variant="success">Live</Badge>);
      // The dot is an empty span; the label still renders.
      expect(screen.getByText('Live')).toBeInTheDocument();
      expect(container.querySelectorAll('span').length).toBeGreaterThan(1);
    });
  });

  describe('Card', () => {
    it('renders children with lift class by default', () => {
      const { container } = render(<Card>content</Card>);
      expect(screen.getByText('content')).toBeInTheDocument();
      expect(container.firstChild).toHaveClass('v5-lift');
    });

    it('opts out of lift when lift={false}', () => {
      const { container } = render(<Card lift={false}>plain</Card>);
      expect(container.firstChild).not.toHaveClass('v5-lift');
    });
  });

  describe('StatCard', () => {
    it('renders label, value and sub', () => {
      render(<StatCard label="Credits" value={40} sub="kein Ablauf" icon="credit" />);
      expect(screen.getByText('Credits')).toBeInTheDocument();
      expect(screen.getByText('40')).toBeInTheDocument();
      expect(screen.getByText('kein Ablauf')).toBeInTheDocument();
    });
  });

  describe('SectionLabel', () => {
    it('renders uppercase caps label', () => {
      render(<SectionLabel>zones</SectionLabel>);
      expect(screen.getByText('zones')).toBeInTheDocument();
    });
  });

  describe('Row', () => {
    it('renders label + sub + child', () => {
      render(
        <Row label="Email" sub="Receive booking updates">
          <span>toggle-here</span>
        </Row>
      );
      expect(screen.getByText('Email')).toBeInTheDocument();
      expect(screen.getByText('Receive booking updates')).toBeInTheDocument();
      expect(screen.getByText('toggle-here')).toBeInTheDocument();
    });
  });

  describe('Toggle', () => {
    it('exposes role=switch with aria-checked', () => {
      render(<Toggle checked={false} ariaLabel="Dark mode" />);
      const sw = screen.getByRole('switch', { name: 'Dark mode' });
      expect(sw).toHaveAttribute('aria-checked', 'false');
    });

    it('calls onChange with inverted value', async () => {
      const onChange = vi.fn();
      render(<Toggle checked={false} onChange={onChange} ariaLabel="notifications" />);
      screen.getByRole('switch').click();
      expect(onChange).toHaveBeenCalledWith(true);
    });
  });

  describe('LiveDot', () => {
    it('renders aria-hidden span', () => {
      const { container } = render(<LiveDot />);
      const dot = container.querySelector('span');
      expect(dot).toBeInTheDocument();
      expect(dot).toHaveAttribute('aria-hidden', 'true');
    });
  });

  describe('V5Icon', () => {
    it('renders an svg with the provided single path', () => {
      const { container } = render(<V5Icon d="M0 0h24v24H0z" />);
      const svg = container.querySelector('svg');
      expect(svg).toBeInTheDocument();
      expect(svg).toHaveAttribute('aria-hidden', 'true');
      expect(container.querySelectorAll('path')).toHaveLength(1);
    });

    it('renders multiple paths when given an array', () => {
      const { container } = render(<V5Icon d={['M0 0h1v1H0z', 'M2 2h1v1H2z']} />);
      expect(container.querySelectorAll('path')).toHaveLength(2);
    });
  });

  describe('V5NamedIcon', () => {
    it('renders from the registry key', () => {
      const { container } = render(<V5NamedIcon name="home" />);
      expect(container.querySelector('svg')).toBeInTheDocument();
      expect(container.querySelectorAll('path').length).toBeGreaterThan(0);
    });
  });
});
