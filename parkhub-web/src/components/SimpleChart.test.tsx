import { describe, it, expect } from 'vitest';
import React from 'react';
import { render } from '@testing-library/react';
import { BarChart, DonutChart, type DonutSlice } from './SimpleChart';

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

describe('DonutChart', () => {
  const sampleSlices: DonutSlice[] = [
    { label: 'Lot A', occupancy: 45, capacity: 100 },
    { label: 'Lot B', occupancy: 70, capacity: 80 },
    { label: 'Lot C', occupancy: 90, capacity: 50 },
  ];

  it('renders nothing when slices are empty', () => {
    const { container } = render(<DonutChart slices={[]} />);
    expect(container.firstChild).toBeNull();
  });

  it('renders an SVG with aria-label', () => {
    const { container } = render(<DonutChart slices={sampleSlices} />);
    const svg = container.querySelector('svg');
    expect(svg).toBeTruthy();
    expect(svg!.getAttribute('aria-label')).toMatch(/occupancy/i);
  });

  it('renders one arc circle per slice plus the background track', () => {
    const { container } = render(<DonutChart slices={sampleSlices} />);
    // background circle + one per slice
    const circles = container.querySelectorAll('circle');
    expect(circles.length).toBe(sampleSlices.length + 1);
  });

  it('shows overall occupancy percentage in center text', () => {
    // All capacity 100, all at 50% -> overall 50%
    const uniform: DonutSlice[] = [
      { label: 'A', occupancy: 50, capacity: 100 },
      { label: 'B', occupancy: 50, capacity: 100 },
    ];
    const { container } = render(<DonutChart slices={uniform} />);
    const texts = container.querySelectorAll('text');
    const centerText = Array.from(texts).find(t => t.textContent?.includes('%'));
    expect(centerText).toBeTruthy();
    expect(centerText!.textContent).toBe('50%');
  });

  it('respects custom size prop', () => {
    const { container } = render(<DonutChart slices={sampleSlices} size={240} />);
    const svg = container.querySelector('svg');
    expect(svg!.getAttribute('width')).toBe('240');
    expect(svg!.getAttribute('height')).toBe('240');
  });

  it('renders title tooltip for each slice', () => {
    const { container } = render(<DonutChart slices={sampleSlices} />);
    const titles = container.querySelectorAll('title');
    expect(titles.length).toBe(sampleSlices.length);
    expect(titles[0].textContent).toContain('Lot A');
  });
});
