import { useEffect, useRef } from 'react';
import uPlot from 'uplot';
import 'uplot/dist/uPlot.min.css';

/* ═════════════════════════════════════════════════════════════
   UPlotChart — thin React wrapper around uPlot (MIT, ~40KB).

   Canvas-rendered, zero-runtime-deps, ~2-3× Chart.js perf at
   ~1/5 the bundle. Tokens drive color via --v5-acc / --v5-acc-muted.

   Contract:
     - `data`   : uPlot.AlignedData — [xs[], ...series[][]]
     - `options`: optional partial uPlot.Options overrides
     - `ariaLabel`: placed on inner <canvas> for screen readers
     - `height` : px, defaults to 240
     - Destroys the uPlot instance on unmount (no leaks)
     - Listens to window resize + calls plot.setSize() responsively
   ═════════════════════════════════════════════════════════════ */

export interface UPlotChartProps {
  data: uPlot.AlignedData;
  options?: Partial<uPlot.Options>;
  ariaLabel?: string;
  height?: number;
}

export function UPlotChart({ data, options, ariaLabel, height = 240 }: UPlotChartProps) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const plotRef = useRef<uPlot | null>(null);

  // Guard: uPlot requires >=1 x-value. Render a labelled empty slot instead.
  const hasData = Array.isArray(data) && data.length > 0 && Array.isArray(data[0]) && data[0].length > 0;

  useEffect(() => {
    if (!hasData || !hostRef.current) return;
    const host = hostRef.current;
    const width = host.clientWidth || 520;

    const baseOpts: uPlot.Options = {
      width,
      height,
      // Caller may override any of this via `options`
      series: [
        {},
        {
          stroke: 'var(--v5-acc)',
          fill: 'var(--v5-acc-muted)',
          width: 2,
          points: { show: false },
        },
      ],
      scales: {
        x: { time: false },
      },
      legend: { show: false },
      cursor: { drag: { x: false, y: false } },
      ...(options ?? {}),
    };

    const plot = new uPlot(baseOpts, data, host);
    plotRef.current = plot;

    // Apply aria-label to the rendered canvas for screen readers.
    if (ariaLabel) {
      const canvas = host.querySelector('canvas');
      if (canvas) canvas.setAttribute('aria-label', ariaLabel);
    }

    const onResize = () => {
      if (!hostRef.current || !plotRef.current) return;
      plotRef.current.setSize({ width: hostRef.current.clientWidth, height });
    };
    window.addEventListener('resize', onResize);

    return () => {
      window.removeEventListener('resize', onResize);
      plot.destroy();
      plotRef.current = null;
    };
  }, [data, options, ariaLabel, height, hasData]);

  if (!hasData) {
    return (
      <div
        role="img"
        aria-label={ariaLabel}
        style={{ fontSize: 11, color: 'var(--v5-mut)', padding: '14px 0' }}
      >
        Keine Daten
      </div>
    );
  }

  return (
    <div
      ref={hostRef}
      role="img"
      aria-label={ariaLabel}
      style={{ width: '100%' }}
    />
  );
}
