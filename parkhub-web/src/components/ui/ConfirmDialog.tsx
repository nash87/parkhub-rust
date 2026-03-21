import { motion, AnimatePresence } from 'framer-motion';
import { useTranslation } from 'react-i18next';
import { Warning } from '@phosphor-icons/react';
import { useEffect, useRef } from 'react';

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  variant?: 'danger' | 'default';
  onConfirm: () => void;
  onCancel: () => void;
}

/** Animated confirmation dialog replacing browser confirm(). Traps focus while open. */
export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel,
  cancelLabel,
  variant = 'default',
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const { t } = useTranslation();
  const confirmRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (open) {
      confirmRef.current?.focus();
      const handleEsc = (e: KeyboardEvent) => { if (e.key === 'Escape') onCancel(); };
      document.addEventListener('keydown', handleEsc);
      return () => document.removeEventListener('keydown', handleEsc);
    }
  }, [open, onCancel]);

  return (
    <AnimatePresence>
      {open && (
        <>
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="fixed inset-0 bg-black/40 z-[60]"
            onClick={onCancel}
            aria-hidden="true"
          />
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ type: 'spring', stiffness: 400, damping: 30 }}
            className="fixed inset-0 z-[61] flex items-center justify-center p-4"
            role="alertdialog"
            aria-modal="true"
            aria-labelledby="confirm-title"
            aria-describedby="confirm-message"
          >
            <div className="glass-modal p-6 max-w-sm w-full space-y-4" onClick={e => e.stopPropagation()}>
              <div className="flex items-start gap-3">
                {variant === 'danger' && (
                  <div className="w-10 h-10 rounded-full bg-danger/10 flex items-center justify-center flex-shrink-0">
                    <Warning weight="fill" className="w-5 h-5 text-danger" />
                  </div>
                )}
                <div>
                  <h3 id="confirm-title" className="text-base font-semibold text-surface-900 dark:text-white">
                    {title}
                  </h3>
                  <p id="confirm-message" className="text-sm text-surface-500 dark:text-surface-400 mt-1">
                    {message}
                  </p>
                </div>
              </div>
              <div className="flex justify-end gap-3">
                <button onClick={onCancel} className="btn btn-secondary btn-sm">
                  {cancelLabel || t('common.cancel')}
                </button>
                <button
                  ref={confirmRef}
                  onClick={onConfirm}
                  className={`btn btn-sm ${variant === 'danger' ? 'bg-danger text-white hover:bg-danger/90' : 'btn-primary'}`}
                >
                  {confirmLabel || t('common.delete')}
                </button>
              </div>
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
