import { describe, it, expect, vi, beforeEach } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

const mockStats = vi.fn();
vi.mock('../../api/client', () => ({
  api: {
    getAdminStatsExtended: (...a: unknown[]) => mockStats(...a),
  },
}));

vi.mock('@number-flow/react', () => ({
  default: ({ value }: { value: number }) => <span>{value}</span>,
}));

// uPlot needs a real 2D canvas context; jsdom has none. Mock uplot and its CSS
// so the Analytics screen renders deterministically inside the test env.
vi.mock('uplot', () => {
  class FakeUPlot {
    constructor(_opts: unknown, _data: unknown, target: HTMLElement) {
      const canvas = document.createElement('canvas');
      target.appendChild(canvas);
    }
    destroy() {}
    setSize() {}
  }
  return { default: FakeUPlot };
});
vi.mock('uplot/dist/uPlot.min.css', () => ({}));

import { AnalyticsV5 } from './Analytics';

const HERE = dirname(fileURLToPath(import.meta.url));
const ANALYTICS_SRC = readFileSync(resolve(HERE, 'Analytics.tsx'), 'utf8');
const PRIMITIVES_SRC = readFileSync(resolve(HERE, '../primitives/index.tsx'), 'utf8');

const STATS = {
  total_users: 42,
  total_lots: 5,
  total_bookings: 1200,
  active_bookings: 7,
  occupancy_by_hour: { '08': 40, '09': 60, '10': 75 },
  occupancy_by_day: {
    Mo: { avg_percentage: 55, peak_hour: 10, peak_percentage: 80, bookings: 120 },
    Di: { avg_percentage: 65, peak_hour: 9, peak_percentage: 85, bookings: 140 },
  },
};

function renderScreen() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <AnalyticsV5 navigate={vi.fn()} />
    </QueryClientProvider>
  );
}

