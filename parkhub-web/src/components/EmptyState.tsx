import { Link } from 'react-router-dom';
import { motion } from 'framer-motion';
import { useFeatures } from '../context/FeaturesContext';

/**
 * Rich empty state component with custom inline SVG illustrations.
 * When `rich_empty_states` is disabled, falls back to simple icon + text.
 */

type Variant = 'no-bookings' | 'no-vehicles' | 'no-transactions' | 'no-data';

interface Props {
  variant: Variant;
  icon?: React.ElementType;
  title: string;
  description?: string;
  actionLabel?: string;
  actionTo?: string;
}

export function EmptyState({ variant, icon: FallbackIcon, title, description, actionLabel, actionTo }: Props) {
  const { isEnabled } = useFeatures();
  const rich = isEnabled('rich_empty_states');

  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      className="flex flex-col items-center py-12 px-6 text-center"
    >
      {rich ? (
        <div className="w-40 h-40 mb-5">
          <Illustration variant={variant} />
        </div>
      ) : (
        FallbackIcon && <FallbackIcon weight="light" className="w-16 h-16 text-surface-200 dark:text-surface-700 mb-4" />
      )}

      <h3 className="text-base font-semibold text-surface-800 dark:text-surface-200 mb-1.5">
        {title}
      </h3>
      {description && (
        <p className="text-sm text-surface-500 dark:text-surface-400 max-w-xs mb-5 leading-relaxed">
          {description}
        </p>
      )}
      {actionLabel && actionTo && (
        <Link to={actionTo} className="btn btn-primary cursor-pointer">
          {actionLabel}
        </Link>
      )}
    </motion.div>
  );
}

