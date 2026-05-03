import {
  HouseIcon, AirplaneIcon, FirstAidKitIcon, BriefcaseIcon, NoteBlankIcon,
} from '@phosphor-icons/react';

export type AbsenceType = 'homeoffice' | 'vacation' | 'sick' | 'business_trip' | 'other';

export const ABSENCE_CONFIG: Record<AbsenceType, { icon: typeof HouseIcon; color: string; bg: string; dot: string }> = {
  homeoffice: { icon: HouseIcon, color: 'text-primary-600 dark:text-primary-400', bg: 'bg-primary-100 dark:bg-primary-900/30', dot: 'bg-primary-500' },
  vacation: { icon: AirplaneIcon, color: 'text-orange-600 dark:text-orange-400', bg: 'bg-orange-100 dark:bg-orange-900/30', dot: 'bg-orange-500' },
  sick: { icon: FirstAidKitIcon, color: 'text-red-600 dark:text-red-400', bg: 'bg-red-100 dark:bg-red-900/30', dot: 'bg-red-500' },
  business_trip: { icon: BriefcaseIcon, color: 'text-purple-600 dark:text-purple-400', bg: 'bg-purple-100 dark:bg-purple-900/30', dot: 'bg-purple-500' },
  other: { icon: NoteBlankIcon, color: 'text-surface-600 dark:text-surface-400', bg: 'bg-surface-100 dark:bg-surface-800/50', dot: 'bg-surface-500' },
};
