import { useMemo } from 'react';
import { useFeatures } from '../context/FeaturesContext';

/**
 * Generative background patterns — industrial-luxury aesthetic.
 * Rendered as a fixed full-screen layer behind all content.
 * Only active when `generative_bg` feature is enabled.
 *
 * Available patterns:
 *  - topo: topographic contour lines (engineering drawings)
 *  - dots: precision dot matrix
 *  - hatch: diagonal safety-zone hatching
 *  - mesh: warm radial mesh gradient
 *  - grid: blueprint grid (default parking-grid)
 */

export type BgPattern = 'topo' | 'dots' | 'hatch' | 'mesh' | 'grid';

interface Props {
  /** Which pattern to render. Defaults to 'topo'. */
  pattern?: BgPattern;
  /** Additional className for the container */
  className?: string;
}

/** CSS class mapping for each pattern */
const PATTERN_CLASS: Record<BgPattern, string> = {
  topo: 'topo-bg',
  dots: 'dot-matrix',
  hatch: 'hatch-bg',
  mesh: 'mesh-gradient',
  grid: 'parking-grid',
};

export function GenerativeBg({ pattern = 'topo', className = '' }: Props) {
  const { isEnabled } = useFeatures();

  // Fall back to plain background when feature is off
  if (!isEnabled('generative_bg')) return null;

  return (
    <div
      className={`fixed inset-0 -z-10 ${PATTERN_CLASS[pattern]} ${className}`}
      aria-hidden="true"
    />
  );
}

/**
 * Hook to get background class for inline use (e.g. on page wrappers).
 * Returns empty string when generative_bg is disabled.
 */
export function useBgClass(pattern: BgPattern = 'topo'): string {
  const { isEnabled } = useFeatures();
  return useMemo(
    () => (isEnabled('generative_bg') ? PATTERN_CLASS[pattern] : ''),
    [isEnabled, pattern],
  );
}