function Illustration({ variant }: { variant: Variant }) {
  switch (variant) {
    case 'no-bookings':
      return (
        <svg viewBox="0 0 160 160" fill="none" xmlns="http://www.w3.org/2000/svg" className="w-full h-full">
          {/* Calendar with parking spot */}
          <rect x="30" y="35" width="100" height="90" rx="6" className="fill-surface-100 dark:fill-surface-800 stroke-surface-300 dark:stroke-surface-700" strokeWidth="1.5" />
          <rect x="30" y="35" width="100" height="22" rx="6" className="fill-accent-100 dark:fill-accent-900/30" />
          <rect x="30" y="51" width="100" height="6" className="fill-accent-100 dark:fill-accent-900/30" />
          {/* Calendar rings */}
          <rect x="52" y="28" width="4" height="16" rx="2" className="fill-accent-500" />
          <rect x="104" y="28" width="4" height="16" rx="2" className="fill-accent-500" />
          {/* Grid lines */}
          <line x1="30" y1="75" x2="130" y2="75" className="stroke-surface-200 dark:stroke-surface-700" strokeWidth="0.5" />
          <line x1="30" y1="95" x2="130" y2="95" className="stroke-surface-200 dark:stroke-surface-700" strokeWidth="0.5" />
          <line x1="63" y1="57" x2="63" y2="125" className="stroke-surface-200 dark:stroke-surface-700" strokeWidth="0.5" />
          <line x1="97" y1="57" x2="97" y2="125" className="stroke-surface-200 dark:stroke-surface-700" strokeWidth="0.5" />
          {/* Empty slot indicator */}
          <circle cx="80" cy="95" r="12" className="stroke-accent-400 dark:stroke-accent-600" strokeWidth="1.5" strokeDasharray="4 3" fill="none" />
          <text x="80" y="99" textAnchor="middle" className="fill-accent-500 text-[10px] font-bold font-[Outfit]">P</text>
        </svg>
      );

    case 'no-vehicles':
      return (
        <svg viewBox="0 0 160 160" fill="none" xmlns="http://www.w3.org/2000/svg" className="w-full h-full">
          {/* Car silhouette */}
          <path d="M35 100 L45 75 Q48 68 55 68 L105 68 Q112 68 115 75 L125 100" className="stroke-surface-300 dark:stroke-surface-600" strokeWidth="1.5" fill="none" />
          <rect x="30" y="100" width="100" height="25" rx="5" className="fill-surface-100 dark:fill-surface-800 stroke-surface-300 dark:stroke-surface-700" strokeWidth="1.5" />
          {/* Wheels */}
          <circle cx="55" cy="125" r="10" className="fill-surface-200 dark:fill-surface-700 stroke-surface-300 dark:stroke-surface-600" strokeWidth="1.5" />
          <circle cx="105" cy="125" r="10" className="fill-surface-200 dark:fill-surface-700 stroke-surface-300 dark:stroke-surface-600" strokeWidth="1.5" />
          <circle cx="55" cy="125" r="4" className="fill-surface-100 dark:fill-surface-800" />
          <circle cx="105" cy="125" r="4" className="fill-surface-100 dark:fill-surface-800" />
          {/* Windshield */}
          <path d="M50 75 L55 68 L105 68 L110 75 Z" className="fill-accent-100/50 dark:fill-accent-900/20" />
          {/* Dashed outline = missing */}
          <rect x="25" y="55" width="110" height="90" rx="8" className="stroke-accent-400 dark:stroke-accent-600" strokeWidth="1.5" strokeDasharray="6 4" fill="none" />
          {/* Plus icon */}
          <circle cx="130" cy="55" r="14" className="fill-accent-500" />
          <line x1="124" y1="55" x2="136" y2="55" stroke="white" strokeWidth="2" strokeLinecap="round" />
          <line x1="130" y1="49" x2="130" y2="61" stroke="white" strokeWidth="2" strokeLinecap="round" />
        </svg>
      );

    case 'no-transactions':
      return (
        <svg viewBox="0 0 160 160" fill="none" xmlns="http://www.w3.org/2000/svg" className="w-full h-full">
          {/* Coin stack */}
          <ellipse cx="65" cy="110" rx="25" ry="8" className="fill-surface-200 dark:fill-surface-700" />
          <ellipse cx="65" cy="100" rx="25" ry="8" className="fill-surface-100 dark:fill-surface-800 stroke-surface-300 dark:stroke-surface-700" strokeWidth="1" />
          <ellipse cx="65" cy="90" rx="25" ry="8" className="fill-accent-100 dark:fill-accent-900/30 stroke-accent-300 dark:stroke-accent-700" strokeWidth="1" />
          <ellipse cx="65" cy="80" rx="25" ry="8" className="fill-accent-200 dark:fill-accent-900/40 stroke-accent-400 dark:stroke-accent-600" strokeWidth="1" />
          {/* Arrow showing empty movement */}
          <path d="M105 60 L105 110" className="stroke-surface-300 dark:stroke-surface-600" strokeWidth="1.5" strokeDasharray="4 3" />
          <path d="M100 65 L105 55 L110 65" className="stroke-surface-300 dark:stroke-surface-600" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" fill="none" />
          <path d="M100 105 L105 115 L110 105" className="stroke-surface-300 dark:stroke-surface-600" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" fill="none" />
          {/* "0" on top coin */}
          <text x="65" y="84" textAnchor="middle" className="fill-accent-600 dark:fill-accent-400 text-[11px] font-bold font-[Outfit]">0</text>
        </svg>
      );

    case 'no-data':
    default:
      return (
        <svg viewBox="0 0 160 160" fill="none" xmlns="http://www.w3.org/2000/svg" className="w-full h-full">
          {/* Chart placeholder */}
          <rect x="30" y="40" width="100" height="80" rx="4" className="fill-surface-100 dark:fill-surface-800 stroke-surface-300 dark:stroke-surface-700" strokeWidth="1.5" />
          {/* Flat line chart */}
          <path d="M45 90 L60 85 L75 88 L90 82 L105 86 L115 84" className="stroke-surface-300 dark:stroke-surface-600" strokeWidth="1.5" strokeDasharray="4 3" fill="none" />
          {/* Y-axis ticks */}
          {[55, 65, 75, 85, 95].map(y => (
            <line key={y} x1="38" y1={y} x2="42" y2={y} className="stroke-surface-300 dark:stroke-surface-600" strokeWidth="0.5" />
          ))}
          {/* Magnifying glass */}
          <circle cx="115" cy="45" r="16" className="fill-white dark:fill-surface-900 stroke-accent-400 dark:stroke-accent-600" strokeWidth="1.5" />
          <line x1="126" y1="56" x2="135" y2="65" className="stroke-accent-400 dark:stroke-accent-600" strokeWidth="2.5" strokeLinecap="round" />
          <text x="115" y="49" textAnchor="middle" className="fill-accent-500 text-[10px] font-bold">?</text>
        </svg>
      );
  }
}
