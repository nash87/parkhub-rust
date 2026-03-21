import { Link, useLocation } from 'react-router-dom';
import { CaretRight } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';

/** Map route segments to i18n keys. */
const SEGMENT_LABELS: Record<string, string> = {
  '': 'nav.dashboard',
  admin: 'nav.admin',
  users: 'admin.users',
  lots: 'admin.lots',
  announcements: 'admin.announcements',
  settings: 'admin.settings',
  reports: 'admin.reports',
  bookings: 'nav.bookings',
  vehicles: 'nav.vehicles',
  absences: 'nav.absences',
  credits: 'nav.credits',
  team: 'nav.team',
  calendar: 'nav.calendar',
  notifications: 'nav.notifications',
  translations: 'nav.translations',
  profile: 'nav.profile',
  book: 'nav.book',
};

/** Breadcrumb navigation — auto-generated from the current route. */
export function Breadcrumb() {
  const { t } = useTranslation();
  const { pathname } = useLocation();

  const segments = pathname.split('/').filter(Boolean);
  if (segments.length === 0) return null; // Dashboard — no breadcrumb needed

  const crumbs = segments.map((seg, i) => ({
    label: t(SEGMENT_LABELS[seg] || seg),
    path: '/' + segments.slice(0, i + 1).join('/'),
    isLast: i === segments.length - 1,
  }));

  return (
    <nav aria-label={t('ui.breadcrumb')} className="flex items-center gap-1.5 text-sm mb-4">
      <Link
        to="/"
        className="text-surface-400 hover:text-primary-600 dark:hover:text-primary-400 transition-colors font-medium"
        aria-current={pathname === '/' ? 'page' : undefined}
      >
        {t('nav.dashboard')}
      </Link>
      {crumbs.map(crumb => (
        <span key={crumb.path} className="flex items-center gap-1.5">
          <CaretRight weight="bold" className="w-3 h-3 text-surface-300 dark:text-surface-600" aria-hidden="true" />
          {crumb.isLast ? (
            <span className="text-surface-900 dark:text-white font-medium" aria-current="page">
              {crumb.label}
            </span>
          ) : (
            <Link
              to={crumb.path}
              className="text-surface-400 hover:text-primary-600 dark:hover:text-primary-400 transition-colors font-medium"
            >
              {crumb.label}
            </Link>
          )}
        </span>
      ))}
    </nav>
  );
}
