/**
 * Reusable skeleton loading components.
 *
 * All variants use the existing `.skeleton` CSS class (shimmer gradient)
 * defined in global.css, which is already dark-mode aware.
 *
 * Wrap with AnimatePresence and use `skeletonExit` from constants/animations.ts
 * for smooth fade-out when real content loads.
 */
import { motion, AnimatePresence } from 'framer-motion';

const exitProps = {
  exit: { opacity: 0, scale: 0.98 },
  transition: { duration: 0.2 },
};

/** Wrapper that animates skeleton exit when `loading` transitions from true to false. */
export function SkeletonContainer({ loading, skeleton, children }: {
  loading: boolean;
  skeleton: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <AnimatePresence mode="wait">
      {loading ? (
        <motion.div key="skeleton" {...exitProps}>
          {skeleton}
        </motion.div>
      ) : (
        <motion.div
          key="content"
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.25, ease: 'easeOut' }}
        >
          {children}
        </motion.div>
      )}
    </AnimatePresence>
  );
}

interface SkeletonTextProps {
  /** Tailwind width class, e.g. "w-48", "w-full". Default "w-full". */
  width?: string;
  /** Extra classes. */
  className?: string;
}

export function SkeletonText({ width = 'w-full', className = '' }: SkeletonTextProps) {
  return <div className={`h-4 skeleton rounded-lg ${width} ${className}`} />;
}

interface SkeletonAvatarProps {
  /** Size in Tailwind (w-10 h-10, etc). Default "w-10 h-10". */
  size?: string;
  className?: string;
}

export function SkeletonAvatar({ size = 'w-10 h-10', className = '' }: SkeletonAvatarProps) {
  return <div className={`skeleton rounded-full ${size} ${className}`} />;
}

interface SkeletonCardProps {
  /** Tailwind height class. Default "h-28". */
  height?: string;
  className?: string;
}

export function SkeletonCard({ height = 'h-28', className = '' }: SkeletonCardProps) {
  return <div className={`skeleton rounded-2xl ${height} ${className}`} />;
}

interface SkeletonTableProps {
  /** Number of skeleton rows. Default 3. */
  rows?: number;
  className?: string;
}

export function SkeletonTable({ rows = 3, className = '' }: SkeletonTableProps) {
  return (
    <div className={`space-y-3 ${className}`}>
      {/* Header row */}
      <div className="flex gap-4">
        <div className="h-4 skeleton rounded-lg w-1/4" />
        <div className="h-4 skeleton rounded-lg w-1/3" />
        <div className="h-4 skeleton rounded-lg w-1/5" />
        <div className="h-4 skeleton rounded-lg w-1/6" />
      </div>
      {/* Data rows */}
      {Array.from({ length: rows }, (_, i) => (
        <div key={i} className="flex gap-4 items-center">
          <div className="h-10 skeleton rounded-xl w-10 shrink-0" />
          <div className="flex-1 space-y-2">
            <div className="h-4 skeleton rounded-lg w-3/4" />
            <div className="h-3 skeleton rounded-lg w-1/2" />
          </div>
          <div className="h-6 skeleton rounded-full w-16 shrink-0" />
        </div>
      ))}
    </div>
  );
}

/* ── Composite skeletons for specific views ─────────────────────────── */

