import { useEffect, useId, useRef, useState } from 'react';
import { V5NamedIcon } from './index';

/**
 * Inline contextual help button. Click/focus reveals a small popover with
 * an explanatory paragraph. Uses the CSS Anchor Positioning API where
 * available (Chrome/Edge/Safari TP, Firefox 147+) and falls back to fixed
 * coordinates calculated from the trigger's bounding rect for older
 * browsers. ARIA-wired for screen readers.
 */
export function HelpTip({
  label,
  children,
  placement = 'top',
}: {
  /** Accessible name — what the tip *is*, not the hint content. */
  label: string;
  /** The hint text / rich content. */
  children: React.ReactNode;
  placement?: 'top' | 'bottom';
}) {
  const [open, setOpen] = useState(false);
  const [coords, setCoords] = useState<{ top: number; left: number } | null>(null);
  const btnRef = useRef<HTMLButtonElement>(null);
  const popRef = useRef<HTMLDivElement>(null);
  const id = useId();

  useEffect(() => {
    if (!open || !btnRef.current) return;
    // Compute fallback coordinates; anchor-positioning CSS below will
    // override these for supporting browsers.
    const rect = btnRef.current.getBoundingClientRect();
    const top = placement === 'top' ? rect.top - 8 : rect.bottom + 8;
    const left = rect.left + rect.width / 2;
    setCoords({ top, left });
  }, [open, placement]);

  useEffect(() => {
    if (!open) return;
    const h = (e: MouseEvent | KeyboardEvent) => {
      if (e instanceof KeyboardEvent && e.key === 'Escape') {
        setOpen(false);
        btnRef.current?.focus();
        return;
      }
      if (e instanceof MouseEvent) {
        const t = e.target as Node;
        if (!popRef.current?.contains(t) && !btnRef.current?.contains(t)) {
          setOpen(false);
        }
      }
    };
    document.addEventListener('mousedown', h);
    document.addEventListener('keydown', h);
    return () => {
      document.removeEventListener('mousedown', h);
      document.removeEventListener('keydown', h);
    };
  }, [open]);

  return (
    <>
      <button
        ref={btnRef}
        type="button"
        onClick={() => setOpen((o) => !o)}
        aria-expanded={open}
        aria-controls={id}
        aria-label={label}
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          width: 18,
          height: 18,
          borderRadius: '50%',
          background: 'var(--v5-sur2, rgba(0,0,0,0.04))',
          border: '1px solid var(--v5-bor, rgba(0,0,0,0.08))',
          color: 'var(--v5-mut, #888)',
          cursor: 'pointer',
          padding: 0,
          verticalAlign: 'middle',
          marginLeft: 6,
        }}
      >
        <V5NamedIcon name="info" size={11} color="currentColor" />
      </button>
      {open && coords && (
        <div
          ref={popRef}
          id={id}
          role="tooltip"
          style={{
            position: 'fixed',
            top: coords.top,
            left: coords.left,
            transform: placement === 'top' ? 'translate(-50%, -100%)' : 'translate(-50%, 0)',
            zIndex: 2500,
            maxWidth: 280,
            padding: '10px 12px',
            background: 'var(--v5-sur, #fff)',
            color: 'var(--v5-txt, #111)',
            border: '1px solid var(--v5-bor, rgba(0,0,0,0.12))',
            borderRadius: 10,
            boxShadow: '0 8px 28px rgba(0,0,0,0.18)',
            fontSize: 12,
            lineHeight: 1.55,
            animation: 'ph-v5-fadeUp 0.14s ease both',
          }}
        >
          {children}
        </div>
      )}
    </>
  );
}
