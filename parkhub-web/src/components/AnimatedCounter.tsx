import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

/**
 * Animated number counter with cubic ease-out.
 * Numbers animate up from previous value on change.
 *
 * Accessibility:
 * - The animated digits are rendered `aria-hidden` so screen readers don't
 *   re-announce every interpolated tick.
 * - A parallel `role="status"` + `aria-live="polite"` element holds the
 *   final formatted value so assistive tech gets one update per value
 *   change instead of 60.
 * - `format` controls the user-visible string: `'number'` (default, grouped
 *   locale digits), `'currency'` with `currency` (e.g. 'EUR'), or `'percent'`.
 * - Respects `prefers-reduced-motion`: no animation, just swap.
 */
export function AnimatedCounter({
  value,
  duration = 1000,
  className,
  format = 'number',
  currency,
  maximumFractionDigits = 0,
}: {
  value: number;
  duration?: number;
  className?: string;
  format?: 'number' | 'currency' | 'percent';
  currency?: string;
  maximumFractionDigits?: number;
}) {
  const { i18n } = useTranslation();
  const locale = i18n.language || 'en';

  const formatter = useMemo(() => {
    const opts: Intl.NumberFormatOptions = { maximumFractionDigits };
    if (format === 'currency' && currency) {
      opts.style = 'currency';
      opts.currency = currency;
    } else if (format === 'percent') {
      opts.style = 'percent';
    }
    return new Intl.NumberFormat(locale, opts);
  }, [locale, format, currency, maximumFractionDigits]);

  const prefersReducedMotion = useMemo(() => {
    if (typeof window === 'undefined' || !window.matchMedia) return false;
    return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  }, []);

  const [display, setDisplay] = useState(prefersReducedMotion ? value : 0);
  const ref = useRef<number>(prefersReducedMotion ? value : 0);
  const rafRef = useRef<number>(0);

  useEffect(() => {
    if (prefersReducedMotion) {
      setDisplay(value);
      ref.current = value;
      return;
    }
    const start = ref.current;
    const diff = value - start;
    if (diff === 0) return;

    const startTime = performance.now();

    function animate(now: number) {
      const elapsed = now - startTime;
      const progress = Math.min(elapsed / duration, 1);
      // cubic ease-out
      const eased = 1 - Math.pow(1 - progress, 3);
      const current = Math.round(start + diff * eased);
      setDisplay(current);
      if (progress < 1) {
        rafRef.current = requestAnimationFrame(animate);
      } else {
        ref.current = value;
      }
    }

    rafRef.current = requestAnimationFrame(animate);
    return () => {
      if (typeof cancelAnimationFrame === 'function') {
        cancelAnimationFrame(rafRef.current);
      }
    };
  }, [value, duration, prefersReducedMotion]);

  const formatted = (n: number) => {
    if (format === 'percent') return formatter.format(n / 100);
    return formatter.format(n);
  };

  return (
    <>
      <span className={className} aria-hidden="true">
        {formatted(display)}
      </span>
      <span role="status" aria-live="polite" aria-atomic="true" className="sr-only">
        {formatted(value)}
      </span>
    </>
  );
}
