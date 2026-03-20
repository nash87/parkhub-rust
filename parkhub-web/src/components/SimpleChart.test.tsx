import { describe, it, expect } from 'vitest';
import React from 'react';
import { render } from '@testing-library/react';
import { BarChart } from './SimpleChart';

describe('BarChart', () => {
  const sampleData = [
    { label: 'Mon', value: 10 },
    { label: 'Tue', value: 25 },
    { label: 'Wed', value: 15 },
  ];

  it('renders nothing when data is empty', () => {
    const { container } = render(<BarChart data={[]} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders an SVG with correct aria-label', () => {
    const { container } = render(<BarChart data={sampleData} />);
    const svg = container.querySelector('svg');
    expect(svg).toBeTruthy();
    expect(svg!.getAttribute('aria-label')).toBe('Bar chart with 3 items');
  });

  it('renders a group per data item', () => {
    const { container } = render(<BarChart data={sampleData} />);
    const groups = container.querySelectorAll('svg > g');
    expect(groups.length).toBe(3);
  });

  it('renders labels for each bar', () => {
    const { container } = render(<BarChart data={sampleData} />);
    const texts = container.querySelectorAll('text');
    // Each item has 2 text elements: label + value
    expect(texts.length).toBe(6);
    expect(texts[0].textContent).toBe('Mon');
    expect(texts[1].textContent).toBe('10');
  });

  it('accepts a custom color', () => {
    const { container } = render(<BarChart data={sampleData} color="red" />);
    const bar = container.querySelector('foreignObject div') as HTMLElement;
    expect(bar.style.background).toBe('red');
  });

  it('accepts a custom height', () => {
    const { container } = render(<BarChart data={sampleData} height={200} />);
    const svg = container.querySelector('svg');
    expect(svg!.getAttribute('height')).toBe('200');
  });

  it('handles all-zero values without crashing', () => {
    const zeros = [{ label: 'A', value: 0 }, { label: 'B', value: 0 }];
    const { container } = render(<BarChart data={zeros} />);
    const svg = container.querySelector('svg');
    expect(svg).toBeTruthy();
  });
});
