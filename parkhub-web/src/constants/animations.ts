// Stagger containers — fade children in sequentially
export const stagger = { hidden: { opacity: 0 }, show: { opacity: 1, transition: { staggerChildren: 0.06 } } };
export const staggerSlow = { hidden: { opacity: 0 }, show: { opacity: 1, transition: { staggerChildren: 0.08 } } };

// Fade + slide up — default item animation for lists
export const fadeUp = { hidden: { opacity: 0, y: 20 }, show: { opacity: 1, y: 0 } };

// Spring physics — natural-feeling transitions (Apple HIG inspired)
export const spring = { type: 'spring' as const, stiffness: 380, damping: 30, mass: 0.8 };
export const springBouncy = { type: 'spring' as const, stiffness: 500, damping: 25, mass: 0.6 };
export const springGentle = { type: 'spring' as const, stiffness: 200, damping: 24, mass: 1 };

// Page transition variants — used with AnimatePresence in App.tsx
export const pageVariants = {
  initial: { opacity: 0, y: 8 },
  animate: { opacity: 1, y: 0 },
  exit: { opacity: 0, y: -4 },
};
export const pageTransition = spring;

// Skeleton fade-out — smooth exit when real content loads
export const skeletonExit = {
  exit: { opacity: 0, scale: 0.98 },
  transition: { duration: 0.2, ease: 'easeOut' as const },
};

// Scale feedback — subtle press effect for interactive cards
export const scaleTap = { whileTap: { scale: 0.97 }, transition: { type: 'spring' as const, stiffness: 400, damping: 17 } };

// Modal / dialog — scale + fade from trigger point
export const modalVariants = {
  initial: { opacity: 0, scale: 0.95 },
  animate: { opacity: 1, scale: 1 },
  exit: { opacity: 0, scale: 0.95 },
};
export const modalTransition = { type: 'spring' as const, stiffness: 400, damping: 30 };

// Slide sidebar — mobile nav with spring physics
export const slideLeft = {
  initial: { x: '-100%' },
  animate: { x: 0 },
  exit: { x: '-100%' },
  transition: { type: 'spring' as const, damping: 25, stiffness: 300 },
};
