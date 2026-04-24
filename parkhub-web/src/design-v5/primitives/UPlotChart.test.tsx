import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render } from '@testing-library/react';

// Mock uplot BEFORE importing UPlotChart — jsdom has no canvas 2d ctx.
// We record constructor args + expose destroy/setSize spies so the tests
// can verify our wrapper's contract with uPlot without booting a real chart.
const destroySpy = vi.fn();
const setSizeSpy = vi.fn();
const ctorSpy = vi.fn();

vi.mock('uplot', () => {
  class FakeUPlot {
    constructor(opts: unknown, data: unknown, target: HTMLElement) {
      ctorSpy(opts, data, target);
      // mimic uPlot's DOM: append a canvas child so our wrapper can set aria-label
      const canvas = document.createElement('canvas');
      target.appendChild(canvas);
    }
    destroy = destroySpy;
    setSize = setSizeSpy;
  }
  return { default: FakeUPlot };
});

// Avoid CSS import pulling in real file during jsdom test — stub it.
vi.mock('uplot/dist/uPlot.min.css', () => ({}));

import { UPlotChart } from './UPlotChart';

describe('UPlotChart', () => {
  beforeEach(() => {
    destroySpy.mockClear();
    setSizeSpy.mockClear();
    ctorSpy.mockClear();
  });

  it('renders a canvas element with the given aria-label', () => {
    const { container } = render(
      <UPlotChart data={[[1, 2, 3], [10, 20, 30]]} ariaLabel="Buchungen pro Tag" />
    );
    const canvas = container.querySelector('canvas');
    expect(canvas).toBeInTheDocument();
    expect(canvas?.getAttribute('aria-label')).toBe('Buchungen pro Tag');
  });

  it('exposes role="img" on the chart container for a11y', () => {
    const { container } = render(
      <UPlotChart data={[[1, 2], [5, 10]]} ariaLabel="Stunden" />
    );
    const img = container.querySelector('[role="img"]');
    expect(img).toBeInTheDocument();
    expect(img?.getAttribute('aria-label')).toBe('Stunden');
  });

  it('disposes uPlot instance on unmount', () => {
    const { unmount } = render(<UPlotChart data={[[1, 2], [5, 10]]} />);
    expect(destroySpy).not.toHaveBeenCalled();
    unmount();
    expect(destroySpy).toHaveBeenCalledTimes(1);
  });

  it('constructs uPlot with the provided aligned data', () => {
    render(<UPlotChart data={[[1, 2, 3], [10, 20, 30]]} />);
    expect(ctorSpy).toHaveBeenCalledTimes(1);
    const [, data] = ctorSpy.mock.calls[0];
    expect(data).toEqual([[1, 2, 3], [10, 20, 30]]);
  });

  it('renders placeholder when data is empty (no x values)', () => {
    const { container } = render(<UPlotChart data={[[], []]} ariaLabel="Leer" />);
    // No canvas should be mounted when there's nothing to plot
    expect(container.querySelector('canvas')).not.toBeInTheDocument();
    expect(ctorSpy).not.toHaveBeenCalled();
  });
});
