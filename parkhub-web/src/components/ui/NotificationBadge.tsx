import { motion, AnimatePresence } from 'framer-motion';

interface NotificationBadgeProps {
  count: number;
  /** Maximum displayed count — shows "9+" if exceeded */
  max?: number;
}

/** Animated notification badge for nav items. Spring-based pop-in/out. */
export function NotificationBadge({ count, max = 9 }: NotificationBadgeProps) {
  const display = count > max ? `${max}+` : String(count);

  return (
    <AnimatePresence>
      {count > 0 && (
        <motion.span
          initial={{ scale: 0, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          exit={{ scale: 0, opacity: 0 }}
          transition={{ type: 'spring', stiffness: 500, damping: 25 }}
          className="absolute -top-1 -right-1 min-w-[18px] h-[18px] flex items-center justify-center rounded-full bg-danger text-white text-[10px] font-bold leading-none px-1"
          aria-label={`${count} unread`}
        >
          {display}
        </motion.span>
      )}
    </AnimatePresence>
  );
}
