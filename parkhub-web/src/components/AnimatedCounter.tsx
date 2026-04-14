import { useEffect, useRef, useState } from 'react';

/**
 * Animated number counter with cubic ease-out.
 * Numbers animate up from previous value on change.
 */
export function AnimatedCounter({
  value,
  duration = 1000,
  className,
}: {
  value: number;
  duration?: number;
  className?: string;
}) {
  const [display, setDisplay] = useState(0);
  const ref = useRef<number>(0);
  const rafRef = useRef<number>(0);

  useEffect(() => {
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
  }, [value, duration]);

  return <span className={className}>{display}</span>;
}