describe('AnalyticsV5', () => {
  beforeEach(() => vi.clearAllMocks());

  it('renders error state when stats fail', async () => {
    mockStats.mockResolvedValue({ success: false, data: null, error: { code: 'X', message: 'denied' } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Fehler beim Laden')).toBeInTheDocument());
  });

  it('renders KPI cards from stats', async () => {
    mockStats.mockResolvedValue({ success: true, data: STATS });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Nutzer')).toBeInTheDocument());
    expect(screen.getByText('42')).toBeInTheDocument();
    expect(screen.getByText('Aktive Buchungen')).toBeInTheDocument();
  });

  it('renders both charts as uPlot canvases with aria-labels', async () => {
    mockStats.mockResolvedValue({ success: true, data: STATS });
    const { container } = renderScreen();
    await waitFor(() =>
      expect(screen.getByRole('img', { name: /Auslastung nach Stunde/ })).toBeInTheDocument(),
    );
    expect(screen.getByRole('img', { name: /Auslastung nach Wochentag/ })).toBeInTheDocument();
    // uPlot renders canvas, not inline-SVG <rect>s — guard the regression.
    expect(container.querySelectorAll('canvas').length).toBeGreaterThanOrEqual(2);
    expect(container.querySelectorAll('[data-testid="analytics-chart"] rect').length).toBe(0);
  });

  it('gracefully handles missing occupancy data', async () => {
    mockStats.mockResolvedValue({ success: true, data: { total_users: 1, total_lots: 1, total_bookings: 1, active_bookings: 1 } });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Nutzer')).toBeInTheDocument());
    // Chart containers still present for both hour + day series (values may be 0, not empty).
    expect(screen.getByRole('img', { name: /Auslastung nach Stunde/ })).toBeInTheDocument();
    expect(screen.getByRole('img', { name: /Auslastung nach Wochentag/ })).toBeInTheDocument();
  });

  it('renders title', async () => {
    mockStats.mockResolvedValue({ success: true, data: STATS });
    renderScreen();
    await waitFor(() => expect(screen.getByText('Analytics')).toBeInTheDocument());
  });
});

describe('AnalyticsV5 — stable uPlot data reference (Codex #379)', () => {
  // Regression for Codex review comment 3136478619 on PR #379:
  // "ChartBlock passes a fresh array literal ([xs, ys]) on every render, and
  //  UPlotChart's effect depends on data, so any parent re-render tears down
  //  and recreates the uPlot instance even when the series values are
  //  unchanged."
  //
  // Two layers of defence:
  //   1. Source-text check: `data` must flow through `useMemo` with `[xs, ys]`
  //      (or equivalent value-equal deps). Inline `[xs, ys]` as a JSX prop is
  //      the banned pattern.
  //   2. Same for `options` — another stable-ref hot-path.
  //
  // Runtime behaviour is also exercised by the KPI/canvas tests above
  // (which would throw if the Suspense chunk collapsed under constant
  // re-mounts).

  it('ChartBlock memoizes the `data` tuple with [xs, ys] deps', () => {
    // data must be useMemo-d with xs + ys as deps so a query-status re-render
    // with unchanged values preserves the tuple identity.
    expect(ANALYTICS_SRC).toMatch(
      /const\s+data\s*=\s*useMemo\s*<\s*\[\s*number\[\]\s*,\s*number\[\]\s*\]\s*>\s*\(\s*\(\s*\)\s*=>\s*\[\s*xs\s*,\s*ys\s*\]\s*,\s*\[\s*xs\s*,\s*ys\s*\]\s*\)/,
    );
  });

  it('ChartBlock does NOT pass an inline [xs, ys] array to UPlotChart', () => {
    // The banned pattern: `data={[xs, ys]}` as a JSX attribute value.
    expect(ANALYTICS_SRC).not.toMatch(/data=\{\s*\[\s*xs\s*,\s*ys\s*\]\s*\}/);
  });

  it('ChartBlock memoizes the `options` object', () => {
    // Options holds closures over stroke/fill/tickLabels; memo keeps the
    // reference stable so UPlotChart's effect does not re-run.
    expect(ANALYTICS_SRC).toMatch(/const\s+options\s*=\s*useMemo\s*</);
  });
});

describe('AnalyticsV5 — Lighthouse LCP budget (lazy uPlot)', () => {
  it('Analytics.tsx dynamically imports UPlotChart via React.lazy', () => {
    // Eager imports of UPlotChart bloat the initial JS bundle (+~40KB uPlot)
    // and push LCP past the Lighthouse budget. The screen must instead use
    // React.lazy so uPlot lands in its own chunk loaded only for admins.
    expect(ANALYTICS_SRC).toMatch(
      /lazy\(\s*\(\)\s*=>\s*import\(\s*['"]\.\.\/primitives\/UPlotChart['"]\s*\)/,
    );
    // No eager `UPlotChart` import from '../primitives' barrel either.
    expect(ANALYTICS_SRC).not.toMatch(
      /import\s*{[^}]*\bUPlotChart\b[^}]*}\s*from\s*['"]\.\.\/primitives['"]/,
    );
  });

  it('Analytics.tsx wraps charts in <Suspense> with a skeleton fallback', () => {
    // The lazy chunk needs a Suspense boundary to avoid the whole app
    // collapsing while uPlot is in flight. Skeleton keeps LCP stable.
    expect(ANALYTICS_SRC).toMatch(/\bSuspense\b/);
    expect(ANALYTICS_SRC).toMatch(/ChartSkeleton/);
  });

  it('primitives barrel does NOT re-export UPlotChart as a runtime binding', () => {
    // 20+ screens import from the primitives barrel; re-exporting UPlotChart
    // there drags uPlot into every screen's module graph, defeating the
    // whole lazy-load. Screens that actually need the chart must deep-import.
    // Type-only re-exports (`export type { ... }`) are fine — they're erased.
    expect(PRIMITIVES_SRC).not.toMatch(
      /export\s*{[^}]*\bUPlotChart\b(?![A-Za-z])[^}]*}\s*from\s*['"]\.\/UPlotChart['"]/,
    );
    expect(PRIMITIVES_SRC).not.toMatch(
      /import\s*{[^}]*\bUPlotChart\b(?![A-Za-z])[^}]*}\s*from\s*['"]\.\/UPlotChart['"]/,
    );
  });
});
