/**
 * Keyboard shortcuts cheat-sheet modal — ported from the claude.ai/design
 * v3 handoff bundle (qol.jsx ShortcutsHelp). Triggered by Cmd+/ via
 * useKeyboardShortcuts; rendered at Layout level.
 *
 * React 19 patterns used:
 *  - `ref` as prop on the dialog container (no forwardRef)
 *  - `<dialog>` element for native backdrop + a11y
 */

import { useEffect, useRef } from 'react';
import { X } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';

interface ShortcutsHelpProps {
  open: boolean;
  onClose: () => void;
}

export function ShortcutsHelp({ open, onClose }: ShortcutsHelpProps) {
  const { t } = useTranslation();
  const dialogRef = useRef<HTMLDialogElement>(null);

  // Drive the native <dialog> element open/close from the prop.
  useEffect(() => {
    const d = dialogRef.current;
    if (!d) return;
    if (open && !d.open) d.showModal();
    if (!open && d.open) d.close();
  }, [open]);

  const groups: { section: string; items: [string, string][] }[] = [
    {
      section: t('shortcuts.general', 'General'),
      items: [
        ['⌘K', t('shortcuts.commandPalette', 'Command palette')],
        ['⌘/', t('shortcuts.thisPanel', 'This shortcuts panel')],
        ['⌘⇧D', t('shortcuts.darkMode', 'Toggle dark mode')],
        ['Esc', t('shortcuts.close', 'Close overlay')],
      ],
    },
    {
      section: t('shortcuts.navigate', 'Navigate'),
      items: [
        ['G D', t('shortcuts.goDashboard', 'Dashboard')],
        ['G B', t('shortcuts.goBook', 'Book a spot')],
        ['G P', t('shortcuts.goPass', 'Parking pass')],
        ['G L', t('shortcuts.goLots', 'Lot editor')],
      ],
    },
    {
      section: t('shortcuts.booking', 'Booking'),
      items: [
        ['N', t('shortcuts.newBooking', 'New booking')],
        ['S', t('shortcuts.swapRequest', 'Swap request')],
        ['⇧⏎', t('shortcuts.confirmPrint', 'Confirm & print')],
      ],
    },
  ];

  return (
    <dialog
      ref={dialogRef}
      onClose={onClose}
      onClick={(e) => {
        // Click on backdrop (outside dialog body) closes.
        if (e.target === e.currentTarget) onClose();
      }}
      className="card p-7 max-w-xl w-full backdrop:bg-black/50 backdrop:backdrop-blur-sm"
      aria-labelledby="shortcuts-help-title"
    >
      <div className="flex items-center justify-between mb-5">
        <h2
          id="shortcuts-help-title"
          className="text-lg font-bold text-surface-900 dark:text-white"
          style={{ letterSpacing: '-0.02em' }}
        >
          {t('shortcuts.title', 'Keyboard shortcuts')}
        </h2>
        <button
          type="button"
          onClick={onClose}
          className="btn btn-ghost btn-icon"
          aria-label={t('common.close', 'Close')}
        >
          <X weight="bold" className="w-4 h-4" />
        </button>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-3 gap-5">
        {groups.map((g) => (
          <div key={g.section}>
            <div
              className="text-[10px] font-bold text-surface-400 dark:text-surface-500 uppercase mb-2"
              style={{ letterSpacing: '0.06em' }}
            >
              {g.section}
            </div>
            <div className="flex flex-col gap-1">
              {g.items.map(([key, label]) => (
                <div key={key} className="flex items-center justify-between gap-2 text-xs">
                  <span className="text-surface-500 dark:text-surface-400">{label}</span>
                  <kbd className="inline-flex items-center px-1.5 py-0.5 text-[10px] font-semibold bg-surface-100 dark:bg-surface-800 border border-surface-200 dark:border-surface-700 rounded font-mono">
                    {key}
                  </kbd>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </dialog>
  );
}