/** Dashboard page skeleton: greeting + 4 stat cards + booking list + quick actions */
export function DashboardSkeleton() {
  return (
    <div className="space-y-8">
      {/* Greeting */}
      <SkeletonText width="w-72" className="h-8" />

      {/* Stat cards */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
        {[1, 2, 3, 4].map(i => (
          <div key={i} className="skeleton rounded-2xl p-5 h-28">
            <div className="flex items-start justify-between">
              <div className="space-y-3">
                <div className="h-3 bg-surface-300/40 dark:bg-surface-600/40 rounded w-20" />
                <div className="h-7 bg-surface-300/40 dark:bg-surface-600/40 rounded w-12" />
              </div>
              <div className="w-10 h-10 bg-surface-300/40 dark:bg-surface-600/40 rounded-xl" />
            </div>
          </div>
        ))}
      </div>

      {/* Active bookings + Quick actions */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Booking list */}
        <div className="lg:col-span-2 bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-6">
          <div className="flex items-center justify-between mb-4">
            <div className="h-5 skeleton rounded-lg w-40" />
            <div className="h-4 skeleton rounded-lg w-20" />
          </div>
          <SkeletonTable rows={3} />
        </div>

        {/* Quick actions */}
        <div className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-6">
          <div className="h-5 skeleton rounded-lg w-32 mb-4" />
          <div className="space-y-3">
            {[1, 2, 3, 4].map(i => (
              <div key={i} className="flex items-center gap-3 p-3">
                <div className="w-10 h-10 skeleton rounded-xl shrink-0" />
                <div className="h-4 skeleton rounded-lg flex-1" />
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

/** Bookings page skeleton: header + filter card + 3 sections with cards */
export function BookingsSkeleton() {
  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="space-y-2">
          <SkeletonText width="w-48" className="h-7" />
          <SkeletonText width="w-64" className="h-4" />
        </div>
        <div className="h-10 skeleton rounded-xl w-28" />
      </div>

      {/* Filter bar */}
      <div className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-4">
        <div className="flex items-center gap-2 mb-3">
          <div className="w-4 h-4 skeleton rounded" />
          <div className="h-4 skeleton rounded-lg w-16" />
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
          <div className="h-10 skeleton rounded-xl" />
          <div className="h-10 skeleton rounded-xl" />
        </div>
      </div>

      {/* Section skeletons */}
      {[1, 2, 3].map(section => (
        <div key={section}>
          <div className="flex items-center gap-2 mb-4">
            <div className="w-5 h-5 skeleton rounded" />
            <div className="h-5 skeleton rounded-lg w-32" />
            <div className="h-5 skeleton rounded-full w-8" />
          </div>
          <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
            {[1, 2].map(card => (
              <div key={card} className="skeleton rounded-2xl border-l-4 border-l-surface-300 dark:border-l-surface-600 p-5 h-40">
                <div className="flex items-start justify-between mb-3">
                  <div className="flex items-center gap-3">
                    <div className="w-12 h-12 bg-surface-300/40 dark:bg-surface-600/40 rounded-xl" />
                    <div className="space-y-2">
                      <div className="h-4 bg-surface-300/40 dark:bg-surface-600/40 rounded w-28" />
                      <div className="h-3 bg-surface-300/40 dark:bg-surface-600/40 rounded w-20" />
                    </div>
                  </div>
                  <div className="h-5 bg-surface-300/40 dark:bg-surface-600/40 rounded-full w-14" />
                </div>
                <div className="flex gap-4 mb-3">
                  <div className="h-3 bg-surface-300/40 dark:bg-surface-600/40 rounded w-20" />
                  <div className="h-3 bg-surface-300/40 dark:bg-surface-600/40 rounded w-24" />
                </div>
                <div className="pt-3 border-t border-surface-100 dark:border-surface-800 flex justify-between">
                  <div className="h-3 bg-surface-300/40 dark:bg-surface-600/40 rounded w-32" />
                  <div className="h-7 bg-surface-300/40 dark:bg-surface-600/40 rounded-lg w-20" />
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

/** Vehicles page skeleton: header + 2 vehicle cards */
export function VehiclesSkeleton() {
  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="space-y-2">
          <SkeletonText width="w-48" className="h-7" />
          <SkeletonText width="w-40" className="h-4" />
        </div>
        <div className="h-10 skeleton rounded-xl w-32" />
      </div>

      {/* Vehicle cards */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {[1, 2].map(i => (
          <div key={i} className="bg-white dark:bg-surface-900 rounded-2xl border border-surface-200 dark:border-surface-800 p-6">
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-4">
                <div className="w-14 h-14 skeleton rounded-xl" />
                <div className="space-y-2">
                  <div className="h-5 skeleton rounded-lg w-32" />
                  <div className="h-3 skeleton rounded-lg w-24" />
                  <div className="flex items-center gap-1.5">
                    <div className="w-2.5 h-2.5 skeleton rounded-full" />
                    <div className="h-3 skeleton rounded-lg w-12" />
                  </div>
                </div>
              </div>
              <div className="w-9 h-9 skeleton rounded-lg" />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
